//! LazyDesktop GPUI frontend — library target.
//!
//! Logic that must be unit-testable lives here, separate from the binary
//! (`main.rs`) whose GPUI element types overflow the compiler stack when
//! `#[test]` expands over them. The binary links this crate and re-exports
//! nothing — see `crates/app/src/diff.rs` for the first module.

pub mod diff;
