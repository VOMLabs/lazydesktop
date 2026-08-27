//! Theme management: YAML-based theme definitions.
//!
//! Themes are YAML files in the themes directory with a `.theme.yaml`
//! extension. Each theme defines a set of named colors.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// A color theme for the application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    /// Display name of the theme.
    pub name: String,
    /// Named color values (hex strings like "#FF0000").
    pub colors: HashMap<String, String>,
}

impl Theme {
    /// Load a theme from a YAML file.
    pub fn load(path: &Path) -> Result<Self, ThemeError> {
        let content = fs::read_to_string(path).map_err(|e| ThemeError::Io {
            path: path.display().to_string(),
            source: e,
        })?;

        serde_yaml::from_str(&content).map_err(|e| ThemeError::Parse(e.to_string()))
    }

    /// Save a theme to a YAML file.
    pub fn save(&self, path: &Path) -> Result<(), ThemeError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| ThemeError::Io {
                path: parent.display().to_string(),
                source: e,
            })?;
        }

        let content = serde_yaml::to_string(self).map_err(|e| ThemeError::Parse(e.to_string()))?;

        fs::write(path, content).map_err(|e| ThemeError::Io {
            path: path.display().to_string(),
            source: e,
        })
    }

    /// Get a color value by name.
    pub fn color(&self, name: &str) -> Option<&str> {
        self.colors.get(name).map(|s| s.as_str())
    }

    /// Get all color names.
    pub fn color_names(&self) -> Vec<&str> {
        self.colors.keys().map(|s| s.as_str()).collect()
    }
}

/// Scan a directory for theme files.
pub fn scan_themes(dir: &Path) -> Vec<ThemeEntry> {
    let mut entries = Vec::new();

    if !dir.is_dir() {
        return entries;
    }

    if let Ok(read_dir) = fs::read_dir(dir) {
        for entry in read_dir.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("yaml")
                && path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.ends_with(".theme"))
                    .unwrap_or(false)
            {
                if let Ok(theme) = Theme::load(&path) {
                    entries.push(ThemeEntry {
                        name: theme.name.clone(),
                        path,
                    });
                }
            }
        }
    }

    entries.sort_by(|a, b| a.name.cmp(&b.name));
    entries
}

/// A discovered theme entry.
#[derive(Debug, Clone)]
pub struct ThemeEntry {
    pub name: String,
    pub path: PathBuf,
}

/// The built-in "Dark" theme with VS Code-style colors.
pub fn builtin_dark_theme() -> Theme {
    let mut colors = HashMap::new();
    colors.insert("background".to_string(), "#1e1e1e".to_string());
    colors.insert("foreground".to_string(), "#d4d4d4".to_string());
    colors.insert("widget_background".to_string(), "#252526".to_string());
    colors.insert("input_background".to_string(), "#3c3c3c".to_string());
    colors.insert("input_foreground".to_string(), "#d4d4d4".to_string());
    colors.insert("button_background".to_string(), "#0e639c".to_string());
    colors.insert("button_foreground".to_string(), "#ffffff".to_string());
    colors.insert("tooltip_background".to_string(), "#252526".to_string());
    colors.insert("tooltip_foreground".to_string(), "#cccccc".to_string());
    colors.insert("selection".to_string(), "#264f78".to_string());

    Theme {
        name: "Dark".to_string(),
        colors,
    }
}

/// Errors from theme operations.
#[derive(Debug, thiserror::Error)]
pub enum ThemeError {
    #[error("io error at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("parse error: {0}")]
    Parse(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn builtin_dark_theme_has_required_colors() {
        let theme = builtin_dark_theme();
        assert_eq!(theme.name, "Dark");
        assert!(theme.color("background").is_some());
        assert!(theme.color("foreground").is_some());
        assert!(theme.color("selection").is_some());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.theme.yaml");

        let mut colors = HashMap::new();
        colors.insert("background".to_string(), "#000000".to_string());
        let theme = Theme {
            name: "Test".to_string(),
            colors,
        };
        theme.save(&path).unwrap();

        let loaded = Theme::load(&path).unwrap();
        assert_eq!(loaded.name, "Test");
        assert_eq!(loaded.color("background"), Some("#000000"));
    }

    #[test]
    fn scan_themes_empty_dir() {
        let tmp = TempDir::new().unwrap();
        let entries = scan_themes(tmp.path());
        assert!(entries.is_empty());
    }

    #[test]
    fn scan_themes_finds_valid_themes() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("My Theme.theme.yaml");

        let theme = builtin_dark_theme();
        theme.save(&path).unwrap();

        let entries = scan_themes(tmp.path());
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Dark");
    }

    #[test]
    fn scan_themes_ignores_non_theme_files() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("not-a-theme.yaml"), "name: test\n").unwrap();
        fs::write(tmp.path().join("also-not.theme.txt"), "name: test\n").unwrap();

        let entries = scan_themes(tmp.path());
        assert!(entries.is_empty());
    }

    #[test]
    fn theme_color_lookup() {
        let mut colors = HashMap::new();
        colors.insert("bg".to_string(), "#111".to_string());
        let theme = Theme {
            name: "T".to_string(),
            colors,
        };
        assert_eq!(theme.color("bg"), Some("#111"));
        assert_eq!(theme.color("missing"), None);
    }
}
