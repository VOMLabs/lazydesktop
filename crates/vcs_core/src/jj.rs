//! `jj` (Jujutsu) subprocess runner and high-level read commands.
//!
//! Locates the `jj` binary, assembles the global flags, spawns the process in
//! the repository directory with a bounded timeout, and captures its output.
//! High-level commands build on [`run_jj`]: [`jj_log`] lists commits as typed
//! records and [`jj_status`] reports working-copy changes. Mutation commands
//! edit descriptions ([`jj_describe`]), undo/redo operations ([`jj_undo`],
//! [`jj_redo`]), and restore or revert operation-log entries ([`jj_op_restore`],
//! [`jj_op_revert`]) listed by [`jj_op_log`]. More commands (`new`, ...) will
//! be added by later tasks.
//!
//! The runner always passes `--no-pager --color never` so command output is
//! stable plain text (no interactive pager, no ANSI escapes) regardless of
//! the user's jj config.

use std::path::Path;
use std::process::Output;
use std::time::Duration;

use tokio::process::Command;

use crate::error::{VcsError, VcsResult};
use crate::jjparse::{self, JjBookmark, JjCommit, JjFileStatus, JjOpEntry, JjTag};
use crate::runtime::runtime;
use crate::state::{repo_state_cache, RepoState};

/// Number of output lines above which parsing is offloaded to a rayon thread
/// pool inside `spawn_blocking` instead of doing the serial parse on the async
/// worker thread.
const PARALLEL_PARSE_THRESHOLD: usize = 512;

