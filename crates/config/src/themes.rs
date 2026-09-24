//! Theme management: Lua-based theme definitions.
//!
//! Themes are Lua files in the themes directory with a `.theme.lua`
//! extension. Each theme defines a set of named colors.
//!
//! Example (`my-theme.theme.lua`):
//! ```lua
//! return {
//!   name = "Dark",
//!   colors = {
//!     background = "#1e1e1e",
//!     foreground = "#d4d4d4",
//!     selection = "#264f78",
//!   }
//! }
//! ```

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use mlua::Lua;

/// A color theme for the application.
#[derive(Debug, Clone)]
pub struct Theme {
    /// Display name of the theme.
    pub name: String,
    /// Named color values (hex strings like "#FF0000").
    pub colors: HashMap<String, String>,
}

impl Theme {
    /// Load a theme from a Lua file.
    pub fn load(path: &Path) -> Result<Self, ThemeError> {
        let content = fs::read_to_string(path).map_err(|e| ThemeError::Io {
            path: path.display().to_string(),
            source: e,
        })?;

        Self::parse_lua(&content).map_err(|e| ThemeError::Parse(e.to_string()))
    }

    /// Parse a Lua string into a Theme.
    fn parse_lua(lua_src: &str) -> Result<Self, mlua::Error> {
        let lua = Lua::new();
        let table: mlua::Table = lua.load(lua_src).eval()?;

        let name = table.get::<String>("name")?;
        let colors_table = table.get::<mlua::Table>("colors")?;

        let mut colors = HashMap::new();
        for pair in colors_table.pairs::<String, String>() {
            let (key, value) = pair?;
            colors.insert(key, value);
        }

        Ok(Self { name, colors })
    }

    /// Save a theme to a Lua file.
    pub fn save(&self, path: &Path) -> Result<(), ThemeError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| ThemeError::Io {
                path: parent.display().to_string(),
                source: e,
            })?;
        }

        let content = self.to_lua();

        fs::write(path, content).map_err(|e| ThemeError::Io {
            path: path.display().to_string(),
            source: e,
        })
    }

    /// Serialize to a Lua return statement.
    fn to_lua(&self) -> String {
        let entries: Vec<String> = self
            .colors
            .iter()
            .map(|(k, v)| format!("  {} = \"{}\"", k, v))
            .collect();

        format!(
            "return {{\n  name = \"{}\",\n  colors = {{\n{}\n  }}\n}}\n",
            self.name.replace('\\', "\\\\").replace('"', "\\\""),
            entries.join(",\n")
        )
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
            if path.extension().and_then(|e| e.to_str()) == Some("lua")
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

/// The built-in "Dark" theme with the application's shade palette.
pub fn builtin_dark_theme() -> Theme {
    let mut colors = HashMap::new();
    colors.insert("background".to_string(), "#18181b".to_string());
    colors.insert("foreground".to_string(), "#ededef".to_string());
    colors.insert("widget_background".to_string(), "#121215".to_string());
    colors.insert("input_background".to_string(), "#27272a".to_string());
    colors.insert("input_foreground".to_string(), "#ededef".to_string());
    colors.insert("button_background".to_string(), "#16a34a".to_string());
    colors.insert("button_foreground".to_string(), "#ffffff".to_string());
    colors.insert("tooltip_background".to_string(), "#27272a".to_string());
    colors.insert("tooltip_foreground".to_string(), "#ededef".to_string());
    colors.insert("selection".to_string(), "#16a34a".to_string());

    Theme {
        name: "Dark".to_string(),
        colors,
    }
}

/// The built-in "Light" theme with the application's shade palette.
pub fn builtin_light_theme() -> Theme {
    let mut colors = HashMap::new();
    colors.insert("background".to_string(), "#f6f7f9".to_string());
    colors.insert("foreground".to_string(), "#1c1f26".to_string());
    colors.insert("widget_background".to_string(), "#eef0f3".to_string());
    colors.insert("input_background".to_string(), "#ffffff".to_string());
    colors.insert("input_foreground".to_string(), "#1c1f26".to_string());
    colors.insert("button_background".to_string(), "#15803d".to_string());
    colors.insert("button_foreground".to_string(), "#ffffff".to_string());
    colors.insert("tooltip_background".to_string(), "#ffffff".to_string());
    colors.insert("tooltip_foreground".to_string(), "#1c1f26".to_string());
    colors.insert("selection".to_string(), "#15803d".to_string());

    Theme {
        name: "Light".to_string(),
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
    #[error("lua parse error: {0}")]
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
    fn builtin_light_theme_has_required_colors() {
        let theme = builtin_light_theme();
        assert_eq!(theme.name, "Light");
        assert!(theme.color("background").is_some());
        assert!(theme.color("foreground").is_some());
        assert!(theme.color("selection").is_some());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.theme.lua");

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
    fn save_produces_valid_lua() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.theme.lua");

        let theme = builtin_dark_theme();
        theme.save(&path).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.starts_with("return {"));
        assert!(content.contains("name = \"Dark\""));
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
        let path = tmp.path().join("My Theme.theme.lua");

        let theme = builtin_dark_theme();
        theme.save(&path).unwrap();

        let entries = scan_themes(tmp.path());
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Dark");
    }

    #[test]
    fn scan_themes_ignores_non_theme_files() {
        let tmp = TempDir::new().unwrap();
        fs::write(
            tmp.path().join("not-a-theme.lua"),
            "return { name = \"test\", colors = {} }\n",
        )
        .unwrap();
        fs::write(
            tmp.path().join("also-not.theme.txt"),
            "return { name = \"test\", colors = {} }\n",
        )
        .unwrap();

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
