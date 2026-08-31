//! INI-format settings (compatible with QSettings).
//!
//! Settings are stored as key-value pairs with dotted hierarchical keys.
//! The format is a simple INI with `[section]` headers and `key = value`
//! lines. QSettings uses `=` as the separator.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// A settings store backed by an INI file.
#[derive(Debug, Clone, Default)]
pub struct Settings {
    /// Hierarchical key-value pairs. Keys use `/` as separator internally
    /// (matching QSettings convention), stored as `section/key` in the file.
    values: HashMap<String, String>,
    /// Path to the settings file on disk.
    path: Option<PathBuf>,
}

impl Settings {
    /// Create a new empty settings store.
    pub fn new() -> Self {
        Self::default()
    }

    /// Load settings from a file. Missing file is treated as empty.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let mut settings = Self {
            values: HashMap::new(),
            path: Some(path.to_path_buf()),
        };

        if !path.exists() {
            return Ok(settings);
        }

        let content = fs::read_to_string(path).map_err(|e| ConfigError::Io {
            path: path.display().to_string(),
            source: e,
        })?;

        settings.parse_ini(&content);
        Ok(settings)
    }

    /// Parse INI content into key-value pairs.
    ///
    /// Format:
    /// ```ini
    /// [section]
    /// key = value
    /// ```
    ///
    /// Keys are stored as `section/key` in the internal map.
    fn parse_ini(&mut self, content: &str) {
        let mut current_section = String::new();

        for line in content.lines() {
            let trimmed = line.trim();

            // Skip empty lines and comments
            if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.starts_with('#') {
                continue;
            }

            // Section header
            if trimmed.starts_with('[') && trimmed.ends_with(']') {
                current_section = trimmed[1..trimmed.len() - 1].trim().to_string();
                continue;
            }

            // Key-value pair
            if let Some(pos) = trimmed.find('=') {
                let key = trimmed[..pos].trim().to_string();
                let value = trimmed[pos + 1..].trim().to_string();

                let full_key = if current_section.is_empty() {
                    key
                } else {
                    format!("{}/{}", current_section, key)
                };

                self.values.insert(full_key, value);
            }
        }
    }

    /// Save settings to the file.
    pub fn save(&self) -> Result<(), ConfigError> {
        let path = self.path.as_ref().ok_or(ConfigError::NoPath)?;
        self.save_to(path)
    }

    /// Save settings to a specific path.
    pub fn save_to(&self, path: &Path) -> Result<(), ConfigError> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| ConfigError::Io {
                path: parent.display().to_string(),
                source: e,
            })?;
        }

        let content = self.to_ini();
        fs::write(path, content).map_err(|e| ConfigError::Io {
            path: path.display().to_string(),
            source: e,
        })
    }

    /// Convert settings to INI format.
    fn to_ini(&self) -> String {
        let mut sections: HashMap<String, Vec<(&str, &str)>> = HashMap::new();

        for (key, value) in &self.values {
            if let Some(pos) = key.find('/') {
                let section = &key[..pos];
                let subkey = &key[pos + 1..];
                sections
                    .entry(section.to_string())
                    .or_default()
                    .push((subkey, value));
            } else {
                sections
                    .entry(String::new())
                    .or_default()
                    .push((key, value));
            }
        }

        let mut output = String::new();

        // Global keys first (no section)
        if let Some(global) = sections.get("") {
            for (key, value) in global {
                output.push_str(&format!("{} = {}\n", key, value));
            }
            output.push('\n');
        }

        // Sorted sections
        let mut section_names: Vec<&str> = sections
            .keys()
            .filter(|s| !s.is_empty())
            .map(|s| s.as_str())
            .collect();
        section_names.sort();

        for section in section_names {
            output.push_str(&format!("[{}]\n", section));
            if let Some(entries) = sections.get(section) {
                for (key, value) in entries {
                    output.push_str(&format!("{} = {}\n", key, value));
                }
            }
            output.push('\n');
        }

        output
    }

    /// Get a setting value by key.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(|s| s.as_str())
    }

    /// Get a setting value with a default.
    pub fn get_or<'a>(&'a self, key: &str, default: &'a str) -> &'a str {
        self.get(key).unwrap_or(default)
    }

    /// Get a setting value parsed as a boolean.
    pub fn get_bool(&self, key: &str) -> bool {
        match self.get(key) {
            Some(v) => matches!(v, "true" | "1" | "yes"),
            None => false,
        }
    }

    /// Get a setting value parsed as an integer.
    pub fn get_int(&self, key: &str) -> Option<i64> {
        self.get(key)?.parse().ok()
    }

    /// Set a setting value.
    pub fn set(&mut self, key: &str, value: &str) {
        self.values.insert(key.to_string(), value.to_string());
    }

    /// Set a boolean value.
    pub fn set_bool(&mut self, key: &str, value: bool) {
        self.set(key, if value { "true" } else { "false" });
    }

    /// Remove a setting.
    pub fn remove(&mut self, key: &str) {
        self.values.remove(key);
    }

    /// Check if a key exists.
    pub fn contains(&self, key: &str) -> bool {
        self.values.contains_key(key)
    }

    /// Get all keys matching a prefix.
    pub fn keys_with_prefix(&self, prefix: &str) -> Vec<&str> {
        self.values
            .keys()
            .filter(|k| k.starts_with(prefix))
            .map(|k| k.as_str())
            .collect()
    }

    /// Get the number of settings.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Check if settings are empty.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }
}

