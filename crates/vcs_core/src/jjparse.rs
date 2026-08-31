//! `jj` output parsing — template builders and JSON-lines parsers.
//!
//! jj 0.44 has no native machine-readable output, so the commands are run with
//! a `-T <template>` expression (from the `*_template()` builders here) that
//! emits exactly one JSON object per line, and the `parse_*` functions turn the
//! captured stdout back into typed records.
//!
//! Template notes (verified against the jj templating docs and test suite):
//! - JSON fragments are single-quoted string literals (no escape syntax), so
//!   quotes stay literal; `"@"` and `"\n"` use double quotes where escapes are
//!   needed.
//! - Every value is emitted through `json()` so quotes and control characters
//!   are escaped; `stringify()` converts ref symbols to plain strings.
//! - `jj log` / `jj op log` still print graph characters unless `--no-graph`
//!   is passed — callers must pass it for parseable output.
//! - Top-level 0-argument methods are keywords and are written without
//!   parentheses (`commit_id`, `description`, `time`); methods on their
//!   results and on lambda-bound variables use parentheses (`commit_id.short()`,
//!   `c.commit_id()`).
//!
//! Parsers are pure: no I/O, no process spawning, no panics on malformed
//! input — blank and malformed lines are skipped.

/// One commit as rendered by [`log_template`] (`jj log --no-graph -T ...`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct JjCommit {
    /// Full 40-hex commit id.
    pub commit_id: String,
    /// Short commit id as shown by jj.
    pub commit_id_short: String,
    /// Full 32-hex change id.
    pub change_id: String,
    /// Short change id as shown by jj.
    pub change_id_short: String,
    /// First line of the description (jj descriptions end with `\n` when set).
    pub description: String,
    /// Short commit ids of non-root parents (root is filtered out).
    pub parents: Vec<String>,
    /// Bookmark names; remote bookmarks carry the `name@remote` suffix.
    pub bookmarks: Vec<String>,
    /// Tag names; remote tags carry the `name@remote` suffix.
    pub tags: Vec<String>,
    /// True when the commit contains merge conflicts.
    pub is_conflict: bool,
    /// True when the commit modifies no files.
    pub is_empty: bool,
    /// Author email address.
    pub author_email: String,
    /// Committer timestamp in RFC 3339-like format (e.g. `2001-02-03T04:05:09+07:00`).
    pub timestamp: String,
}

/// One working-copy change as rendered by `jj status`.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct JjFileStatus {
    /// jj status code: `M` (modified), `A` (added), `D` (deleted),
    /// `R` (renamed), `C` (copied) or `?` (untracked).
    pub status: String,
    /// Path relative to the repo root. For renames/copies this is the new path.
    pub path: String,
    /// Original path for renames (`R {old => new}`) and copies (`C`);
    /// `None` for all other statuses.
    pub original_path: Option<String>,
}

/// One operation as rendered by [`op_log_template`] (`jj op log -G -T ...`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct JjOpEntry {
    /// Short operation id (12 hex digits by default).
    pub op_id: String,
    /// First line of the operation description.
    pub description: String,
    /// Operation start timestamp (RFC 3339-like).
    pub timestamp: String,
}

/// One bookmark as rendered by [`bookmark_template`] (`jj bookmark list -T ...`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct JjBookmark {
    /// Bookmark name; remote bookmarks carry the `name@remote` suffix.
    pub name: String,
    /// Short change id of the target commit; empty when the bookmark is conflicted.
    pub target: String,
}

/// One tag as rendered by [`tag_template`] (`jj tag list -T ...`).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct JjTag {
    /// Tag name; remote tags carry the `name@remote` suffix.
    pub name: String,
    /// Short change id of the target commit; empty when the tag is conflicted.
    pub target: String,
}

