//! Jujutsu (jj) CLI command execution.
//!
//! Provides typed wrappers around jj commands. Used by the GPUI app on
//! `tokio::task::spawn_blocking` workers.

use std::path::Path;
use std::process::Command;

use crate::types::{CommandResult, FileStatus, VcsError};

/// Run a jj command in a repository directory.
pub fn run_jj(repo_path: &Path, args: &[&str]) -> Result<CommandResult, VcsError> {
    let output = Command::new("jj")
        .current_dir(repo_path)
        .args(args)
        .output()?;

    Ok(CommandResult {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    })
}

/// Check if jj is available on PATH.
pub fn is_jj_available() -> bool {
    Command::new("jj")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if a path is a jujutsu repository.
pub fn is_jj_repo(path: &Path) -> bool {
    path.join(".jj").is_dir()
}

/// Parse `jj status` output into file statuses.
pub fn parse_status(output: &str) -> Vec<FileStatus> {
    let mut statuses = Vec::new();
    let mut in_section = false;

    for line in output.lines() {
        if line.starts_with("Working copy changes") {
            in_section = true;
            continue;
        }
        if line.trim().is_empty() || line.ends_with(':') {
            in_section = false;
            continue;
        }
        // jj status prints: "M path", "A path", "D path", "R {old => new}"
        if in_section && line.len() >= 2 && !line.starts_with(' ') {
            let status = line.chars().next().unwrap();
            let mut path = line[2..].trim().to_string();

            // Handle renames: "R {old => new}" -> extract new path
            if status == 'R' && path.starts_with('{') {
                if let Some(arrow_pos) = path.find("=>") {
                    path = path[arrow_pos + 2..]
                        .trim()
                        .trim_end_matches('}')
                        .trim()
                        .to_string();
                }
            }

            statuses.push(FileStatus { status, path });
        }
    }

    statuses
}

/// Get the current change ID (jj equivalent of branch).
pub fn current_change_id(repo_path: &Path) -> Result<String, VcsError> {
    let result = run_jj(
        repo_path,
        &[
            "log",
            "--no-graph",
            "-r",
            "@",
            "--limit",
            "1",
            "--config",
            "ui.pagination=never",
            "-T",
            "change_id.short()",
        ],
    )?;
    Ok(result.stdout.trim().to_string())
}

/// Get the commit log.
pub fn log(repo_path: &Path, limit: usize) -> Result<Vec<crate::types::CommitEntry>, VcsError> {
    let limit_str = format!("{}", limit);
    let result = run_jj(
        repo_path,
        &[
            "log",
            "--no-graph",
            "--limit",
            &limit_str,
            "--config",
            "ui.pagination=never",
            "-T",
            "change_id.short() ++ \" \" ++ description.first_line()",
        ],
    )?;

    if result.success() {
        Ok(crate::git::parse_oneline_log(&result.stdout))
    } else {
        Err(VcsError::CommandFailed(result.stderr))
    }
}

/// Get the diff for a file.
pub fn diff_file(repo_path: &Path, file: &str) -> Result<String, VcsError> {
    let result = run_jj(
        repo_path,
        &[
            "diff",
            "--git",
            "--color",
            "never",
            "--config",
            "ui.pagination=never",
            "--",
            file,
        ],
    )?;
    Ok(result.stdout)
}

/// Restore a file (undo changes).
pub fn restore_file(repo_path: &Path, file: &str) -> Result<CommandResult, VcsError> {
    run_jj(repo_path, &["restore", file])
}

/// Check if a jj repository is dirty.
pub fn is_dirty(repo_path: &Path) -> bool {
    run_jj(
        repo_path,
        &["diff", "--git", "--config", "ui.pagination=never"],
    )
    .map(|r| !r.stdout.trim().is_empty())
    .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_status_basic() {
        let output = "Working copy changes:\nM src/main.cpp\nA src/new.rs\nD src/old.rs\n";
        let statuses = parse_status(output);
        assert_eq!(statuses.len(), 3);
        assert_eq!(statuses[0].status, 'M');
        assert_eq!(statuses[0].path, "src/main.cpp");
        assert_eq!(statuses[1].status, 'A');
        assert_eq!(statuses[2].status, 'D');
    }

    #[test]
    fn parse_status_rename() {
        let output = "Working copy changes:\nR {old.txt => new.txt}\n";
        let statuses = parse_status(output);
        assert_eq!(statuses.len(), 1);
        assert_eq!(statuses[0].status, 'R');
        assert_eq!(statuses[0].path, "new.txt");
    }
}
