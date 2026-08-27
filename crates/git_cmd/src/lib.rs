//! lazydesktop-git-cmd — Git/Jujutsu CLI command execution.
//!
//! Provides a typed, synchronous API for running git and jj commands
//! via subprocess execution. The crate is independent of Qt and can
//! be consumed by any frontend.

pub mod ffi;
pub mod git;
pub mod jj;
pub mod types;
