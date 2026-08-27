//! lazydesktop-config — Configuration management for LazyDesktop.
//!
//! Provides typed access to:
//! - Settings (INI format, compatible with QSettings)
//! - Projects (YAML format)
//! - Themes (YAML format)
//! - Application paths
//!
//! The crate is independent of Qt and can be consumed by any frontend.

pub mod ffi;
pub mod paths;
pub mod projects;
pub mod settings;
pub mod themes;