/// `jj log --no-graph -T` expression: one JSON commit object per line.
///
/// Fields mirror [`JjCommit`]. `parents` filters out the virtual root commit;
/// bookmark/tag names are built with `separate("@", name, remote)` so remote
/// refs serialize as `name@remote`.
pub fn log_template() -> &'static str {
    r#"'{ "commit_id": ' ++ json(commit_id) ++ ', "commit_id_short": ' ++ json(commit_id.short()) ++ ', "change_id": ' ++ json(change_id) ++ ', "change_id_short": ' ++ json(change_id.short()) ++ ', "description": ' ++ json(description.first_line()) ++ ', "parents": ' ++ json(parents.filter(|c| !c.root()).map(|c| c.commit_id().short())) ++ ', "bookmarks": ' ++ json(bookmarks.map(|b| stringify(separate("@", b.name(), b.remote())))) ++ ', "tags": ' ++ json(tags.map(|t| stringify(separate("@", t.name(), t.remote())))) ++ ', "is_conflict": ' ++ json(conflict) ++ ', "is_empty": ' ++ json(empty) ++ ', "author_email": ' ++ json(author.email()) ++ ', "timestamp": ' ++ json(committer.timestamp()) ++ ' }' ++ "\n""#
}

/// `jj op log --no-graph -T` expression: one JSON operation object per line.
///
/// Fields mirror [`JjOpEntry`]. The operation context type is `Operation`, so
/// `id` is the operation id and `time.start()` the start timestamp.
pub fn op_log_template() -> &'static str {
    r#"'{ "op_id": ' ++ json(id.short()) ++ ', "description": ' ++ json(description.first_line()) ++ ', "timestamp": ' ++ json(time.start()) ++ ' }' ++ "\n""#
}

/// `jj bookmark list -T` expression: one JSON bookmark object per line.
///
/// Fields mirror [`JjBookmark`]. The per-entry context type is `CommitRef`;
/// `try(..., "")` keeps the target empty for conflicted bookmarks.
pub fn bookmark_template() -> &'static str {
    r#"'{ "name": ' ++ json(stringify(separate("@", name, remote))) ++ ', "target": ' ++ json(try(normal_target.change_id().short(), "")) ++ ' }' ++ "\n""#
}

/// `jj tag list -T` expression: one JSON tag object per line.
///
/// Fields mirror [`JjTag`]. The per-entry context type is `CommitRef` (same as
/// bookmark list); tags have no remote, so `name()` alone is used.
pub fn tag_template() -> &'static str {
    r#"'{ "name": ' ++ json(stringify(name)) ++ ', "target": ' ++ json(try(normal_target.change_id().short(), "")) ++ ' }' ++ "\n""#
}

/// Deserialize one JSON object per non-blank line; malformed lines are skipped.
fn parse_jsonl<T: serde::de::DeserializeOwned>(output: &str) -> Vec<T> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect()
}

/// Parse `jj log --no-graph -T <log_template()>` output into commits.
pub fn parse_commits_jsonl(output: &str) -> Vec<JjCommit> {
    parse_jsonl(output)
}

/// Parse `jj op log -G -T <op_log_template()>` output into operations.
pub fn parse_ops_jsonl(output: &str) -> Vec<JjOpEntry> {
    parse_jsonl(output)
}

/// Parse `jj bookmark list -T <bookmark_template()>` output into bookmarks.
pub fn parse_bookmarks_jsonl(output: &str) -> Vec<JjBookmark> {
    parse_jsonl(output)
}

/// Parse `jj tag list -T <tag_template()>` output into tags.
pub fn parse_tags_jsonl(output: &str) -> Vec<JjTag> {
    parse_jsonl(output)
}

/// Parse `jj status` output into working-copy change records.
///
/// Only the `Working copy changes:` and `Untracked paths:` sections are read.
/// Summary lines (`Working copy  (@) : ...`, `Parent commit (@-): ...`),
/// warnings and hints do not match the change-line grammar and are ignored, so
/// `"The working copy has no changes."` simply yields an empty vec.
pub fn parse_status_output(output: &str) -> Vec<JjFileStatus> {
    let mut files = Vec::new();
    let mut in_changes = false;
    for line in output.lines() {
        let line = line.trim();
        if matches!(line, "Working copy changes:" | "Untracked paths:") {
            in_changes = true;
            continue;
        }
        if !in_changes {
            continue;
        }
        if let Some(file) = parse_status_line(line) {
            files.push(file);
        }
    }
    files
}

