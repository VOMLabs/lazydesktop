//! Project management: recent projects list stored as Lua.
//!
//! The projects file is a Lua script returning a table with a `projects` key
//! containing an array of absolute paths.
//!
//! Example (`projects.lua`):
//! ```lua
//! return {
//!   projects = {
//!     "/home/user/project1",
//!     "/home/user/project2",
//!   }
//! }
//! ```

use std::fs;
use std::path::Path;

use mlua::Lua;

/// Maximum number of recent projects to retain.
pub const MAX_RECENT_PROJECTS: usize = 100;

/// A persisted list of recent project paths.
#[derive(Debug, Clone, Default)]
pub struct ProjectsFile {
    pub projects: Vec<String>,
}

impl ProjectsFile {
    /// Load projects from a Lua file. Missing or corrupt files return an
    /// empty list.
    pub fn load(path: &Path) -> Self {
        if !path.exists() {
            return Self::default();
        }

        let content = match fs::read_to_string(path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };

        Self::parse_lua(&content).unwrap_or_default()
    }

    /// Parse a Lua string into a ProjectsFile.
    fn parse_lua(lua_src: &str) -> Result<Self, mlua::Error> {
        let lua = Lua::new();
        let table: mlua::Table = lua.load(lua_src).eval()?;

        let projects = table
            .get::<mlua::Table>("projects")?
            .sequence_values::<String>()
            .filter_map(Result::ok)
            .collect();

        Ok(Self { projects })
    }

    /// Save projects to a Lua file.
    pub fn save(&self, path: &Path) -> Result<(), ProjectError> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| ProjectError::Io {
                path: parent.display().to_string(),
                source: e,
            })?;
        }

        let content = self.to_lua();

        fs::write(path, content).map_err(|e| ProjectError::Io {
            path: path.display().to_string(),
            source: e,
        })
    }

    /// Serialize to a Lua return statement.
    fn to_lua(&self) -> String {
        let entries: Vec<String> = self
            .projects
            .iter()
            .map(|p| format!("  \"{}\"", p.replace('\\', "\\\\").replace('"', "\\\"")))
            .collect();

        format!(
            "return {{\n  projects = {{\n{}\n  }}\n}}\n",
            entries.join(",\n")
        )
    }
}

/// A managed recent projects list with add/remove/clear operations.
#[derive(Debug, Clone)]
pub struct RecentProjects {
    paths: Vec<String>,
}

impl RecentProjects {
    /// Create a new empty list.
    pub fn new() -> Self {
        Self { paths: Vec::new() }
    }

    /// Load from a Lua file.
    pub fn load(path: &Path) -> Self {
        Self {
            paths: ProjectsFile::load(path).projects,
        }
    }

    /// Save to a Lua file.
    pub fn save(&self, path: &Path) -> Result<(), ProjectError> {
        let file = ProjectsFile {
            projects: self.paths.clone(),
        };
        file.save(path)
    }

    /// Add a project path, moving it to the front and deduplicating.
    pub fn add(&mut self, path: &str) {
        self.paths.retain(|p| p != path);
        self.paths.insert(0, path.to_string());
        self.truncate(MAX_RECENT_PROJECTS);
    }

    /// Remove a project path.
    pub fn remove(&mut self, path: &str) {
        self.paths.retain(|p| p != path);
    }

    /// Clear all projects.
    pub fn clear(&mut self) {
        self.paths.clear();
    }

    /// Get all project paths.
    pub fn paths(&self) -> &[String] {
        &self.paths
    }

    /// Get the number of projects.
    pub fn len(&self) -> usize {
        self.paths.len()
    }

    /// Check if the list is empty.
    pub fn is_empty(&self) -> bool {
        self.paths.is_empty()
    }

    /// Truncate to the given maximum length.
    fn truncate(&mut self, max: usize) {
        self.paths.truncate(max);
    }
}

impl Default for RecentProjects {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors from project operations.
#[derive(Debug, thiserror::Error)]
pub enum ProjectError {
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
    fn load_missing_file() {
        let projects = RecentProjects::load(Path::new("/nonexistent/projects.lua"));
        assert!(projects.is_empty());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("projects.lua");

        let mut projects = RecentProjects::new();
        projects.add("/home/user/project1");
        projects.add("/home/user/project2");
        projects.save(&path).unwrap();

        let loaded = RecentProjects::load(&path);
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.paths()[0], "/home/user/project2");
        assert_eq!(loaded.paths()[1], "/home/user/project1");
    }

    #[test]
    fn save_produces_valid_lua() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("projects.lua");

        let mut projects = RecentProjects::new();
        projects.add("/a/path");
        projects.save(&path).unwrap();

        let content = fs::read_to_string(&path).unwrap();
        assert!(content.starts_with("return {"));
        assert!(content.contains("\"/a/path\""));
    }

    #[test]
    fn add_deduplicates() {
        let mut projects = RecentProjects::new();
        projects.add("/a");
        projects.add("/b");
        projects.add("/a"); // moves to front
        assert_eq!(projects.len(), 2);
        assert_eq!(projects.paths()[0], "/a");
        assert_eq!(projects.paths()[1], "/b");
    }

    #[test]
    fn add_truncates_at_max() {
        let mut projects = RecentProjects::new();
        for i in 0..MAX_RECENT_PROJECTS + 10 {
            projects.add(&format!("/project/{}", i));
        }
        assert_eq!(projects.len(), MAX_RECENT_PROJECTS);
    }

    #[test]
    fn remove_project() {
        let mut projects = RecentProjects::new();
        projects.add("/a");
        projects.add("/b");
        projects.remove("/a");
        assert_eq!(projects.len(), 1);
        assert_eq!(projects.paths()[0], "/b");
    }

    #[test]
    fn clear_projects() {
        let mut projects = RecentProjects::new();
        projects.add("/a");
        projects.add("/b");
        projects.clear();
        assert!(projects.is_empty());
    }

    #[test]
    fn projects_file_default() {
        let file = ProjectsFile::default();
        assert!(file.projects.is_empty());
    }
}
