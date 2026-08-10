//! Package model: descriptors, sources, and validated loaded packages
//! (ADDON_SPEC §9.3).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::error::AddonError;
use crate::ignore::IgnoreMatcher;
use crate::manifest::{validate_addon_id, AddonManifest};
use crate::pathsec;

/// A validated reverse-DNS addon id.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct AddonId(pub String);

impl AddonId {
    pub fn new(id: String) -> Result<Self, AddonError> {
        if validate_addon_id(&id) {
            Ok(Self(id))
        } else {
            Err(AddonError::invalid_addon(Some(id), "invalid addon id"))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// How a package is stored on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RootKind {
    Directory = 0,
    Zip = 1,
    Lzd = 2,
}

impl RootKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Directory => "directory",
            Self::Zip => "zip",
            Self::Lzd => "lzd",
        }
    }
}

/// Fully validated addon descriptor, ready for JSON serialization.
#[derive(Debug, Clone)]
pub struct AddonDescriptor {
    pub id: String,
    pub name: String,
    pub version: semver::Version,
    pub entry: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub api_version: (u32, u32),
    pub provider: String,
    pub root_kind: RootKind,
    pub size_bytes: u64,
    pub has_icons: BTreeMap<String, bool>,
    pub state: String,
    pub error: Option<String>,
}

/// An install/load input.
#[derive(Debug, Clone)]
pub enum PackageSource {
    Zip { path: PathBuf },
    Lzd { path: PathBuf },
    Directory { path: PathBuf },
}

/// An opened, validated package. All asset/entry access goes through this
/// type, which re-enforces normalization, ignore, and containment rules.
#[derive(Debug)]
pub struct LoadedPackage {
    id: String,
    root: PathBuf,
    ignore: IgnoreMatcher,
    manifest: AddonManifest,
}

impl LoadedPackage {
    pub fn new(
        id: String,
        root: PathBuf,
        ignore: IgnoreMatcher,
        manifest: AddonManifest,
    ) -> Result<Arc<Self>, AddonError> {
        Ok(Arc::new(Self {
            id,
            root,
            ignore,
            manifest,
        }))
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn manifest(&self) -> &AddonManifest {
        &self.manifest
    }

    /// Read a package file as validated UTF-8 bytes.
    ///
    /// Enforces (in order): normalization → traversal rejection → ignore
    /// rules → denial of `config.toml`/`.lzdignore` → canonical containment
    /// → bounded read (≤ 64 MiB).
    pub fn read_asset(&self, rel: &str) -> Result<Vec<u8>, AddonError> {
        let norm = pathsec::normalize_relative(rel)?;
        if norm == "config.toml" || norm == ".lzdignore" {
            return Err(AddonError::path_security(
                &norm,
                "config.toml and .lzdignore are not readable as assets",
            ));
        }
        if self.ignore.is_ignored(&norm, false) {
            return Err(AddonError::path_security(
                &norm,
                "path is excluded by ignore rules",
            ));
        }
        let target = self.root.join(&norm);
        pathsec::canonical_within(&self.root, &target)?;
        let meta = fs::metadata(&target)
            .map_err(|e| AddonError::internal(format!("cannot stat asset: {e}")))?;
        if !meta.is_file() {
            return Err(AddonError::invalid_addon(
                Some(self.id.clone()),
                "asset path is not a regular file",
            ));
        }
        if meta.len() > pathsec::MAX_ASSET_READ {
            return Err(AddonError::internal("asset exceeds the 64 MiB read limit"));
        }
        fs::read(&target).map_err(|e| AddonError::internal(format!("cannot read asset: {e}")))
    }

    /// Read the addon's entry script as bytes (never as a host path).
    pub fn entry_script(&self) -> Result<Vec<u8>, AddonError> {
        self.read_asset(&self.manifest.entry)
    }
}

/// Compute `assets/{16,32,64,128}.{ico,png}` availability.
pub fn compute_has_icons(root: &Path) -> BTreeMap<String, bool> {
    let mut map = BTreeMap::new();
    for size in [16u32, 32, 64, 128] {
        let mut ok = false;
        for ext in ["ico", "png"] {
            if root.join("assets").join(format!("{size}.{ext}")).is_file() {
                ok = true;
                break;
            }
        }
        map.insert(size.to_string(), ok);
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{validate_manifest, HOST_API_VERSION};
    use tempfile::TempDir;

    fn sample_manifest(app: &semver::Version) -> AddonManifest {
        let text = r#"
id = "example.addon"
name = "Example"
version = "1.0.0"
entry = "src/main.lua"
"#;
        validate_manifest(text, app, HOST_API_VERSION).unwrap()
    }

    #[test]
    fn read_asset_normalizes_and_rejects_meta() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/main.lua"), b"return {}").unwrap();
        fs::write(root.join("config.toml"), b"x").unwrap();

        let pkg = LoadedPackage::new(
            "example.addon".into(),
            root.to_path_buf(),
            IgnoreMatcher::from_lines(""),
            sample_manifest(&semver::Version::new(0, 3, 0)),
        )
        .unwrap();

        let bytes = pkg.read_asset("./src/main.lua").unwrap();
        assert_eq!(bytes, b"return {}");

        // config.toml and .lzdignore are denied as assets.
        assert!(pkg.read_asset("config.toml").is_err());
        assert!(pkg.read_asset(".lzdignore").is_err());

        // Traversal rejected.
        assert!(pkg.read_asset("../outside").is_err());
    }

    #[test]
    fn entry_script_returns_bytes() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/main.lua"), b"return {}").unwrap();
        let pkg = LoadedPackage::new(
            "example.addon".into(),
            root.to_path_buf(),
            IgnoreMatcher::from_lines(""),
            sample_manifest(&semver::Version::new(0, 3, 0)),
        )
        .unwrap();
        assert_eq!(pkg.entry_script().unwrap(), b"return {}");
    }

    #[test]
    fn addon_id_validation() {
        assert!(AddonId::new("a.b".into()).is_ok());
        assert!(AddonId::new("a".into()).is_err());
        assert!(AddonId::new("A.b".into()).is_err());
    }
}