/// Default timeout for jj subprocess calls (read-only and mutating commands).
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Environment variable that overrides the `jj` binary path (e.g. a bundled
/// static binary or a toolchain-managed install).
const JJ_BIN_ENV: &str = "JJ_BIN";

/// Path of the `jj` executable: `$JJ_BIN` when set, else `jj` from `PATH`.
fn jj_binary() -> String {
    std::env::var(JJ_BIN_ENV).unwrap_or_else(|_| "jj".to_string())
}

/// `repo_path` must exist and be a directory before we run anything in it.
fn validate_repo_path(repo_path: &Path) -> VcsResult<()> {
    if !repo_path.is_dir() {
        return Err(VcsError::Invalid(format!(
            "not a directory: {}",
            repo_path.display()
        )));
    }
    Ok(())
}

/// Run `jj <args>` in `repo_path` with the default [`DEFAULT_TIMEOUT`].
pub async fn run_jj(repo_path: &Path, args: &[&str]) -> VcsResult<Output> {
    run_jj_with_timeout(repo_path, args, DEFAULT_TIMEOUT).await
}

/// Run `jj <args>` in `repo_path`, bounding execution to `timeout`.
///
/// Always passes the global flags `--no-pager --color never`. A non-zero exit
/// status is an error carrying the exit code and stderr; a timeout is a
/// [`VcsError::Timeout`]. On success the raw [`Output`] (stdout, stderr, exit
/// status) is returned so callers can parse it.
pub async fn run_jj_with_timeout(
    repo_path: &Path,
    args: &[&str],
    timeout: Duration,
) -> VcsResult<Output> {
    validate_repo_path(repo_path)?;

    let binary = jj_binary();
    let mut command = Command::new(&binary);
    command
        .current_dir(repo_path)
        .arg("--no-pager")
        .arg("--color")
        .arg("never")
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);

    let child = command.spawn().map_err(|err| {
        VcsError::Other(format!("failed to spawn {binary}: {err}"))
    })?;

    let output = match tokio::time::timeout(timeout, child.wait_with_output()).await {
        Ok(result) => result.map_err(|err| {
            VcsError::Other(format!("failed to read {binary} output: {err}"))
        })?,
        Err(_elapsed) => {
            // `child` is dropped here; `kill_on_drop(true)` terminates it so
            // a timed-out jj never keeps running in the background.
            return Err(VcsError::Timeout(format!(
                "{binary} did not finish within {} seconds",
                timeout.as_secs()
            )));
        }
    };

    if output.status.success() {
        return Ok(output);
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stderr = if stderr.is_empty() {
        "(no stderr output)".to_string()
    } else {
        stderr
    };
    Err(VcsError::Subprocess {
        command: binary,
        status: describe_status(output.status),
        stderr,
    })
}

/// Synchronous wrapper for callers that cannot await (e.g. the FFI layer).
/// Drives [`run_jj_with_timeout`] on the shared [`runtime`].
pub fn run_jj_blocking(
    repo_path: &Path,
    args: &[&str],
    timeout: Duration,
) -> VcsResult<Output> {
    runtime().block_on(run_jj_with_timeout(repo_path, args, timeout))
}

/// Human-readable description of a failed exit status.
fn describe_status(status: std::process::ExitStatus) -> String {
    match status.code() {
        Some(code) => format!("exit code {code}"),
        // Terminated by a signal (Unix): `code()` is None.
        None => "terminated by signal".to_string(),
    }
}

/// List commits as typed records via `jj log --no-graph --ignore-working-copy -T <log_template()>`.
///
/// `revset` selects the revisions to show; an empty string uses jj's default
/// revset (`@ | ancestors(@, ..) | root()`). `limit` caps the number of
/// commits with jj's `-n`/`--limit` option. The command is read-only, so it
/// runs with [`DEFAULT_TIMEOUT`]; stdout is parsed with
/// [`jjparse::parse_commits_jsonl`], offloaded to rayon for large outputs.
///
/// On success the working-copy commit id (the first entry, `@`) is recorded in
/// the process-wide [`repo_state_cache`]; see [`jj_cached_working_copy_id`].
pub async fn jj_log(
    repo_path: &Path,
    revset: &str,
    limit: Option<usize>,
) -> VcsResult<Vec<JjCommit>> {
    let mut args: Vec<String> = vec![
        "log".to_string(),
        "--no-graph".to_string(),
        "--ignore-working-copy".to_string(),
        "-T".to_string(),
        jjparse::log_template().to_string(),
    ];
    if !revset.is_empty() {
        args.push("-r".to_string());
        args.push(revset.to_string());
    }
    if let Some(limit) = limit {
        args.push("-n".to_string());
        args.push(limit.to_string());
    }
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = run_jj(repo_path, &arg_refs).await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let commits = if stdout.lines().count() >= PARALLEL_PARSE_THRESHOLD {
        parse_commits_parallel(stdout).await
    } else {
        jjparse::parse_commits_jsonl(&stdout)
    };
    if let Some(working_copy) = commits.first() {
        repo_state_cache().update(repo_path, |state| {
            state.working_copy_commit_id = Some(working_copy.commit_id.clone());
        });
    }
    Ok(commits)
}

/// Read the cached working-copy commit id for `repo_path`, if known.
///
/// Populated by the last [`jj_log`] call; never spawns a subprocess and never
/// blocks (the dashmap guard is released before returning).
pub fn jj_cached_working_copy_id(repo_path: &Path) -> Option<String> {
    repo_state_cache()
        .get(repo_path)
        .working_copy_commit_id
}

/// Parse a JSON-lines `jj log` output on the rayon thread pool.
///
/// The parse runs inside [`tokio::task::spawn_blocking`] and uses rayon's
/// `par_iter` over the lines so a large history does not stall an async worker
/// thread. Falls back to the same [`jjparse::parse_commits_jsonl`] grammar;
/// malformed lines are skipped, exactly like the serial parser.
async fn parse_commits_parallel(stdout: std::borrow::Cow<'_, str>) -> Vec<JjCommit> {
    let output = stdout.into_owned();
    tokio::task::spawn_blocking(move || {
        use rayon::prelude::*;
        output
            .par_lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect()
    })
    .await
    .unwrap_or_default()
}

/// Spawn a thread that runs blocking `work`, streaming results through a
/// bounded [`flume`] channel that can be awaited from the async runtime.
///
/// Use this when a result is produced by blocking code (a subprocess, a CPU
/// bound computation) and the consumer wants to process items as they arrive
/// instead of buffering the whole output first. Dropping the returned
/// receiver ends the stream; the worker observes the disconnect via
/// [`flume::Sender::send`] and can stop early.
pub fn stream_blocking<T: Send + 'static>(
    capacity: usize,
    work: impl FnOnce(flume::Sender<T>) + Send + 'static,
) -> flume::Receiver<T> {
    let (sender, receiver) = flume::bounded(capacity);
    std::thread::spawn(move || work(sender));
    receiver
}

