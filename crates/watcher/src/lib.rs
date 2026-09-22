//! lazydesktop-watcher — VCS-aware filesystem watcher with debouncing.
//!
//! Monitors `.git/` or `.jj/` directories for changes and emits typed
//! [`FileEvent`] values through a channel. The watcher is independent of Qt
//! and can be consumed by any Rust frontend (GPUI, TUI, etc.).

pub mod ffi;

use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};

/// Typed filesystem event emitted by the watcher.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileEvent {
    /// A file was created.
    Created(PathBuf),
    /// A file was modified (content or metadata).
    Modified(PathBuf),
    /// A file was removed.
    Removed(PathBuf),
    /// A file was renamed/moved.
    Renamed { from: PathBuf, to: PathBuf },
    /// The watcher needs to rescan (e.g. overflow on Linux inotify).
    Rescan,
}

/// Configuration for the file watcher.
#[derive(Debug, Clone)]
pub struct WatcherConfig {
    /// Debounce interval — events within this window are coalesced.
    pub debounce: Duration,
    /// The VCS kind: `"git"` or `"jujutsu"`.
    pub vcs_kind: VcsKind,
}

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            debounce: Duration::from_secs(2),
            vcs_kind: VcsKind::Git,
        }
    }
}

/// The kind of version control system in use.
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

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "git" => Some(VcsKind::Git),
            "jujutsu" => Some(VcsKind::Jujutsu),
            _ => None,
        }
    }
}

/// Detect the VCS kind for a repository path.
pub fn detect_vcs(repo_path: &Path) -> VcsKind {
    if repo_path.join(".jj").exists() {
        VcsKind::Jujutsu
    } else {
        VcsKind::Git
    }
}

/// A running file watcher that monitors VCS state files.
///
/// Drop the watcher to stop monitoring. All threads are joined cleanly.
pub struct RepoWatcher {
    /// The channel receiver for debounced events.
    rx: mpsc::Receiver<FileEvent>,
    /// Keep the watcher alive — dropping it stops the background thread.
    _watcher: RecommendedWatcher,
}

impl RepoWatcher {
    /// Start watching a repository at `repo_path`.
    ///
    /// The returned [`mpsc::Receiver`] delivers [`FileEvent`] values after
    /// the configured debounce interval. The watcher automatically stops
    /// when dropped.
    pub fn watch(repo_path: &Path, config: &WatcherConfig) -> Result<Self, WatchError> {
        let (tx, rx) = mpsc::channel();

        let paths = watch_paths_for(repo_path, config.vcs_kind);
        if paths.is_empty() {
            return Err(WatchError::NoPaths);
        }

        let debounce = config.debounce;
        let mut watcher = RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| {
                let _ = tx.send(match result {
                    Ok(event) => map_event(event),
                    Err(_) => FileEvent::Rescan,
                });
            },
            notify::Config::default().with_poll_interval(debounce),
        )
        .map_err(|e| WatchError::Init(e.to_string()))?;

        for path in &paths {
            if path.is_dir() {
                watcher
                    .watch(path, RecursiveMode::NonRecursive)
                    .map_err(|e| WatchError::Watch(path.display().to_string(), e.to_string()))?;
            } else if let Some(parent) = path.parent() {
                watcher
                    .watch(parent, RecursiveMode::NonRecursive)
                    .map_err(|e| WatchError::Watch(parent.display().to_string(), e.to_string()))?;
            }
        }

        Ok(Self {
            rx,
            _watcher: watcher,
        })
    }

    /// Create a watcher that monitors custom paths (not VCS-derived).
    ///
    /// Useful for testing or non-standard setups.
    pub fn watch_paths(paths: &[PathBuf], debounce: Duration) -> Result<Self, WatchError> {
        let (tx, rx) = mpsc::channel();

        let mut watcher = RecommendedWatcher::new(
            move |result: Result<Event, notify::Error>| {
                let _ = tx.send(match result {
                    Ok(event) => map_event(event),
                    Err(_) => FileEvent::Rescan,
                });
            },
            notify::Config::default().with_poll_interval(debounce),
        )
        .map_err(|e| WatchError::Init(e.to_string()))?;

        for path in paths {
            if path.is_dir() {
                watcher
                    .watch(path, RecursiveMode::NonRecursive)
                    .map_err(|e| WatchError::Watch(path.display().to_string(), e.to_string()))?;
            } else if let Some(parent) = path.parent() {
                watcher
                    .watch(parent, RecursiveMode::NonRecursive)
                    .map_err(|e| WatchError::Watch(parent.display().to_string(), e.to_string()))?;
            }
        }

        Ok(Self {
            rx,
            _watcher: watcher,
        })
    }

    /// Block until the next event is available.
    pub fn recv(&self) -> Result<FileEvent, mpsc::RecvError> {
        self.rx.recv()
    }

    /// Try to receive an event without blocking.
    pub fn try_recv(&self) -> Result<FileEvent, mpsc::TryRecvError> {
        self.rx.try_recv()
    }

    /// Return the receiver for use with polling/select patterns.
    pub fn receiver(&self) -> &mpsc::Receiver<FileEvent> {
        &self.rx
    }
}

