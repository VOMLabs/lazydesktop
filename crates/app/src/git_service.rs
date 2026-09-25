//! Async wrapper around git_cmd for GPUI integration.
//!
//! Provides spawn-based async execution so git operations don't block the UI.

use std::path::{Path, PathBuf};

use git_cmd::{git, types::*};

/// Shared state for git operations on a repository.
pub struct GitService {
    pub repo_path: PathBuf,
    pub file_statuses: Vec<FileStatus>,
    pub branches: Vec<BranchEntry>,
    pub current_branch: String,
    pub commit_history: Vec<CommitEntry>,
    pub is_dirty: bool,
    /// Number of stashes, cached during `refresh_all`.
    pub stash_count: usize,
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
            stash_count: 0,
        }
    }

    /// Refresh all git state (status, branches, log).
    pub fn refresh_all(&mut self) {
        self.file_statuses = git::status(&self.repo_path).unwrap_or_default();
        self.branches = git::list_branches(&self.repo_path).unwrap_or_default();
        self.current_branch = git::current_branch(&self.repo_path).unwrap_or_default();
        self.commit_history = git::log(&self.repo_path, 50).unwrap_or_default();
        self.is_dirty = git::is_dirty(&self.repo_path);
        self.stash_count = git::stash_list(&self.repo_path).len();
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
    #[allow(dead_code)] // Convenience wrapper; UI uses `commit_with_hooks`.
    pub fn commit(&self, message: &str) -> Result<CommandResult, VcsError> {
        git::commit(&self.repo_path, message)
    }

    /// Commit with message, honoring the skip-hooks toggle.
    pub fn commit_with_hooks(
        &self,
        message: &str,
        verify: bool,
    ) -> Result<CommandResult, VcsError> {
        if verify {
            git::commit(&self.repo_path, message)
        } else {
            git::run_git(&self.repo_path, &["commit", "--no-verify", "-m", message])
        }
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
    pub fn diff_file(&self, file: &str) -> Result<String, VcsError> {
        git::diff_file(&self.repo_path, file)
    }

    /// Get the full diff introduced by a commit.
    pub fn show_commit(&self, hash: &str) -> Result<String, VcsError> {
        git::show_commit(&self.repo_path, hash)
    }

    /// Raw bytes of a file at a revision (`git show <hash>:<path>`) — used to
    /// materialize image files for inline rendering in the diff viewer.
    pub fn file_bytes_at(&self, hash: &str, path: &str) -> Result<Vec<u8>, VcsError> {
        git::show_file_bytes(&self.repo_path, hash, path)
    }

    /// Get files changed in a commit.
    #[allow(dead_code)] // Planned: affected-files list in commit detail
    pub fn commit_files(&self, hash: &str) -> Result<Vec<FileStatus>, VcsError> {
        git::commit_files(&self.repo_path, hash)
    }

    /// Create a new branch and switch to it.
    pub fn create_branch(&self, name: &str) -> Result<CommandResult, VcsError> {
        git::create_branch(&self.repo_path, name)
    }

    /// Delete a branch.
    pub fn delete_branch(&self, name: &str) -> Result<CommandResult, VcsError> {
        git::delete_branch(&self.repo_path, name)
    }

    /// Rename the current branch.
    pub fn rename_branch(&self, name: &str) -> Result<CommandResult, VcsError> {
        git::rename_branch(&self.repo_path, name)
    }

    /// Stash all working-tree changes.
    pub fn stash_push(&self) -> Result<CommandResult, VcsError> {
        git::stash_push(&self.repo_path)
    }

    /// List stashes.
    #[allow(dead_code)] // CLI parity; UI shows the cached `stash_count`.
    pub fn stash_list(&self) -> Vec<String> {
        git::stash_list(&self.repo_path)
    }

    /// Restore the most recent stash.
    pub fn stash_pop(&self) -> Result<CommandResult, VcsError> {
        git::stash_pop(&self.repo_path)
    }

    /// Reset the index to HEAD (keeps working-tree changes).
    pub fn reset_mixed(&self) -> Result<CommandResult, VcsError> {
        git::reset_mixed(&self.repo_path)
    }

    /// Recent commit subjects for AI style reference.
    pub fn recent_subjects(&self, limit: usize) -> Vec<String> {
        git::recent_subjects(&self.repo_path, limit)
    }

    /// Recent commit authors as `Name <email>` lines for co-author suggestions.
    pub fn recent_authors(&self, limit: usize) -> Vec<String> {
        git::recent_authors(&self.repo_path, limit)
    }

    /// Clone a repository into `dest`.
    pub fn clone_repo(&self, url: &str, dest: &Path) -> Result<CommandResult, VcsError> {
        git::clone(url, dest)
    }

    /// Initialize a git repository at `path`.
    pub fn init_repo(&self, path: &Path) -> Result<CommandResult, VcsError> {
        git::init(path)
    }

    /// Whether the repository is jujutsu-backed (used for AI context).
    pub fn vcs_kind(&self) -> &'static str {
        if self.repo_path.join(".jj").is_dir() && !git::is_git_repo(&self.repo_path) {
            "jujutsu"
        } else {
            "git"
        }
    }

    /// Build the diff used for AI commit-message generation: per changed file,
    /// `git diff HEAD -- <file>` with a `--- <file>` header (mirrors Qt).
    pub fn ai_diff_text(&self) -> String {
        let mut parts = Vec::new();
        for file in &self.file_statuses {
            if let Ok(d) = git::diff_file(&self.repo_path, &file.path) {
                if !d.trim().is_empty() {
                    parts.push(format!("--- {}\n{}", file.path, d.trim_end()));
                }
            }
        }
        parts.join("\n\n")
    }
}
