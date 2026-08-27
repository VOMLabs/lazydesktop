//! Application path resolution.
//!
//! Provides platform-appropriate default paths for configuration, data,
//! and cache directories. All paths are resolved without touching Qt.

use std::path::PathBuf;

/// Resolve the default configuration directory.
///
/// On Linux/macOS: `$HOME/.config/lazydesktop`
/// On Windows: `%APPDATA%/lazydesktop`
pub fn config_dir() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home).join(".config").join("lazydesktop")
    } else if let Some(appdata) = std::env::var_os("APPDATA") {
        PathBuf::from(appdata).join("lazydesktop")
    } else {
        PathBuf::from(".").join("lazydesktop")
    }
}

/// Resolve the default data directory.
///
/// On Linux/macOS: `$HOME/.local/share/lazydesktop`
/// On Windows: `%LOCALAPPDATA%/lazydesktop`
pub fn data_dir() -> PathBuf {
    if let Some(home) = std::env::var_os("HOME") {
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("lazydesktop")
    } else if let Some(localappdata) = std::env::var_os("LOCALAPPDATA") {
        PathBuf::from(localappdata).join("lazydesktop")
    } else {
        config_dir()
    }
}

/// Resolve the default projects file path.
pub fn projects_path() -> PathBuf {
    config_dir().join("projects.yaml")
}

/// Resolve the default settings file path.
pub fn settings_path() -> PathBuf {
    config_dir().join("lazydesktop.conf")
}

/// Resolve the default themes directory.
pub fn themes_dir() -> PathBuf {
    config_dir().join("themes")
}

/// Resolve the default models directory.
pub fn models_dir() -> PathBuf {
    data_dir().join("models")
}

/// Resolve the default addons directory.
pub fn addons_dir() -> PathBuf {
    config_dir().join("addons")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_dir_is_non_empty() {
        let dir = config_dir();
        assert!(!dir.as_os_str().is_empty());
        assert!(dir.to_string_lossy().contains("lazydesktop"));
    }

    #[test]
    fn data_dir_is_non_empty() {
        let dir = data_dir();
        assert!(!dir.as_os_str().is_empty());
        assert!(dir.to_string_lossy().contains("lazydesktop"));
    }

    #[test]
    fn projects_path_ends_with_yaml() {
        let path = projects_path();
        assert_eq!(path.extension().unwrap(), "yaml");
    }

    #[test]
    fn settings_path_ends_with_conf() {
        let path = settings_path();
        assert_eq!(path.extension().unwrap(), "conf");
    }

    #[test]
    fn themes_dir_name() {
        let dir = themes_dir();
        assert_eq!(dir.file_name().unwrap(), "themes");
    }

    #[test]
    fn models_dir_name() {
        let dir = models_dir();
        assert_eq!(dir.file_name().unwrap(), "models");
    }

    #[test]
    fn addons_dir_name() {
        let dir = addons_dir();
        assert_eq!(dir.file_name().unwrap(), "addons");
    }
}
