//! LazyDesktop GPUI frontend — library target.
//!
//! Logic that must be unit-testable lives here, separate from the binary
//! (`main.rs`) whose GPUI element types overflow the compiler stack when
//! `#[test]` expands over them. The binary links this crate and imports from
//! it; see `crates/app/src/diff.rs` and `crates/app/src/color.rs`.

pub mod color;
pub mod diff;
pub mod theme;
