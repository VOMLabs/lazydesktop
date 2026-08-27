//! Process-wide per-repository state cache.
//!
//! The FFI layer cannot return borrowed state across the C boundary, and every
//! `jj` subprocess round-trip is comparatively expensive. This module keeps a
//! small, concurrency-safe cache of per-repo state so C++ callers can answer
//! cheap questions (e.g. "what is the current working-copy commit?") without
//! spawning `jj`.
//!
//! Concurrency notes:
//! - [`DashMap`] gives concurrent reads and sharded writes, so the cache is
//!   safe to touch from any thread (including several FFI calls at once).
//! - Guards (dashmap `Ref`s) are always dropped before returning; callers get
//!   an owned clone, so no lock or shard guard is ever held across an `.await`.
//! - The cache is process-global via [`repo_state_cache`] so all FFI entry
//!   points share one instance.

use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;

use dashmap::DashMap;

/// Cached state for a single repository.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RepoState {
    /// Full 40-hex commit id of the working-copy commit as of the last
    /// [`crate::jj::jj_log`] call; `None` when the repo has not been logged.
    pub working_copy_commit_id: Option<String>,
}

/// Concurrent cache keyed by repository path.
#[derive(Debug, Default)]
pub struct RepoStateCache {
    inner: DashMap<PathBuf, RepoState>,
}

impl RepoStateCache {
    /// Create an empty cache.
    pub fn new() -> Self {
        Self::default()
    }

    /// Return an owned clone of the state for `repo_path` (defaults to empty).
    ///
    /// The dashmap shard guard is released before the value is returned, so
    /// the caller never holds a lock across an `.await`.
    pub fn get(&self, repo_path: &Path) -> RepoState {
        self.inner
            .get(repo_path)
            .map(|state| state.clone())
            .unwrap_or_default()
    }

    /// Replace the state for `repo_path`.
    pub fn set(&self, repo_path: &Path, state: RepoState) {
        self.inner.insert(repo_path.to_path_buf(), state);
    }

    /// Update the state for `repo_path` in place (inserting a default first).
    ///
    /// Uses the dashmap `entry` API; the `RefMut` guard is dropped when the
    /// closure returns, before this function returns.
    pub fn update(&self, repo_path: &Path, f: impl FnOnce(&mut RepoState)) {
        let mut state = self
            .inner
            .entry(repo_path.to_path_buf())
            .or_insert_with(RepoState::default);
        f(&mut state);
    }

    /// Remove the state for `repo_path` (e.g. when a repo is unloaded).
    pub fn remove(&self, repo_path: &Path) {
        self.inner.remove(repo_path);
    }
}

/// The process-global repo state cache.
pub fn repo_state_cache() -> &'static RepoStateCache {
    static CACHE: OnceLock<RepoStateCache> = OnceLock::new();
    CACHE.get_or_init(RepoStateCache::new)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_returns_default_for_unknown_repo() {
        let cache = RepoStateCache::new();
        assert_eq!(cache.get(Path::new("/no/such/repo")), RepoState::default());
    }

    #[test]
    fn set_and_get_round_trip() {
        let cache = RepoStateCache::new();
        let path = Path::new("/tmp/repo-a");
        let state = RepoState {
            working_copy_commit_id: Some("0123456789abcdef0123456789abcdef01234567".to_string()),
        };
        cache.set(path, state.clone());
        assert_eq!(cache.get(path), state);
        // Other paths are unaffected.
        assert_eq!(cache.get(Path::new("/tmp/repo-b")), RepoState::default());
    }

    #[test]
    fn update_uses_entry_api() {
        let cache = RepoStateCache::new();
        let path = Path::new("/tmp/repo-a");
        cache.update(path, |state| {
            state.working_copy_commit_id = Some("a".repeat(40));
        });
        assert_eq!(
            cache.get(path).working_copy_commit_id,
            Some("a".repeat(40))
        );
        cache.update(path, |state| {
            state.working_copy_commit_id = Some("b".repeat(40));
        });
        assert_eq!(
            cache.get(path).working_copy_commit_id,
            Some("b".repeat(40))
        );
    }

    #[test]
    fn concurrent_updates_do_not_lose_entries() {
        let cache = std::sync::Arc::new(RepoStateCache::new());
        let paths: Vec<PathBuf> = (0..32).map(|i| PathBuf::from(format!("/tmp/repo-{i}"))).collect();
        let handles: Vec<_> = paths
            .iter()
            .map(|path| {
                let path = path.clone();
                let cache = std::sync::Arc::clone(&cache);
                std::thread::spawn(move || {
                    for _ in 0..100 {
                        cache.update(&path, |state| {
                            state.working_copy_commit_id = Some(path.to_string_lossy().into_owned());
                        });
                    }
                })
            })
            .collect();
        for handle in handles {
            handle.join().expect("worker thread must finish");
        }
        for path in &paths {
            assert_eq!(
                cache.get(path).working_copy_commit_id.as_deref(),
                Some(path.to_str().expect("utf8 path")),
                "entry for {path:?} was lost"
            );
        }
    }

    #[test]
    fn remove_clears_entry() {
        let cache = RepoStateCache::new();
        let path = Path::new("/tmp/repo-a");
        cache.set(path, RepoState::default());
        cache.remove(path);
        assert_eq!(cache.get(path), RepoState::default());
    }
}
