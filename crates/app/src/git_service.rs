//! Async wrapper around git_cmd for GPUI integration.
//!
//! Provides spawn-based async execution so git operations don't block the UI.

use std::path::PathBuf;

use git_cmd::{git, types::*};

/// Shared state for git operations on a repository.
pub struct GitService {
    pub repo_path: PathBuf,
    pub file_statuses: Vec<FileStatus>,
    pub branches: Vec<BranchEntry>,
    pub current_branch: String,
    pub commit_history: Vec<CommitEntry>,
    pub is_dirty: bool,
}

impl GitService {
    pub fn new(repo_path: PathBuf) -> Self {
        Self {
            repo_path,
            file_statuses: Vec::new(),
            branches: Vec::new(),
            current_branch: String::new(),
            commit_history: Vec::new(),
            is_dirty: false,
        }
    }

    /// Refresh all git state (status, branches, log).
    pub fn refresh_all(&mut self) {
        self.file_statuses = git::status(&self.repo_path).unwrap_or_default();
        self.branches = git::list_branches(&self.repo_path).unwrap_or_default();
        self.current_branch = git::current_branch(&self.repo_path).unwrap_or_default();
        self.commit_history = git::log(&self.repo_path, 50).unwrap_or_default();
        self.is_dirty = git::is_dirty(&self.repo_path);
    }

    /// Stage files by path.
    pub fn stage_files(&self, files: &[&str]) -> Result<CommandResult, VcsError> {
        git::add_files(&self.repo_path, files)
    }

    /// Unstage files.
    pub fn unstage_files(&self, files: &[&str]) -> Result<CommandResult, VcsError> {
        let mut args = vec!["reset", "HEAD", "--"];
        args.extend(files);
        git::run_git(&self.repo_path, &args)
    }

    /// Commit with message.
    pub fn commit(&self, message: &str) -> Result<CommandResult, VcsError> {
        git::commit(&self.repo_path, message)
    }

    /// Checkout a branch.
    pub fn checkout(&self, branch: &str) -> Result<CommandResult, VcsError> {
        git::checkout(&self.repo_path, branch)
    }

    /// Push to remote.
    pub fn push(&self) -> Result<CommandResult, VcsError> {
        git::push(&self.repo_path)
    }

    /// Pull from remote.
    pub fn pull(&self) -> Result<CommandResult, VcsError> {
        git::pull(&self.repo_path)
    }

    /// Fetch from remote.
    #[allow(dead_code)] // Planned: fetch button in toolbar
    pub fn fetch(&self) -> Result<CommandResult, VcsError> {
        git::fetch(&self.repo_path)
    }

    /// Get diff for a file.
    #[allow(dead_code)] // Planned: diff viewer
    pub fn diff_file(&self, file: &str) -> Result<String, VcsError> {
        git::diff_file(&self.repo_path, file)
    }

    /// Get files changed in a commit.
    #[allow(dead_code)] // Planned: commit detail view
    pub fn commit_files(&self, hash: &str) -> Result<Vec<FileStatus>, VcsError> {
        git::commit_files(&self.repo_path, hash)
    }
}