/// Report working-copy changes via `jj status`.
///
/// Plain `jj status` is used (not `--ignore-working-copy`): jj only computes
/// the working-copy diff against the last *snapshot*, so skipping the snapshot
/// would miss changes made on disk since the previous jj command. The snapshot
/// creates a working-copy commit, matching the semantics of the jj CLI that
/// the UI exposes. Parsed with [`jjparse::parse_status_output`]; an empty vec
/// is returned when there are no changes.
pub async fn jj_status(repo_path: &Path) -> VcsResult<Vec<JjFileStatus>> {
    let args = ["status"];
    let output = run_jj(repo_path, &args).await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(jjparse::parse_status_output(&stdout))
}

/// Set the description of the commit selected by `revset`.
///
/// Uses `-m` so no editor is opened. jj prints nothing on success; a non-zero
/// exit (e.g. for an immutable commit) surfaces as a [`VcsError::Subprocess`].
pub async fn jj_describe(repo_path: &Path, revset: &str, message: &str) -> VcsResult<()> {
    let args = ["describe", "-r", revset, "-m", message];
    run_jj(repo_path, &args).await?;
    Ok(())
}

/// Undo the most recent operation (`jj undo`; `jj op undo` was removed in 0.39).
pub async fn jj_undo(repo_path: &Path) -> VcsResult<()> {
    run_jj(repo_path, &["undo"]).await?;
    Ok(())
}

/// Redo the most recently undone operation (`jj redo`).
pub async fn jj_redo(repo_path: &Path) -> VcsResult<()> {
    run_jj(repo_path, &["redo"]).await?;
    Ok(())
}

/// List operation-log entries as typed records via
/// `jj op log --no-graph -T <op_log_template()>`.
///
/// `limit` caps the number of entries with jj's `-n`/`--limit` option. Parsed
/// with [`jjparse::parse_ops_jsonl`].
pub async fn jj_op_log(
    repo_path: &Path,
    limit: Option<usize>,
) -> VcsResult<Vec<JjOpEntry>> {
    let mut args: Vec<String> = vec![
        "op".to_string(),
        "log".to_string(),
        "--no-graph".to_string(),
        "-T".to_string(),
        jjparse::op_log_template().to_string(),
    ];
    if let Some(limit) = limit {
        args.push("-n".to_string());
        args.push(limit.to_string());
    }
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    let output = run_jj(repo_path, &arg_refs).await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(jjparse::parse_ops_jsonl(&stdout))
}

/// Restore the local repo state to `op` (`jj op restore --what repo <op>`).
///
/// `--what repo` is an experimental jj 0.44 flag that scopes the restore to
/// the repo state and local bookmarks.
pub async fn jj_op_restore(repo_path: &Path, op: &str) -> VcsResult<()> {
    run_jj(repo_path, &["op", "restore", "--what", "repo", op]).await?;
    Ok(())
}

/// Revert the working copy to `op` (`jj op revert --what repo <op>`).
///
/// `--what repo` is an experimental jj 0.44 flag that scopes the revert to
/// the repo state and local bookmarks.
pub async fn jj_op_revert(repo_path: &Path, op: &str) -> VcsResult<()> {
    run_jj(repo_path, &["op", "revert", "--what", "repo", op]).await?;
    Ok(())
}

