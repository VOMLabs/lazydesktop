//! Shared error type for vcs_core.

/// Errors surfaced by the vcs_core modules. `Display` implementations produce
/// human-readable messages suitable for the Qt UI.
#[derive(Debug, thiserror::Error)]
pub enum VcsError {
    #[error("{0}")]
    Io(#[from] std::io::Error),

    #[error("{0}")]
    Ssh(#[from] ssh_key::Error),

    #[error("config: {0}")]
    Config(String),

    #[error("invalid input: {0}")]
    Invalid(String),

    /// A spawned subprocess exited with a non-zero status. `stderr` carries
    /// the captured (trimmed) diagnostics from the failed invocation.
    #[error("{command} failed ({status}): {stderr}")]
    Subprocess {
        /// Executable that was run (e.g. `jj`).
        command: String,
        /// How it failed, e.g. `exit code 1` or `terminated by signal`.
        status: String,
        /// Captured stderr from the failed invocation.
        stderr: String,
    },

    /// A bounded operation exceeded its time limit.
    #[error("{0}")]
    Timeout(String),

    #[error("{0}")]
    Other(String),
}

/// Convenience alias used by FFI helpers.
pub type VcsResult<T> = Result<T, VcsError>;
