//! Addon registry: provider aggregation, snapshot index, and operations
//! (ADDON_SPEC §10.4).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use semver::Version;

use crate::archive::Limits;
use crate::error::AddonError;
use crate::manifest::{validate_addon_id, HOST_API_VERSION};
use crate::package::{AddonDescriptor, LoadedPackage};
use crate::provider::{
    local::LocalProvider, package_source_from_path, AddonProvider, DynProvider,
};

/// Immutable snapshot of the resolved addon set. Swapped atomically on
/// load/refresh so readers never block on writers.
///
/// `by_id` holds only the winning descriptor per id (used by `get`/lookups);
/// `all` holds every surfaced descriptor — winners plus shadowed and error
/// entries — in display order (used by `list`).
#[derive(Debug, Default)]
pub struct RegistrySnapshot {
    by_id: HashMap<String, AddonDescriptor>,
    all: Vec<AddonDescriptor>,
}

impl RegistrySnapshot {
    fn list(&self) -> Vec<AddonDescriptor> {
        self.all.clone()
    }
}

/// The addon system facade used by the FFI layer.
pub struct AddonRegistry {
    local: Arc<LocalProvider>,
    others: Vec<DynProvider>,
    snapshot: RwLock<Arc<RegistrySnapshot>>,
    storage_root: RwLock<Option<PathBuf>>,
}

impl AddonRegistry {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            local: Arc::new(LocalProvider::new(Limits::default())),
            others: Vec::new(),
            snapshot: RwLock::new(Arc::new(RegistrySnapshot::default())),
            storage_root: RwLock::new(None),
        })
    }

    pub fn set_app_version(&self, version: &str) -> Result<(), AddonError> {
        let v = Version::parse(version)
            .map_err(|_| AddonError::internal(format!("invalid app version {version:?}")))?;
        self.local.set_app_version(v);
        Ok(())
    }

    pub fn set_host_api(&self, api: (u32, u32)) {
        self.local.set_host_api(api);
    }

    pub fn host_api(&self) -> (u32, u32) {
        HOST_API_VERSION
    }

    pub fn add_root(&self, path: &str, user_priority: bool) -> Result<(), AddonError> {
        if path.is_empty() {
            return Err(AddonError::internal("empty addon root path"));
        }
        self.local.add_root(Path::new(path), user_priority);
        Ok(())
    }

    pub fn set_storage_root(&self, path: &str) -> Result<(), AddonError> {
        if path.is_empty() {
            return Err(AddonError::internal("empty storage root path"));
        }
        *self.storage_root.write().unwrap() = Some(PathBuf::from(path));
        self.local
            .set_cache_root(Path::new(path).join("addons-cache"));
        Ok(())
    }

    pub fn load(&self) -> Result<(), AddonError> {
        let mut by_id: HashMap<String, AddonDescriptor> = HashMap::new();
        let mut extras: Vec<AddonDescriptor> = Vec::new();

        let local_list = self.local.load()?;
        merge_descriptors(&mut by_id, &mut extras, local_list);

        for p in &self.others {
            let list = p.load()?;
            merge_descriptors(&mut by_id, &mut extras, list);
        }

        let mut all = by_id.values().cloned().collect::<Vec<_>>();
        all.extend(extras);
        all.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));

        *self.snapshot.write().unwrap() = Arc::new(RegistrySnapshot {
            by_id,
            all,
        });
        Ok(())
    }

    pub fn refresh(&self) -> Result<(), AddonError> {
        self.load()
    }

    pub fn list(&self) -> Vec<AddonDescriptor> {
        self.snapshot.read().unwrap().list()
    }

    pub fn get(&self, id: &str) -> Option<AddonDescriptor> {
        self.snapshot.read().unwrap().by_id.get(id).cloned()
    }

    pub fn install(&self, source_path: &Path) -> Result<AddonDescriptor, AddonError> {
        let source = package_source_from_path(source_path);
        let descriptor = self.local.install(&source)?;
        self.load()?;
        self.get(&descriptor.id)
            .ok_or_else(|| AddonError::internal("installed addon vanished after refresh"))
    }

    pub fn uninstall(&self, id: &str) -> Result<(), AddonError> {
        self.local.uninstall(id)?;
        self.load()?;
        Ok(())
    }

    pub fn open(&self, id: &str) -> Result<Arc<LoadedPackage>, AddonError> {
        self.local.open(id).map_err(AddonError::from)
    }

    pub fn read_asset(&self, id: &str, rel: &str) -> Result<Vec<u8>, AddonError> {
        self.open(id)?.read_asset(rel)
    }

    pub fn has_asset(&self, id: &str, rel: &str) -> bool {
        self.read_asset(id, rel).is_ok()
    }

    pub fn entry_script(&self, id: &str) -> Result<Vec<u8>, AddonError> {
        self.open(id)?.entry_script()
    }

    pub fn storage_dir(&self, id: &str) -> Result<PathBuf, AddonError> {
        if !validate_addon_id(id) {
            return Err(AddonError::invalid_addon(
                Some(id.to_string()),
                "invalid addon id",
            ));
        }
        let root = self
            .storage_root
            .read()
            .unwrap()
            .clone()
            .ok_or_else(|| AddonError::internal("storage root is not configured"))?;
        Ok(root.join("addon-data").join(id))
    }
}