/// Errors from configuration operations.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("io error reading {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("no path set for settings")]
    NoPath,
    #[error("invalid TOML: {0}")]
    InvalidToml(String),
    #[error("invalid YAML: {0}")]
    InvalidYaml(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn parse_ini_basic() {
        let mut settings = Settings::new();
        settings.parse_ini(
            "[git]\nuser.name = Alice\nuser.email = alice@example.com\n\n[ai]\nenabled = true\n",
        );
        assert_eq!(settings.get("git/user.name"), Some("Alice"));
        assert_eq!(settings.get("git/user.email"), Some("alice@example.com"));
        assert_eq!(settings.get("ai/enabled"), Some("true"));
    }

    #[test]
    fn parse_ini_comments() {
        let mut settings = Settings::new();
        settings.parse_ini("; comment\n# another comment\nkey = value\n");
        assert_eq!(settings.get("key"), Some("value"));
    }

    #[test]
    fn roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("test.conf");

        let mut original = Settings::new();
        original.set("git/user.name", "Bob");
        original.set("ai/enabled", "false");
        original.save_to(&path).unwrap();

        let loaded = Settings::load(&path).unwrap();
        assert_eq!(loaded.get("git/user.name"), Some("Bob"));
        assert_eq!(loaded.get("ai/enabled"), Some("false"));
    }

    #[test]
    fn get_bool_values() {
        let mut settings = Settings::new();
        settings.set("a", "true");
        settings.set("b", "false");
        settings.set("c", "1");
        settings.set("d", "0");
        settings.set("e", "yes");
        settings.set("f", "no");

        assert!(settings.get_bool("a"));
        assert!(!settings.get_bool("b"));
        assert!(settings.get_bool("c"));
        assert!(!settings.get_bool("d"));
        assert!(settings.get_bool("e"));
        assert!(!settings.get_bool("f"));
        assert!(!settings.get_bool("missing"));
    }

    #[test]
    fn get_int_values() {
        let mut settings = Settings::new();
        settings.set("port", "8080");
        settings.set("notnum", "abc");

        assert_eq!(settings.get_int("port"), Some(8080));
        assert_eq!(settings.get_int("notnum"), None);
        assert_eq!(settings.get_int("missing"), None);
    }

    #[test]
    fn remove_key() {
        let mut settings = Settings::new();
        settings.set("key", "value");
        assert!(settings.contains("key"));
        settings.remove("key");
        assert!(!settings.contains("key"));
    }

    #[test]
    fn keys_with_prefix() {
        let mut settings = Settings::new();
        settings.set("ai/enabled", "true");
        settings.set("ai/provider", "openai");
        settings.set("git/user.name", "Alice");

        let ai_keys = settings.keys_with_prefix("ai/");
        assert_eq!(ai_keys.len(), 2);
        assert!(ai_keys.contains(&"ai/enabled"));
        assert!(ai_keys.contains(&"ai/provider"));
    }

    #[test]
    fn len_and_is_empty() {
        let mut settings = Settings::new();
        assert!(settings.is_empty());
        settings.set("a", "1");
        assert_eq!(settings.len(), 1);
        assert!(!settings.is_empty());
    }

    #[test]
    fn load_missing_file_returns_empty() {
        let settings = Settings::load(Path::new("/nonexistent/path/settings.conf")).unwrap();
        assert!(settings.is_empty());
    }

    #[test]
    fn get_or_returns_default() {
        let settings = Settings::new();
        assert_eq!(settings.get_or("missing", "default"), "default");
    }
}
