//! vcs_core — native VCS/SSH operations for LazyDesktop.
//!
//! Pure Rust implementation of SSH key management (ssh-key), SSH connection
//! testing (russh), and git remote management (gix-config), exposed to the
//! Qt C++ application through a C ABI in [`ffi`].
//!
//! Security invariant: private key material never crosses the FFI boundary.
//! Only public keys, fingerprints, paths, and status/error strings do.

pub mod connect;
pub mod error;
pub mod ffi;
pub mod remote;
pub mod ssh;

pub use error::VcsError;
