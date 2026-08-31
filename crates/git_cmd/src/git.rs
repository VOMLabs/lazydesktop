//! Git CLI command execution.
//!
//! Provides typed wrappers around common git commands that were previously
//! executed via QProcess in mainwindow.cpp.

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
}