/// Parse a single `jj status` change line into a [`JjFileStatus`].
///
/// Handles the current rename/copy syntax `R {old => new}` (jj 0.44) and the
/// legacy `R old -> new` form. Returns `None` for anything that does not look
/// like a change line.
fn parse_status_line(line: &str) -> Option<JjFileStatus> {
    let (code, rest) = line.split_once(char::is_whitespace)?;
    if !matches!(code, "M" | "A" | "D" | "R" | "C" | "?") {
        return None;
    }
    let rest = rest.trim();
    if rest.is_empty() {
        return None;
    }
    if code == "R" || code == "C" {
        let (old, new) = rest
            .strip_prefix('{')
            .and_then(|inner| inner.strip_suffix('}'))
            .and_then(|inner| inner.split_once(" => "))
            .or(rest.split_once(" -> "))?;
        return Some(JjFileStatus {
            status: code.to_string(),
            path: new.trim().to_string(),
            original_path: Some(old.trim().to_string()),
        });
    }
    Some(JjFileStatus {
        status: code.to_string(),
        path: rest.to_string(),
        original_path: None,
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::*;
    use crate::jj::{run_jj_blocking, DEFAULT_TIMEOUT};

    /// Whether a real `jj` binary is available for integration tests. Mirrors
    /// the resolution used by `crate::jj` (`JJ_BIN` override, else PATH).
    fn jj_available() -> bool {
        let binary = std::env::var("JJ_BIN").unwrap_or_else(|_| "jj".to_string());
        std::process::Command::new(binary)
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
    fn jj_stdout(repo: &std::path::Path, args: &[&str]) -> String {
        let output = run_jj_blocking(repo, args, DEFAULT_TIMEOUT).expect("jj command must succeed");
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    #[test]
    fn parses_commits_jsonl_and_skips_malformed_lines() {
        let sample = r#"{"commit_id":"b1cb6b2f9141e6ffee18532a8bf9a2075ca02606","commit_id_short":"b1cb6b2f9141","change_id":"kkmpptxzrspxrzommnulwmwkkqwworpl","change_id_short":"kkmpptxzr","description":"second","parents":["68a505386f93"],"bookmarks":["main","feature@origin"],"tags":["v1.0"],"is_conflict":false,"is_empty":true,"author_email":"a@example.com","timestamp":"2001-02-03T04:05:09+07:00"}
        this line is not JSON
        {"commit_id":"68a505386f936fff6d718f55005e77ea72589bc1","commit_id_short":"68a505386f93","change_id":"qpvuntsmwlqtpsluzzsnyyzlmlwvmlnu","change_id_short":"qpvuntsmw","description":"first","parents":[],"bookmarks":[],"tags":[],"is_conflict":true,"is_empty":false,"author_email":"","timestamp":"2001-02-03T04:05:08+07:00"}
        "#;
        let commits = parse_commits_jsonl(sample);
        assert_eq!(commits.len(), 2, "malformed line must be skipped");
        assert_eq!(
            commits[0],
            JjCommit {
                commit_id: "b1cb6b2f9141e6ffee18532a8bf9a2075ca02606".to_string(),
                commit_id_short: "b1cb6b2f9141".to_string(),
                change_id: "kkmpptxzrspxrzommnulwmwkkqwworpl".to_string(),
                change_id_short: "kkmpptxzr".to_string(),
                description: "second".to_string(),
                parents: vec!["68a505386f93".to_string()],
                bookmarks: vec!["main".to_string(), "feature@origin".to_string()],
                tags: vec!["v1.0".to_string()],
                is_conflict: false,
                is_empty: true,
                author_email: "a@example.com".to_string(),
                timestamp: "2001-02-03T04:05:09+07:00".to_string(),
            }
        );
        assert!(commits[1].is_conflict);
        assert!(!commits[1].is_empty);
        assert!(commits[1].parents.is_empty());
        assert_eq!(commits[1].author_email, "");
    }

    #[test]
    fn parses_status_output_sections() {
        let sample = "Working copy changes:\n\
            M src/main.rs\n\
            A new-file.txt\n\
            D deleted.txt\n\
            R {old-name.txt => new-name.txt}\n\
            C {copy-source => copy-target}\n\
            Untracked paths:\n\
            ? untracked.log\n\
            ? sub/\n\
            Working copy  (@) : rlvkpnrz c2fce842 (no description set)\n\
            Parent commit (@-): qpvuntsm ebf799bc (no description set)\n";
        let files = parse_status_output(sample);
        let rows: Vec<_> = files
            .iter()
            .map(|f| {
                (
                    f.status.as_str(),
                    f.path.as_str(),
                    f.original_path.as_deref(),
                )
            })
            .collect();
        assert_eq!(
            rows,
            vec![
                ("M", "src/main.rs", None),
                ("A", "new-file.txt", None),
                ("D", "deleted.txt", None),
                ("R", "new-name.txt", Some("old-name.txt")),
                ("C", "copy-target", Some("copy-source")),
                ("?", "untracked.log", None),
                ("?", "sub/", None),
            ]
        );
    }

    #[test]
    fn parses_legacy_status_rename_syntax() {
        let sample = "Working copy changes:\nR old-name.txt -> new-name.txt\n";
        let files = parse_status_output(sample);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, "R");
        assert_eq!(files[0].path, "new-name.txt");
        assert_eq!(files[0].original_path.as_deref(), Some("old-name.txt"));
    }

    #[test]
    fn status_output_without_changes_is_empty() {
        let sample = "The working copy has no changes.\n\
            Working copy  (@) : rlvkpnrz c2fce842 (no description set)\n\
            Parent commit (@-): qpvuntsm ebf799bc (no description set)\n";
        assert!(parse_status_output(sample).is_empty());
    }

    #[test]
    fn parses_ops_jsonl() {
        let sample = r#"{"op_id":"a1b2c3d4e5f6","description":"snapshot working copy","timestamp":"2001-02-03T04:05:09+07:00"}
        {"op_id":"f6e5d4c3b2a1","description":"repo initialized","timestamp":"2001-02-03T04:05:08+07:00"}
        "#;
        let ops = parse_ops_jsonl(sample);
        assert_eq!(ops.len(), 2);
        assert_eq!(ops[0].op_id, "a1b2c3d4e5f6");
        assert_eq!(ops[0].description, "snapshot working copy");
        assert_eq!(ops[0].timestamp, "2001-02-03T04:05:09+07:00");
        assert_eq!(ops[1].op_id, "f6e5d4c3b2a1");
    }

    #[test]
    fn parses_bookmarks_jsonl() {
        let sample = r#"{"name":"main","target":"kkmpptxzr"}
        {"name":"feature@origin","target":"qpvuntsmw"}
        "#;
        let bookmarks = parse_bookmarks_jsonl(sample);
        assert_eq!(bookmarks.len(), 2);
        assert_eq!(bookmarks[0].name, "main");
        assert_eq!(bookmarks[0].target, "kkmpptxzr");
        assert_eq!(bookmarks[1].name, "feature@origin");
    }

    #[test]
    fn parses_tags_jsonl() {
        let sample = r#"{"name":"v1.0","target":"kkmpptxzr"}
        "#;
        let tags = parse_tags_jsonl(sample);
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].name, "v1.0");
        assert_eq!(tags[0].target, "kkmpptxzr");
    }

    #[test]
    fn commits_round_trip_via_real_jj() {
        if !jj_available() {
            eprintln!("skipping: jj not installed");
            return;
        }
        let (_temp, repo) = init_repo();
        jj_stdout(&repo, &["describe", "-m", "first"]);
        jj_stdout(&repo, &["new", "-m", "second"]);
        jj_stdout(&repo, &["bookmark", "create", "-r", "@", "mybookmark"]);
        jj_stdout(&repo, &["tag", "set", "-r", "@", "v0.1.0"]);

        let stdout = jj_stdout(&repo, &["log", "--no-graph", "-T", log_template()]);
        let commits = parse_commits_jsonl(&stdout);
        assert_eq!(
            commits.len(),
            3,
            "expected root + first + second, got:\n{stdout}"
        );

        let second = commits
            .iter()
            .find(|c| c.description == "second")
            .expect("second commit");
        assert_eq!(second.bookmarks, vec!["mybookmark".to_string()]);
        assert_eq!(second.tags, vec!["v0.1.0".to_string()]);
        assert!(second.is_empty, "jj new -m creates an empty commit");
        assert!(!second.is_conflict);
        assert_eq!(second.parents.len(), 1);
        assert!(!second.commit_id_short.is_empty());
        assert!(!second.change_id_short.is_empty());

        let root = commits
            .iter()
            .find(|c| c.commit_id == "0000000000000000000000000000000000000000")
            .expect("root commit");
        assert!(root.parents.is_empty(), "root parent must be filtered out");
    }

    #[test]
    fn status_round_trip_via_real_jj() {
        if !jj_available() {
            eprintln!("skipping: jj not installed");
            return;
        }
        let (_temp, repo) = init_repo();
        std::fs::write(repo.join("a.txt"), "a\n").expect("write a.txt");
        std::fs::write(repo.join("b.txt"), "b\n").expect("write b.txt");
        jj_stdout(&repo, &["describe", "-m", "init files"]);
        jj_stdout(&repo, &["new", "-m", "next"]);
        std::fs::remove_file(repo.join("a.txt")).expect("remove a.txt");
        std::fs::write(repo.join("c.txt"), "c\n").expect("write c.txt");

        let stdout = jj_stdout(&repo, &["status"]);
        let files = parse_status_output(&stdout);
        let mut rows: Vec<_> = files
            .iter()
            .map(|f| {
                (
                    f.status.as_str(),
                    f.path.as_str(),
                    f.original_path.as_deref(),
                )
            })
            .collect();
        rows.sort();
        assert_eq!(
            rows,
            vec![("A", "c.txt", None), ("D", "a.txt", None)],
            "got:\n{stdout}"
        );
    }

    #[test]
    fn op_log_round_trip_via_real_jj() {
        if !jj_available() {
            eprintln!("skipping: jj not installed");
            return;
        }
        let (_temp, repo) = init_repo();
        jj_stdout(&repo, &["describe", "-m", "first"]);
        jj_stdout(&repo, &["new", "-m", "second"]);

        let stdout = jj_stdout(&repo, &["op", "log", "--no-graph", "-T", op_log_template()]);
        let ops = parse_ops_jsonl(&stdout);
        assert!(!ops.is_empty(), "got:\n{stdout}");
        for op in &ops {
            assert!(!op.op_id.is_empty(), "unexpected op: {op:?}");
            assert!(!op.timestamp.is_empty(), "unexpected op: {op:?}");
        }
    }

    #[test]
    fn bookmarks_round_trip_via_real_jj() {
        if !jj_available() {
            eprintln!("skipping: jj not installed");
            return;
        }
        let (_temp, repo) = init_repo();
        jj_stdout(&repo, &["bookmark", "create", "-r", "@", "mybookmark"]);

        let stdout = jj_stdout(&repo, &["bookmark", "list", "-T", bookmark_template()]);
        let bookmarks = parse_bookmarks_jsonl(&stdout);
        assert_eq!(bookmarks.len(), 1, "got:\n{stdout}");
        assert_eq!(bookmarks[0].name, "mybookmark");
        assert!(!bookmarks[0].target.is_empty());
    }

    #[test]
    fn tags_round_trip_via_real_jj() {
        if !jj_available() {
            eprintln!("skipping: jj not installed");
            return;
        }
        let (_temp, repo) = init_repo();
        jj_stdout(&repo, &["tag", "set", "-r", "@", "v0.1.0"]);

        let stdout = jj_stdout(&repo, &["tag", "list", "-T", tag_template()]);
        let tags = parse_tags_jsonl(&stdout);
        assert_eq!(tags.len(), 1, "got:\n{stdout}");
        assert_eq!(tags[0].name, "v0.1.0");
        assert!(!tags[0].target.is_empty());
    }
}