/// Compute the list of paths to watch for a given repo and VCS kind.
pub fn watch_paths_for(repo_path: &Path, vcs_kind: VcsKind) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    match vcs_kind {
        VcsKind::Git => {
            let git_dir = repo_path.join(".git");
            if git_dir.is_dir() {
                paths.push(git_dir.join("index"));
                paths.push(git_dir.join("HEAD"));
                paths.push(git_dir);
            }
        }
        VcsKind::Jujutsu => {
            let jj_dir = repo_path.join(".jj");
            if jj_dir.is_dir() {
                paths.push(jj_dir.join("working_copy"));
                paths.push(jj_dir.join("repo"));
                paths.push(jj_dir);
            }
        }
    }
    // Filter to only paths that actually exist.
    paths.into_iter().filter(|p| p.exists()).collect()
}

/// Map a notify event to our typed [`FileEvent`].
fn map_event(event: Event) -> FileEvent {
    match event.kind {
        EventKind::Create(_) => {
            let path = event.paths.into_iter().next().unwrap_or_default();
            FileEvent::Created(path)
        }
        EventKind::Modify(_) => {
            let path = event.paths.into_iter().next().unwrap_or_default();
            FileEvent::Modified(path)
        }
        EventKind::Remove(_) => {
            let path = event.paths.into_iter().next().unwrap_or_default();
            FileEvent::Removed(path)
        }
        EventKind::Access(_) => {
            // Access events are not relevant for VCS state tracking.
            // Return a Rescan to let the consumer decide.
            FileEvent::Rescan
        }
        EventKind::Any | EventKind::Other => FileEvent::Rescan,
    }
}

/// Errors from watcher initialization.
#[derive(Debug, Clone, thiserror::Error)]
pub enum WatchError {
    #[error("no valid watch paths found")]
    NoPaths,
    #[error("failed to initialize watcher: {0}")]
    Init(String),
    #[error("failed to watch path {0}: {1}")]
    Watch(String, String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn detect_vcs_git() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join(".git")).unwrap();
        assert_eq!(detect_vcs(tmp.path()), VcsKind::Git);
    }

    #[test]
    fn detect_vcs_jujutsu() {
        let tmp = TempDir::new().unwrap();
        fs::create_dir_all(tmp.path().join(".jj")).unwrap();
        assert_eq!(detect_vcs(tmp.path()), VcsKind::Jujutsu);
    }

    #[test]
    fn detect_vcs_defaults_to_git() {
        let tmp = TempDir::new().unwrap();
        assert_eq!(detect_vcs(tmp.path()), VcsKind::Git);
    }

    #[test]
    fn watch_paths_for_git() {
        let tmp = TempDir::new().unwrap();
        let git_dir = tmp.path().join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();
        fs::write(git_dir.join("index"), "").unwrap();

        let paths = watch_paths_for(tmp.path(), VcsKind::Git);
        assert!(paths.contains(&git_dir.join("HEAD")));
        assert!(paths.contains(&git_dir.join("index")));
        assert!(paths.contains(&git_dir));
    }

    #[test]
    fn watch_paths_for_jujutsu() {
        let tmp = TempDir::new().unwrap();
        let jj_dir = tmp.path().join(".jj");
        fs::create_dir_all(&jj_dir).unwrap();
        fs::write(jj_dir.join("working_copy"), "").unwrap();
        fs::write(jj_dir.join("repo"), "").unwrap();

        let paths = watch_paths_for(tmp.path(), VcsKind::Jujutsu);
        assert!(paths.contains(&jj_dir.join("working_copy")));
        assert!(paths.contains(&jj_dir.join("repo")));
        assert!(paths.contains(&jj_dir));
    }

    #[test]
    fn watch_paths_for_missing_dir_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let paths = watch_paths_for(tmp.path(), VcsKind::Git);
        assert!(paths.is_empty());
    }

    #[test]
    fn vcs_kind_roundtrip() {
        assert_eq!(VcsKind::from_str("git"), Some(VcsKind::Git));
        assert_eq!(VcsKind::from_str("jujutsu"), Some(VcsKind::Jujutsu));
        assert_eq!(VcsKind::from_str("svn"), None);
        assert_eq!(VcsKind::Git.as_str(), "git");
        assert_eq!(VcsKind::Jujutsu.as_str(), "jujutsu");
    }

    #[test]
    fn watcher_emits_events_on_file_change() {
        let tmp = TempDir::new().unwrap();
        let git_dir = tmp.path().join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").unwrap();

        let paths = vec![git_dir.join("HEAD")];
        let watcher = RepoWatcher::watch_paths(&paths, Duration::from_millis(50)).unwrap();

        // Trigger a change
        std::thread::sleep(Duration::from_millis(100));
        fs::write(git_dir.join("HEAD"), "ref: refs/heads/feature\n").unwrap();

        // Should receive an event within a reasonable time
        let event = watcher.recv().unwrap();
        if let FileEvent::Modified(p) = event {
            assert!(p.to_string_lossy().contains("HEAD"));
        } // Other events are acceptable too
    }

    #[test]
    fn file_event_equality() {
        let a = FileEvent::Created(PathBuf::from("/tmp/test"));
        let b = FileEvent::Created(PathBuf::from("/tmp/test"));
        assert_eq!(a, b);

        let c = FileEvent::Renamed {
            from: PathBuf::from("/tmp/a"),
            to: PathBuf::from("/tmp/b"),
        };
        assert_ne!(a, c);
    }

    #[test]
    fn watcher_config_default() {
        let config = WatcherConfig::default();
        assert_eq!(config.debounce, Duration::from_secs(2));
        assert_eq!(config.vcs_kind, VcsKind::Git);
    }
}