/// Merge provider descriptors: `installed` wins on first occurrence; anything
/// else goes to `extras`; a later collision is demoted to `shadowed`.
fn merge_descriptors(
    by_id: &mut HashMap<String, AddonDescriptor>,
    extras: &mut Vec<AddonDescriptor>,
    descriptors: Vec<AddonDescriptor>,
) {
    for mut d in descriptors {
        if d.state == "installed" {
            if by_id.contains_key(&d.id) {
                d.state = "shadowed".to_string();
                extras.push(d);
            } else {
                by_id.insert(d.id.clone(), d);
            }
        } else {
            extras.push(d);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn write_dir_addon(root: &Path, manifest: &str, entry_body: &str) {
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(root.join("config.toml"), manifest).unwrap();
        std::fs::write(root.join("src/main.lua"), entry_body).unwrap();
    }

    #[test]
    fn registry_load_list_get_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let user = tmp.path().join("user");
        std::fs::create_dir_all(&user).unwrap();
        write_dir_addon(
            &user.join("one"),
            "id = \"a.one\"\nname = \"One\"\nversion = \"1.0.0\"\nentry = \"src/main.lua\"\n",
            "return {}",
        );
        let reg = AddonRegistry::new();
        reg.set_app_version("0.3.0").unwrap();
        reg.set_storage_root(tmp.path().join("cfg").to_str().unwrap())
            .unwrap();
        reg.add_root(user.to_str().unwrap(), true).unwrap();
        reg.load().unwrap();

        let list = reg.list();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].id, "a.one");
        assert_eq!(reg.get("a.one").unwrap().name, "One");

        let script = reg.entry_script("a.one").unwrap();
        assert_eq!(script, b"return {}");
        assert!(reg.open("a.one").is_ok());
    }

    #[test]
    fn user_root_wins_over_system() {
        let tmp = TempDir::new().unwrap();
        let user = tmp.path().join("user");
        let system = tmp.path().join("system");
        std::fs::create_dir_all(&user).unwrap();
        std::fs::create_dir_all(&system).unwrap();
        write_dir_addon(
            &user.join("one"),
            "id = \"a.one\"\nname = \"UserOne\"\nversion = \"1.0.0\"\nentry = \"src/main.lua\"\n",
            "return {}",
        );
        write_dir_addon(
            &system.join("one"),
            "id = \"a.one\"\nname = \"SysOne\"\nversion = \"9.0.0\"\nentry = \"src/main.lua\"\n",
            "return {}",
        );
        let reg = AddonRegistry::new();
        reg.set_app_version("0.3.0").unwrap();
        reg.set_storage_root(tmp.path().join("cfg").to_str().unwrap())
            .unwrap();
        reg.add_root(user.to_str().unwrap(), true).unwrap();
        reg.add_root(system.to_str().unwrap(), false).unwrap();
        reg.load().unwrap();

        let list = reg.list();
        assert_eq!(list.len(), 2);
        let winner = reg.get("a.one").unwrap();
        assert_eq!(winner.name, "UserOne", "user root must win over system");
    }

    #[test]
    fn invalid_manifest_is_surfaced_not_fatal() {
        let tmp = TempDir::new().unwrap();
        let user = tmp.path().join("user");
        std::fs::create_dir_all(user.join("bad")).unwrap();
        std::fs::write(user.join("bad/config.toml"), "id = [not valid").unwrap();

        let reg = AddonRegistry::new();
        reg.set_app_version("0.3.0").unwrap();
        reg.add_root(user.to_str().unwrap(), true).unwrap();
        reg.load().unwrap();
        assert!(!reg.list().is_empty());
        assert_eq!(reg.list()[0].state, "error");
    }
}
