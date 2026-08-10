//! lazydesktop-addons — LazyDesktop addon system security core.
//!
//! Rust is the trust boundary: archive parsing, directory loading, ignore
//! rules, manifest validation, path security, resource access, and the C ABI
//! all live here and are enforced strictly. C++ only consumes validated data.
//!
//! `unsafe` is denied except inside [`ffi`]; the FFI module carries
//! `#![allow(unsafe_code)]` and contains every `unsafe` block in the crate.

#![deny(unsafe_code)]

pub mod archive;
pub mod error;
pub mod ignore;
pub mod json;
pub mod manifest;
pub mod package;
pub mod pathsec;
pub mod provider;
pub mod registry;
pub(crate) mod runtime;

mod ffi;

/// Re-export of the C ABI for external consumers (e.g. tests).
pub use ffi::*;

/// Emit a log message through the FFI log sink (if one was installed).
///
/// Retained for Rust-internal logging and FFI tests; the primary path today is
/// the host installing a sink via `lda_set_log_sink`.
#[allow(dead_code)]
pub(crate) fn emit_log(level: u8, msg: &str) {
    ffi::emit_log_impl(level, msg);
}
