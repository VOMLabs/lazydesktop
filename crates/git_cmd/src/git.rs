//! Git CLI command execution.
//!
//! Provides typed wrappers around common git commands. Used by the GPUI app
//! (`crates/app/src/git_service.rs`) on `tokio::task::spawn_blocking` workers.

use std::path::Path;
use std::process::Command;

use crate::types::{BranchEntry, CommandResult, CommitEntry, FileStatus, VcsError};

/// Run a git command in a repository directory.
pub fn run_git(repo_path: &Path, args: &[&str]) -> Result<CommandResult, VcsError> {
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(args)
        .output()?;

    Ok(CommandResult {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

/// Check if git is available on PATH.
pub fn is_git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Parse `git status --porcelain` output into file statuses.
pub fn parse_status(output: &str) -> Vec<FileStatus> {
    let mut statuses = Vec::new();

    for line in output.lines() {
        if line.len() < 4 {
            continue;
        }
        let status = if line.as_bytes()[0] != b' ' {
            line.chars().next().unwrap()
        } else {
            line.chars().nth(1).unwrap()
        };
        let path = line[3..].trim().to_string();
        statuses.push(FileStatus { status, path });
    }

    statuses
}

/// Parse `git log --oneline` output into commit entries.
pub fn parse_oneline_log(output: &str) -> Vec<CommitEntry> {
    let mut entries = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        if let Some(space_pos) = line.find(' ') {
            let hash = line[..space_pos].to_string();
            let subject = line[space_pos + 1..].to_string();
            entries.push(CommitEntry {
                hash,
                subject,
                author: String::new(),
                date: String::new(),
            });
        }
    }

    entries
}

/// Parse `git branch` output into branch entries.
pub fn parse_branch_list(output: &str) -> Vec<BranchEntry> {
    let mut entries = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let (current, name) = if let Some(stripped) = line.strip_prefix("* ") {
            (true, stripped.trim())
        } else {
            (false, line.trim())
        };

        entries.push(BranchEntry {
            name: name.to_string(),
            current,
        });
    }

    entries
}

/// Get the current branch name.
pub fn current_branch(repo_path: &Path) -> Result<String, VcsError> {
    let result = run_git(repo_path, &["symbolic-ref", "--short", "HEAD"])?;
    if result.success() {
        Ok(result.stdout.trim().to_string())
    } else {
        // Detached HEAD — fall back to short hash
        let result = run_git(repo_path, &["rev-parse", "--short", "HEAD"])?;
        Ok(result.stdout.trim().to_string())
    }
}

/// Get the list of branches.
pub fn list_branches(repo_path: &Path) -> Result<Vec<BranchEntry>, VcsError> {
    let result = run_git(repo_path, &["branch", "-a"])?;
    if result.success() {
        Ok(parse_branch_list(&result.stdout))
    } else {
        Err(VcsError::CommandFailed(result.stderr))
    }
}

/// Get `git status --porcelain`.
pub fn status(repo_path: &Path) -> Result<Vec<FileStatus>, VcsError> {
    let result = run_git(repo_path, &["status", "--porcelain"])?;
    Ok(parse_status(&result.stdout))
}

/// Get the commit log.
pub fn log(repo_path: &Path, limit: usize) -> Result<Vec<CommitEntry>, VcsError> {
    let limit_str = format!("{}", limit);
    let result = run_git(repo_path, &["log", "--oneline", "-n", &limit_str])?;
    if result.success() {
        Ok(parse_oneline_log(&result.stdout))
    } else {
        Err(VcsError::CommandFailed(result.stderr))
    }
}

/// Stage files.
pub fn add_files(repo_path: &Path, files: &[&str]) -> Result<CommandResult, VcsError> {
    let mut args = vec!["add", "--"];
    args.extend(files);
    run_git(repo_path, &args)
}

/// Commit with a message.
pub fn commit(repo_path: &Path, message: &str) -> Result<CommandResult, VcsError> {
    run_git(repo_path, &["commit", "-m", message])
}

/// Push to the remote.
pub fn push(repo_path: &Path) -> Result<CommandResult, VcsError> {
    run_git(repo_path, &["push"])
}

/// Fetch from the remote.
pub fn fetch(repo_path: &Path) -> Result<CommandResult, VcsError> {
    run_git(repo_path, &["fetch"])
}

/// Pull from the remote.
pub fn pull(repo_path: &Path) -> Result<CommandResult, VcsError> {
    run_git(repo_path, &["pull"])
}

/// Checkout a branch.
pub fn checkout(repo_path: &Path, branch: &str) -> Result<CommandResult, VcsError> {
    run_git(repo_path, &["checkout", branch])
}

/// Create a new branch.
pub fn create_branch(repo_path: &Path, name: &str) -> Result<CommandResult, VcsError> {
    run_git(repo_path, &["checkout", "-b", name])
}

/// Delete a branch.
pub fn delete_branch(repo_path: &Path, name: &str) -> Result<CommandResult, VcsError> {
    run_git(repo_path, &["branch", "-D", name])
}

/// Get the diff for a file.
pub fn diff_file(repo_path: &Path, file: &str) -> Result<String, VcsError> {
    let result = run_git(repo_path, &["diff", "HEAD", "--", file])?;
    if result.stdout.is_empty() {
        // Try unstaged diff
        let result = run_git(repo_path, &["diff", "--", file])?;
        Ok(result.stdout)
    } else {
        Ok(result.stdout)
    }
}

/// Check if a repository is dirty (has uncommitted changes).
pub fn is_dirty(repo_path: &Path) -> bool {
    run_git(repo_path, &["diff", "--quiet"])
        .map(|r| !r.success())
        .unwrap_or(false)
}

/// Get the list of files changed in a commit.
pub fn commit_files(repo_path: &Path, hash: &str) -> Result<Vec<FileStatus>, VcsError> {
    let result = run_git(
        repo_path,
        &["diff-tree", "--no-commit-id", "-r", "--name-status", hash],
    )?;
    if result.success() {
        Ok(parse_status(&result.stdout))
    } else {
        Err(VcsError::CommandFailed(result.stderr))
    }
}

/// Get the full diff introduced by a commit (`git show`).
///
/// Uses `--format=fuller` so the raw output starts with the commit header and
/// message (rendered as context by the diff view) followed by the unified
/// diff. `--no-ext-diff` keeps output deterministic.
pub fn show_commit(repo_path: &Path, hash: &str) -> Result<String, VcsError> {
    let result = run_git(
        repo_path,
        &["show", "--format=fuller", "--no-ext-diff", hash],
    )?;
    if result.success() {
        Ok(result.stdout)
    } else {
        Err(VcsError::CommandFailed(result.stderr))
    }
}

/// Read a global git config value. Unset keys return an empty string.
///
/// The working directory is irrelevant for `--global` lookups.
pub fn config_get_global(name: &str) -> Result<String, VcsError> {
    let result = run_git(Path::new("."), &["config", "--global", "--get", name])?;
    // Exit code 1 means the key is not set; any other failure is surfaced
    // through the error, and callers treat the value as empty when unset.
    Ok(result.stdout.trim().to_string())
}

/// Set a global git config value.
pub fn config_set_global(name: &str, value: &str) -> Result<CommandResult, VcsError> {
    run_git(Path::new("."), &["config", "--global", name, value])
}

/// Clone a repository.
pub fn clone(url: &str, dest: &Path) -> Result<CommandResult, VcsError> {
    let output = Command::new("git")
        .arg("clone")
        .arg(url)
        .arg(dest)
        .output()?;

    Ok(CommandResult {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

/// Initialize a new repository.
pub fn init(path: &Path) -> Result<CommandResult, VcsError> {
    let output = Command::new("git").arg("init").current_dir(path).output()?;

    Ok(CommandResult {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

/// Check if a path is a git repository.
pub fn is_git_repo(path: &Path) -> bool {
    path.join(".git").is_dir()
}

/// Rename the current branch (`git branch -m`).
pub fn rename_branch(repo_path: &Path, name: &str) -> Result<CommandResult, VcsError> {
    run_git(repo_path, &["branch", "-m", name])
}

/// Stash all working-tree changes (`git stash push`).
pub fn stash_push(repo_path: &Path) -> Result<CommandResult, VcsError> {
    run_git(repo_path, &["stash", "push"])
}

/// List stashes (`git stash list`). Returns one entry per line.
pub fn stash_list(repo_path: &Path) -> Vec<String> {
    run_git(repo_path, &["stash", "list"])
        .map(|r| {
            r.stdout
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        })
        .unwrap_or_default()
}

/// Restore the most recent stash (`git stash pop`).
pub fn stash_pop(repo_path: &Path) -> Result<CommandResult, VcsError> {
    run_git(repo_path, &["stash", "pop"])
}

/// Reset the index to HEAD, keeping working-tree changes (`git reset`).
///
/// Equivalent to unstaging everything; never discards file contents.
pub fn reset_mixed(repo_path: &Path) -> Result<CommandResult, VcsError> {
    run_git(repo_path, &["reset"])
}

/// Recent commit subjects (`git log --oneline -<limit>`), used as style
/// reference when generating AI commit messages.
pub fn recent_subjects(repo_path: &Path, limit: usize) -> Vec<String> {
    let limit_arg = format!("-{limit}");
    run_git(repo_path, &["log", "--oneline", &limit_arg])
        .map(|r| {
            parse_oneline_log(&r.stdout)
                .into_iter()
                .map(|e| e.subject)
                .collect()
        })
        .unwrap_or_default()
}

/// Raw bytes of a file at a revision (`git show <hash>:<path>`).
///
/// Unlike `run_git` — which decodes stdout lossily as UTF-8 — this preserves
/// binary content so image files can be written to disk and rendered inline
/// by the diff viewer.
pub fn show_file_bytes(repo_path: &Path, hash: &str, path: &str) -> Result<Vec<u8>, VcsError> {
    let rev_path = format!("{hash}:{path}");
    let output = Command::new("git")
        .current_dir(repo_path)
        .args(["show", &rev_path])
        .output()?;
    if output.status.success() {
        Ok(output.stdout)
    } else {
        Err(VcsError::CommandFailed(
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }
}

/// Recent commit authors as `Name <email>` lines (`git log --pretty=%an <%ae>`),
/// deduplicated in recency order — used for co-author suggestions.
pub fn recent_authors(repo_path: &Path, limit: usize) -> Vec<String> {
    let limit_arg = format!("-{limit}");
    run_git(repo_path, &["log", "--pretty=%an <%ae>", &limit_arg])
        .map(|r| parse_author_lines(&r.stdout))
        .unwrap_or_default()
}

/// Parse `git log --pretty=%an <%ae>` output into deduplicated author lines.
fn parse_author_lines(output: &str) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    output
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .filter(|l| seen.insert(l.clone()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn parse_status_basic() {
        let output = " M src/main.cpp\nA  src/new.rs\nD  src/old.rs\n?? untracked.txt\n";
        let statuses = parse_status(output);
        assert_eq!(statuses.len(), 4);
        assert_eq!(statuses[0].status, 'M');
        assert_eq!(statuses[0].path, "src/main.cpp");
        assert_eq!(statuses[1].status, 'A');
        assert_eq!(statuses[2].status, 'D');
        assert_eq!(statuses[3].status, '?');
    }

    #[test]
    fn parse_oneline_log_basic() {
        let output = "abc1234 Fix bug\ndef5678 Add feature\n";
        let entries = parse_oneline_log(output);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].hash, "abc1234");
        assert_eq!(entries[0].subject, "Fix bug");
    }

    #[test]
    fn parse_branch_list_basic() {
        let output = "  main\n* feature\n  develop\n";
        let entries = parse_branch_list(output);
        assert_eq!(entries.len(), 3);
        assert!(!entries[0].current);
        assert!(entries[1].current);
        assert_eq!(entries[1].name, "feature");
    }

    #[test]
    fn is_git_available_works() {
        // Git should be available in CI
        assert!(is_git_available());
    }

    #[test]
    fn init_and_status() {
        let tmp = TempDir::new().unwrap();
        init(tmp.path()).unwrap();
        assert!(is_git_repo(tmp.path()));

        let statuses = status(tmp.path()).unwrap();
        assert!(statuses.is_empty());
    }

    #[test]
    fn show_commit_returns_unified_diff() {
        use std::process::Command;

        let tmp = TempDir::new().unwrap();
        init(tmp.path()).unwrap();

        // Configure an author locally so `git commit` succeeds in CI.
        Command::new("git")
            .current_dir(tmp.path())
            .args(["config", "user.name", "LazyDesktop Test"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(tmp.path())
            .args(["config", "user.email", "test@example.com"])
            .output()
            .unwrap();

        std::fs::write(tmp.path().join("f.txt"), "hello\n").unwrap();
        Command::new("git")
            .current_dir(tmp.path())
            .args(["add", "f.txt"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(tmp.path())
            .args(["commit", "-m", "Add f"])
            .output()
            .unwrap();

        let out = show_commit(tmp.path(), "HEAD").unwrap();
        assert!(out.contains("commit "), "missing commit header: {out}");
        assert!(out.contains("diff --git"), "missing diff header: {out}");
        assert!(out.contains("+hello"), "missing added line: {out}");
    }

    /// Helper: initialize a temp repo with a local identity and one commit.
    fn init_repo(tmp: &TempDir) {
        use std::process::Command;

        init(tmp.path()).unwrap();
        Command::new("git")
            .current_dir(tmp.path())
            .args(["config", "user.name", "LazyDesktop Test"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(tmp.path())
            .args(["config", "user.email", "test@example.com"])
            .output()
            .unwrap();
        std::fs::write(tmp.path().join("f.txt"), "hello\n").unwrap();
        Command::new("git")
            .current_dir(tmp.path())
            .args(["add", "f.txt"])
            .output()
            .unwrap();
        Command::new("git")
            .current_dir(tmp.path())
            .args(["commit", "-m", "Add f"])
            .output()
            .unwrap();
    }

    #[test]
    fn rename_branch_renames_current() {
        let tmp = TempDir::new().unwrap();
        init_repo(&tmp);

        rename_branch(tmp.path(), "renamed").unwrap();
        assert_eq!(current_branch(tmp.path()).unwrap(), "renamed");
    }

    #[test]
    fn stash_push_list_pop_roundtrip() {
        let tmp = TempDir::new().unwrap();
        init_repo(&tmp);

        // Make a dirty change.
        std::fs::write(tmp.path().join("f.txt"), "modified\n").unwrap();
        assert!(is_dirty(tmp.path()));

        stash_push(tmp.path()).unwrap();
        assert!(!is_dirty(tmp.path()), "stash should clean the tree");
        let stashes = stash_list(tmp.path());
        assert_eq!(stashes.len(), 1, "one stash expected: {stashes:?}");

        stash_pop(tmp.path()).unwrap();
        assert!(is_dirty(tmp.path()), "pop should restore the change");
        assert!(stash_list(tmp.path()).is_empty());
    }

    #[test]
    fn reset_mixed_unstages_keeps_worktree() {
        let tmp = TempDir::new().unwrap();
        init_repo(&tmp);

        std::fs::write(tmp.path().join("f.txt"), "modified\n").unwrap();
        add_files(tmp.path(), &["f.txt"]).unwrap();
        assert!(!status(tmp.path()).unwrap().is_empty());

        reset_mixed(tmp.path()).unwrap();
        // File contents are preserved; only the index is reset.
        assert_eq!(
            std::fs::read_to_string(tmp.path().join("f.txt")).unwrap(),
            "modified\n"
        );
    }

    #[test]
    fn recent_subjects_returns_subjects() {
        let tmp = TempDir::new().unwrap();
        init_repo(&tmp);

        let subjects = recent_subjects(tmp.path(), 5);
        assert_eq!(subjects, vec!["Add f".to_string()]);
    }

    #[test]
    fn parse_author_lines_deduplicates_in_order() {
        let output = "Alice <alice@example.com>\n\
                      Bob <bob@example.com>\n\
                      Alice <alice@example.com>\n\
                      \n";
        let authors = parse_author_lines(output);
        assert_eq!(
            authors,
            vec![
                "Alice <alice@example.com>".to_string(),
                "Bob <bob@example.com>".to_string(),
            ]
        );
    }

    #[test]
    fn recent_authors_returns_author_lines() {
        let tmp = TempDir::new().unwrap();
        init_repo(&tmp);

        let authors = recent_authors(tmp.path(), 5);
        assert_eq!(
            authors,
            vec!["LazyDesktop Test <test@example.com>".to_string()]
        );
    }

    #[test]
    fn show_file_bytes_returns_raw_content() {
        let tmp = TempDir::new().unwrap();
        init_repo(&tmp);

        let bytes = show_file_bytes(tmp.path(), "HEAD", "f.txt").unwrap();
        assert_eq!(bytes, b"hello\n");
    }
}