/// List bookmarks as typed records via `jj bookmark list -T <bookmark_template()>`.
///
/// Remote bookmarks carry the `name@remote` suffix. Parsed with
/// [`jjparse::parse_bookmarks_jsonl`].
pub async fn jj_bookmark_list(repo_path: &Path) -> VcsResult<Vec<JjBookmark>> {
    let args = ["bookmark", "list", "-T", jjparse::bookmark_template()];
    let output = run_jj(repo_path, &args).await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(jjparse::parse_bookmarks_jsonl(&stdout))
}

/// List tags as typed records via `jj tag list -T <tag_template()>`.
///
/// Parsed with [`jjparse::parse_tags_jsonl`].
pub async fn jj_tag_list(repo_path: &Path) -> VcsResult<Vec<JjTag>> {
    let args = ["tag", "list", "-T", jjparse::tag_template()];
    let output = run_jj(repo_path, &args).await?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(jjparse::parse_tags_jsonl(&stdout))
}

/// Create or update the tag `name` to point at the commit selected by
/// `revset` (`jj tag set -r <revset> <name>`; default revset is `@`).
pub async fn jj_tag_set(repo_path: &Path, name: &str, revset: &str) -> VcsResult<()> {
    let args = ["tag", "set", "-r", revset, name];
    run_jj(repo_path, &args).await?;
    Ok(())
}

/// Delete the tag `name` (`jj tag delete <name>`).
pub async fn jj_tag_delete(repo_path: &Path, name: &str) -> VcsResult<()> {
    run_jj(repo_path, &["tag", "delete", name]).await?;
    Ok(())
}

