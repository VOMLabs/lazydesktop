//! Shared types for VCS command execution.

/// The kind of version control system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VcsKind {
    Git,
    Jujutsu,
}

impl VcsKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            VcsKind::Git => "git",
            VcsKind::Jujutsu => "jujutsu",
        }
    }
}

/// Result of a VCS command execution.
#[derive(Debug, Clone)]
pub struct CommandResult {
    /// The exit code (0 = success).
    pub exit_code: i32,
    /// Standard output.
    pub stdout: String,
    /// Standard error.
    pub stderr: String,
}

impl CommandResult {
    /// Check if the command succeeded.
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// A file change entry from `git status --porcelain`.
#[derive(Debug, Clone)]
pub struct FileStatus {
    /// The status code (M, A, D, R, ?, etc.).
    pub status: char,
    /// The relative file path.
    pub path: String,
}

/// A commit entry from `git log`.
#[derive(Debug, Clone)]
pub struct CommitEntry {
    /// The commit hash (short form).
    pub hash: String,
    /// The commit subject line.
    pub subject: String,
    /// The author name.
    pub author: String,
    /// The commit date.
    pub date: String,
}

/// A branch entry from `git branch`.
#[derive(Debug, Clone)]
pub struct BranchEntry {
    /// The branch name.
    pub name: String,
    /// Whether this is the current branch.
    pub current: bool,
}

/// A remote entry.
#[derive(Debug, Clone)]
pub struct RemoteEntry {
    /// The remote name.
    pub name: String,
    /// The remote URL.
    pub url: String,
}

/// Errors from VCS operations.
#[derive(Debug, thiserror::Error)]
pub enum VcsError {
    #[error("git/jj not found on PATH")]
    NotInstalled,
    #[error("command failed: {0}")]
    CommandFailed(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(String),
}