/// Push the given `bookmarks` and `tags` to `remote`
/// (`jj git push --remote <remote> -b ... -t ...`).
///
/// `bookmarks` and `tags` are passed through `-b`/`-t` flags (each repeatable).
/// jj runs its own safety checks before moving remote refs; failures surface
/// as [`VcsError::Subprocess`] with jj's stderr.
pub async fn jj_git_push(
    repo_path: &Path,
    remote: &str,
    bookmarks: &[&str],
    tags: &[&str],
) -> VcsResult<()> {
    let mut args: Vec<String> = vec!["git".to_string(), "push".to_string()];
    args.push("--remote".to_string());
    args.push(remote.to_string());
    for bookmark in bookmarks {
        args.push("-b".to_string());
        args.push((*bookmark).to_string());
    }
    for tag in tags {
        args.push("-t".to_string());
        args.push((*tag).to_string());
    }
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    run_jj(repo_path, &arg_refs).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::*;

    /// Whether a real `jj` binary is available for integration tests. Uses the
    /// same resolution as production code (`JJ_BIN` override, else PATH).
    fn jj_available() -> bool {
        std::process::Command::new(jj_binary())
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    /// Whether the `git` CLI is available (needed by the push test to create a
    /// bare remote repository).
    fn git_available() -> bool {
        std::process::Command::new("git")
            .arg("--version")
            .output()
            .is_ok_and(|output| output.status.success())
    }

    /// Create a throwaway jj repository at `temp/repo`; returns both paths so
    /// the `TempDir` stays alive for the whole test.
    fn init_repo() -> (TempDir, PathBuf) {
        let temp = TempDir::new().expect("create temp dir");
        let repo = temp.path().join("repo");
        run_jj_blocking(temp.path(), &["git", "init", "repo"], DEFAULT_TIMEOUT)
            .expect("jj git init must succeed");
        (temp, repo)
    }

    /// Run `jj <args>` in `repo` and return stdout as a lossy string.
    fn jj_stdout(repo: &Path, args: &[&str]) -> String {
        let output =
            run_jj_blocking(repo, args, DEFAULT_TIMEOUT).expect("jj command must succeed");
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    #[test]
    fn run_jj_rejects_missing_repo_path() {
        let dir = TempDir::new().expect("create temp dir");
        let missing = dir.path().join("does-not-exist");
        let err =
            run_jj_blocking(&missing, &["--version"], DEFAULT_TIMEOUT).expect_err("must fail");
        assert!(matches!(err, VcsError::Invalid(_)), "got: {err}");
        assert!(err.to_string().contains("not a directory"), "got: {err}");
    }

    #[test]
    fn run_jj_rejects_file_repo_path() {
        let dir = TempDir::new().expect("create temp dir");
        let file_path = dir.path().join("file");
        std::fs::write(&file_path, "not a repo").expect("write file");
        let err =
            run_jj_blocking(&file_path, &["--version"], DEFAULT_TIMEOUT).expect_err("must fail");
        assert!(matches!(err, VcsError::Invalid(_)), "got: {err}");
    }

    #[test]
    fn run_jj_blocking_prints_version_when_jj_installed() {
        if !jj_available() {
            eprintln!("skipping: jj not installed");
            return;
        }
        let dir = TempDir::new().expect("create temp dir");
        let output = run_jj_blocking(dir.path(), &["--version"], DEFAULT_TIMEOUT)
            .expect("jj --version must succeed");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("jj"), "unexpected output: {stdout}");
    }

    #[test]
    fn run_jj_async_works_on_shared_runtime() {
        if !jj_available() {
            eprintln!("skipping: jj not installed");
            return;
        }
        let dir = TempDir::new().expect("create temp dir");
        let output = runtime()
            .block_on(run_jj(dir.path(), &["--version"]))
            .expect("jj --version must succeed");
        assert!(output.status.success());
    }

    #[test]
    fn run_jj_failure_includes_status_and_stderr() {
        if !jj_available() {
            eprintln!("skipping: jj not installed");
            return;
        }
        let dir = TempDir::new().expect("create temp dir");
        let err = run_jj_blocking(
            dir.path(),
            &["this-subcommand-does-not-exist"],
            DEFAULT_TIMEOUT,
        )
        .expect_err("unknown subcommand must fail");
        match err {
            VcsError::Subprocess {
                command,
                status,
                stderr,
            } => {
                assert_eq!(command, jj_binary());
                assert!(status.contains("exit code"), "unexpected status: {status}");
                assert!(!stderr.is_empty(), "stderr should be captured");
            }
            other => panic!("expected Subprocess error, got: {other:?}"),
        }
    }

    #[test]
    fn jj_log_lists_commits_with_parsed_fields() {
        if !jj_available() {
            eprintln!("skipping: jj not installed");
            return;
        }
        let (_temp, repo) = init_repo();
        jj_stdout(&repo, &["describe", "-m", "first"]);
        jj_stdout(&repo, &["new", "-m", "second"]);

        let commits = runtime()
            .block_on(jj_log(&repo, "", None))
            .expect("jj log must succeed");
        assert_eq!(
            commits.len(),
            3,
            "expected root + first + second, got {commits:?}"
        );
        let second = commits
            .iter()
            .find(|c| c.description == "second")
            .expect("second commit");
        assert_eq!(second.description, "second");
        assert_eq!(second.parents.len(), 1);
        assert!(second.is_empty, "jj new -m creates an empty commit");
    }

    #[test]
    fn jj_log_respects_revset_and_limit() {
        if !jj_available() {
            eprintln!("skipping: jj not installed");
            return;
        }
        let (_temp, repo) = init_repo();
        jj_stdout(&repo, &["describe", "-m", "first"]);
        jj_stdout(&repo, &["new", "-m", "second"]);

        let at_head = runtime()
            .block_on(jj_log(&repo, "@", None))
            .expect("jj log -r @ must succeed");
        assert_eq!(at_head.len(), 1, "got {at_head:?}");
        assert_eq!(at_head[0].description, "second");

        let limited = runtime()
            .block_on(jj_log(&repo, "", Some(1)))
            .expect("jj log -n 1 must succeed");
        assert_eq!(limited.len(), 1, "got {limited:?}");
    }

    #[test]
    fn jj_status_reports_working_copy_changes() {
        if !jj_available() {
            eprintln!("skipping: jj not installed");
            return;
        }
        let (_temp, repo) = init_repo();
        std::fs::write(repo.join("a.txt"), "a\n").expect("write a.txt");
        jj_stdout(&repo, &["describe", "-m", "init"]);
        jj_stdout(&repo, &["new", "-m", "next"]);
        std::fs::remove_file(repo.join("a.txt")).expect("remove a.txt");
        std::fs::write(repo.join("c.txt"), "c\n").expect("write c.txt");

        let files = runtime()
            .block_on(jj_status(&repo))
            .expect("jj status must succeed");
        let mut rows: Vec<_> = files
            .iter()
            .map(|f| (f.status.as_str(), f.path.as_str()))
            .collect();
        rows.sort();
        assert_eq!(rows, vec![("A", "c.txt"), ("D", "a.txt")], "got {rows:?}");
    }

    #[test]
    fn jj_status_empty_when_clean() {
        if !jj_available() {
            eprintln!("skipping: jj not installed");
            return;
        }
        let (_temp, repo) = init_repo();
        let files = runtime()
            .block_on(jj_status(&repo))
            .expect("jj status must succeed");
        assert!(files.is_empty(), "expected clean status, got {files:?}");
    }

    #[test]
    fn jj_describe_sets_message() {
        if !jj_available() {
            eprintln!("skipping: jj not installed");
            return;
        }
        let (_temp, repo) = init_repo();
        runtime()
            .block_on(jj_describe(&repo, "@", "hello world"))
            .expect("jj describe must succeed");
        let stdout = jj_stdout(
            &repo,
            &["log", "--no-graph", "-r", "@", "-T", "description.first_line()"],
        );
        assert!(stdout.contains("hello world"), "got: {stdout}");
    }

    #[test]
    fn jj_op_log_round_trip() {
        if !jj_available() {
            eprintln!("skipping: jj not installed");
            return;
        }
        let (_temp, repo) = init_repo();
        jj_stdout(&repo, &["describe", "-m", "first"]);
        jj_stdout(&repo, &["new", "-m", "second"]);

        let ops = runtime()
            .block_on(jj_op_log(&repo, None))
            .expect("jj op log must succeed");
        assert!(!ops.is_empty(), "expected at least one operation");
        for op in &ops {
            assert!(!op.op_id.is_empty(), "unexpected op: {op:?}");
            assert!(!op.timestamp.is_empty(), "unexpected op: {op:?}");
        }

        let limited = runtime()
            .block_on(jj_op_log(&repo, Some(1)))
            .expect("jj op log -n 1 must succeed");
        assert_eq!(limited.len(), 1, "got {limited:?}");
    }

    #[test]
    fn jj_undo_redo_round_trip() {
        if !jj_available() {
            eprintln!("skipping: jj not installed");
            return;
        }
        let (_temp, repo) = init_repo();
        runtime()
            .block_on(jj_describe(&repo, "@", "undone-msg"))
            .expect("jj describe must succeed");

        runtime()
            .block_on(jj_undo(&repo))
            .expect("jj undo must succeed");
        let after_undo = jj_stdout(
            &repo,
            &["log", "--no-graph", "-r", "@", "-T", "description.first_line()"],
        );
        assert!(
            !after_undo.contains("undone-msg"),
            "description should be undone, got: {after_undo}"
        );

        runtime()
            .block_on(jj_redo(&repo))
            .expect("jj redo must succeed");
        let after_redo = jj_stdout(
            &repo,
            &["log", "--no-graph", "-r", "@", "-T", "description.first_line()"],
        );
        assert!(
            after_redo.contains("undone-msg"),
            "description should be restored, got: {after_redo}"
        );
    }

    #[test]
    fn jj_op_restore_round_trip() {
        if !jj_available() {
            eprintln!("skipping: jj not installed");
            return;
        }
        let (_temp, repo) = init_repo();
        runtime()
            .block_on(jj_describe(&repo, "@", "original"))
            .expect("jj describe must succeed");

        // Latest operation is the describe; its parent op's state had the
        // empty description, so restoring to the describe op is a no-op for
        // the description. Instead: describe again, then restore to the op id
        // captured *before* the second describe.
        let ops = runtime()
            .block_on(jj_op_log(&repo, Some(1)))
            .expect("jj op log must succeed");
        let before_second_describe = ops[0].op_id.clone();
        runtime()
            .block_on(jj_describe(&repo, "@", "changed"))
            .expect("jj describe must succeed");
        assert!(jj_stdout(
            &repo,
            &["log", "--no-graph", "-r", "@", "-T", "description.first_line()"]
        )
        .contains("changed"));

        runtime()
            .block_on(jj_op_restore(&repo, &before_second_describe))
            .expect("jj op restore must succeed");
        let restored = jj_stdout(
            &repo,
            &["log", "--no-graph", "-r", "@", "-T", "description.first_line()"],
        );
        assert!(restored.contains("original"), "got: {restored}");
    }

    #[test]
    fn jj_op_revert_round_trip() {
        if !jj_available() {
            eprintln!("skipping: jj not installed");
            return;
        }
        let (_temp, repo) = init_repo();
        jj_stdout(&repo, &["describe", "-m", "first"]);
        jj_stdout(&repo, &["new", "-m", "second"]);

        // The latest operation is "new empty commit"; reverting it applies the
        // inverse and removes the "second" commit, returning @ to "first".
        let ops = runtime()
            .block_on(jj_op_log(&repo, Some(1)))
            .expect("jj op log must succeed");
        let new_op = ops[0].op_id.clone();
        runtime()
            .block_on(jj_op_revert(&repo, &new_op))
            .expect("jj op revert must succeed");

        let stdout = jj_stdout(
            &repo,
            &["log", "--no-graph", "-T", "description.first_line()"],
        );
        assert!(stdout.contains("first"), "got: {stdout}");
        assert!(
            !stdout.contains("second"),
            "reverted commit must be gone, got: {stdout}"
        );
    }

    #[test]
    fn jj_bookmark_list_round_trip() {
        if !jj_available() {
            eprintln!("skipping: jj not installed");
            return;
        }
        let (_temp, repo) = init_repo();
        jj_stdout(&repo, &["bookmark", "create", "-r", "@", "mybookmark"]);

        let bookmarks = runtime()
            .block_on(jj_bookmark_list(&repo))
            .expect("jj bookmark list must succeed");
        assert_eq!(bookmarks.len(), 1, "got {bookmarks:?}");
        assert_eq!(bookmarks[0].name, "mybookmark");
        assert!(!bookmarks[0].target.is_empty(), "got {bookmarks:?}");
    }

    #[test]
    fn jj_tag_set_list_delete_round_trip() {
        if !jj_available() {
            eprintln!("skipping: jj not installed");
            return;
        }
        let (_temp, repo) = init_repo();
        runtime()
            .block_on(jj_tag_set(&repo, "v1.0", "@"))
            .expect("jj tag set must succeed");

        let tags = runtime()
            .block_on(jj_tag_list(&repo))
            .expect("jj tag list must succeed");
        assert_eq!(tags.len(), 1, "got {tags:?}");
        assert_eq!(tags[0].name, "v1.0");
        assert!(!tags[0].target.is_empty(), "got {tags:?}");

        runtime()
            .block_on(jj_tag_delete(&repo, "v1.0"))
            .expect("jj tag delete must succeed");
        let tags = runtime()
            .block_on(jj_tag_list(&repo))
            .expect("jj tag list must succeed");
        assert!(tags.is_empty(), "expected no tags, got {tags:?}");
    }

    #[test]
    fn jj_git_push_to_local_remote() {
        if !jj_available() || !git_available() {
            eprintln!("skipping: jj or git not installed");
            return;
        }
        let (temp, repo) = init_repo();
        // Configure identity so pushed commits are valid.
        runtime()
            .block_on(run_jj(
                &repo,
                &["config", "set", "--user", "user.name", "Test"],
            ))
            .expect("set user.name");
        runtime()
            .block_on(run_jj(
                &repo,
                &["config", "set", "--user", "user.email", "test@example.com"],
            ))
            .expect("set user.email");
        jj_stdout(&repo, &["describe", "-m", "initial commit"]);
        jj_stdout(&repo, &["bookmark", "create", "-r", "@", "main"]);

        // Bare git repo acts as the origin remote.
        let remote = temp.path().join("remote.git");
        let git_ok = std::process::Command::new("git")
            .args(["init", "--bare"])
            .arg(&remote)
            .status()
            .expect("spawn git")
            .success();
        assert!(git_ok, "git init --bare must succeed");
        jj_stdout(
            &repo,
            &["git", "remote", "add", "origin", remote.to_str().unwrap()],
        );

        runtime()
            .block_on(jj_git_push(&repo, "origin", &["main"], &[]))
            .expect("jj git push must succeed");

        let show_ref = std::process::Command::new("git")
            .args(["--git-dir"])
            .arg(&remote)
            .arg("show-ref")
            .output()
            .expect("spawn git");
        let stdout = String::from_utf8_lossy(&show_ref.stdout);
        assert!(stdout.contains("refs/heads/main"), "got: {stdout}");
    }

    #[test]
    fn jj_log_caches_working_copy_id() {
        if !jj_available() {
            eprintln!("skipping: jj not installed");
            return;
        }
        let (temp, repo) = init_repo();
        // Fresh repo: cache must be empty for an unknown path.
        assert!(jj_cached_working_copy_id(&repo).is_none());

        jj_stdout(&repo, &["describe", "-m", "cached"]);
        let commits = runtime()
            .block_on(jj_log(&repo, "", None))
            .expect("jj log must succeed");
        assert!(!commits.is_empty());
        assert_eq!(
            jj_cached_working_copy_id(&repo).as_deref(),
            Some(commits[0].commit_id.as_str()),
            "cache must record the working-copy commit id"
        );

        // Other repos stay untouched.
        let other = temp.path().join("other");
        assert!(jj_cached_working_copy_id(&other).is_none());
    }

    #[test]
    fn parse_commits_parallel_matches_serial() {
        // Build > PARALLEL_PARSE_THRESHOLD JSON lines so the parallel path is
        // exercised for real.
        let line = r#"{"commit_id":"0123456789abcdef0123456789abcdef01234567","commit_id_short":"0123456789ab","change_id":"qqqqqqqqqqqqqqqqqqqqqqqqqqqqqqqq","change_id_short":"qqqqqqqq","description":"bulk","parents":[],"bookmarks":[],"tags":[],"is_conflict":false,"is_empty":false,"author_email":"a@example.com","timestamp":"2001-02-03T04:05:09+07:00"}"#;
        let output = std::iter::repeat_with(|| line).take(600).collect::<Vec<_>>().join("\n");
        let stdout = std::borrow::Cow::Owned(output.clone());

        let parallel = runtime()
            .block_on(parse_commits_parallel(stdout));
        let serial = jjparse::parse_commits_jsonl(&output);
        assert_eq!(parallel.len(), 600, "all lines must parse");
        assert_eq!(parallel, serial, "parallel and serial parses must agree");
    }

    #[test]
    fn stream_blocking_delivers_items_in_order() {
        let receiver = stream_blocking(4, |sender| {
            for i in 0..100 {
                if sender.send(i).is_err() {
                    return;
                }
            }
        });

        let received: Vec<u32> = runtime()
            .block_on(async move {
                let mut items = Vec::new();
                while let Ok(item) = receiver.recv_async().await {
                    items.push(item);
                }
                items
            });
        assert_eq!(received, (0..100).collect::<Vec<u32>>());
    }
}
