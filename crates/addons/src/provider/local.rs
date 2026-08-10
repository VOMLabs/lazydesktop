//! LocalProvider: scans user/system addon directories, extracts archives into
//! a provider-owned cache, resolves shadowing/conflicts, and serves validated
//! packages (ADDON_SPEC §10.2).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use dashmap::DashMap;
use semver::Version;

use crate::archive::{self, ContainerKind, Limits};
use crate::error::AddonError;
use crate::ignore::IgnoreMatcher;
use crate::manifest::{validate_manifest, AddonManifest, HOST_API_VERSION};
use crate::package::{self, AddonDescriptor, LoadedPackage, PackageSource, RootKind};
use crate::pathsec;
use crate::provider::{AddonProvider, ProviderError};
use crate::runtime;

const MANIFEST_FILE: &str = "config.toml";
const IGNORE_FILE: &str = ".lzdignore";
const MAX_IGNORE_FILE: u64 = 64 * 1024;

/// Result of one background load job: `(user-flag, source-name, outcome)` is
/// split out by [`LocalProvider::scan`] when draining the receivers.
type LoadJob = flume::Receiver<Result<LocalEntry, (String, AddonError)>>;

#[derive(Debug, Clone)]
struct Root {
    path: PathBuf,
    user: bool,
}

#[derive(Debug, Clone)]
enum EntryKind {
    Directory { path: PathBuf },
    Archive {
        archive_path: PathBuf,
        #[allow(dead_code)] // structural metadata for diagnostics/future use
        archive_kind: ContainerKind,
        cache_dir: PathBuf,
    },
}

/// A validated package found by a scan.
#[derive(Clone)]
struct LocalEntry {
    descriptor: AddonDescriptor,
    manifest: AddonManifest,
    ignore: IgnoreMatcher,
    root: PathBuf,
    kind: EntryKind,
    user: bool,
}

/// Local filesystem provider.
pub struct LocalProvider {
    roots: RwLock<Vec<Root>>,
    cache_root: RwLock<Option<PathBuf>>,
    limits: Limits,
    app_version: RwLock<Option<Version>>,
    host_api: RwLock<(u32, u32)>,
    entries: DashMap<String, LocalEntry>,
}

impl LocalProvider {
    pub fn new(limits: Limits) -> Self {
        Self {
            roots: RwLock::new(Vec::new()),
            cache_root: RwLock::new(None),
            limits,
            app_version: RwLock::new(None),
            host_api: RwLock::new(HOST_API_VERSION),
            entries: DashMap::new(),
        }
    }

    pub fn add_root(&self, path: &Path, user: bool) {
        self.roots.write().unwrap().push(Root {
            path: path.to_path_buf(),
            user,
        });
    }

    pub fn set_app_version(&self, v: Version) {
        *self.app_version.write().unwrap() = Some(v);
    }

    pub fn set_host_api(&self, api: (u32, u32)) {
        *self.host_api.write().unwrap() = api;
    }

    pub fn set_cache_root(&self, path: PathBuf) {
        *self.cache_root.write().unwrap() = Some(path);
    }

    fn app_version(&self) -> Version {
        self.app_version
            .read()
            .unwrap()
            .clone()
            .unwrap_or_else(|| Version::new(0, 0, 0))
    }

    /// Scan all roots and resolve winners/shadowed/conflict/error entries.
    pub fn scan(&self) -> Vec<AddonDescriptor> {
        let api = *self.host_api.read().unwrap();
        let app = self.app_version();
        let roots = self.roots.read().unwrap().clone();
        let cache_root = self.cache_root.read().unwrap().clone();

        // Phase 1: collect candidate paths (serial; cheap directory reads).
        struct Candidate {
            user: bool,
            name: String,
            path: PathBuf,
            kind: Option<ContainerKind>, // None = directory package
        }
        let mut candidates: Vec<Candidate> = Vec::new();
        for root in &roots {
            if !root.path.is_dir() {
                continue;
            }
            let mut names: Vec<String> = match fs::read_dir(&root.path) {
                Ok(rd) => rd
                    .filter_map(|e| e.ok())
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect(),
                Err(_) => continue,
            };
            names.sort();

            for name in names {
                let child = root.path.join(&name);
                let kind = if child.is_dir() {
                    if child.join(MANIFEST_FILE).is_file() {
                        None
                    } else {
                        continue;
                    }
                } else {
                    match extension_kind(&name) {
                        Some(k) => Some(k),
                        None => continue,
                    }
                };
                candidates.push(Candidate {
                    user: root.user,
                    name,
                    path: child,
                    kind,
                });
            }
        }

        // Phase 2: load every candidate concurrently on the background
        // blocking pool. Receivers are collected in candidate order so the
        // duplicate-resolution pass below stays deterministic.
        let receivers: Vec<(bool, String, LoadJob)> = candidates
            .into_iter()
            .map(|c| {
                let path = c.path;
                let app = app.clone();
                let cache = cache_root.clone();
                let limits = self.limits;
                let rx = match c.kind {
                    None => {
                        runtime::spawn_blocking(move || Self::load_directory(&path, &app, api))
                    }
                    Some(kind) => runtime::spawn_blocking(move || {
                        Self::load_archive(&path, kind, &app, api, &limits, &cache)
                    }),
                };
                (c.user, c.name, rx)
            })
            .collect();

        let mut found: Vec<(bool, LocalEntry)> = Vec::new();
        let mut errors: Vec<AddonDescriptor> = Vec::new();
        for (user, name, rx) in receivers {
            match rx.recv() {
                Ok(Ok(entry)) => found.push((user, entry)),
                Ok(Err((slug, err))) => errors.push(Self::error_descriptor(&slug, &name, err)),
                Err(_) => errors.push(Self::error_descriptor(
                    &Self::slug_id(&name, "job"),
                    &name,
                    AddonError::internal("background load failed"),
                )),
            }
        }

        // Resolve duplicates deterministically.
        let mut shadowed: Vec<AddonDescriptor> = Vec::new();
        let mut by_id: HashMap<String, usize> = HashMap::new(); // id -> index into found

        for (idx, (user, entry)) in found.iter().enumerate() {
            match by_id.get(&entry.descriptor.id).copied() {
                None => {
                    by_id.insert(entry.descriptor.id.clone(), idx);
                }
                Some(prev_idx) => {
                    let (prev_user, prev_entry) = (&found[prev_idx].0, &found[prev_idx].1);
                    let replace = if *user != *prev_user {
                        *user && !*prev_user // user root overrides system root
                    } else {
                        entry.manifest.version > prev_entry.manifest.version
                    };
                    if replace {
                        let mut d = prev_entry.descriptor.clone();
                        d.state = "shadowed".to_string();
                        shadowed.push(d);
                        by_id.insert(entry.descriptor.id.clone(), idx);
                    } else if entry.manifest.version == prev_entry.manifest.version {
                        // Equal version in the same priority: conflict.
                        let mut d = entry.descriptor.clone();
                        d.state = "conflict".to_string();
                        d.error = Some(format!(
                            "duplicate addon id '{}' with equal version {}",
                            entry.descriptor.id, entry.manifest.version
                        ));
                        shadowed.push(d);
                    } else {
                        let mut d = entry.descriptor.clone();
                        d.state = "shadowed".to_string();
                        shadowed.push(d);
                    }
                }
            }
        }

        // Build winners from by_id indices.
        let mut winner_ids: Vec<usize> = by_id.values().copied().collect();
        winner_ids.sort_unstable();
        winner_ids.dedup();

        let mut result: Vec<AddonDescriptor> = Vec::new();
        let mut entries_map: HashMap<String, LocalEntry> = HashMap::new();
        for idx in winner_ids {
            let (user, entry) = &found[idx];
            let mut entry = entry.clone();
            // load_directory/load_archive hardcode user:false; the flag only
            // lives on the root tuple. Propagate it so uninstall/open know
            // whether the winner lives in a user root.
            entry.user = *user;
            entries_map.insert(entry.descriptor.id.clone(), entry);
            result.push(found[idx].1.descriptor.clone());
        }
        result.extend(shadowed);
        result.extend(errors);
        result.sort_by(|a, b| a.name.cmp(&b.name).then_with(|| a.id.cmp(&b.id)));

        self.entries.clear();
        for (id, entry) in entries_map {
            self.entries.insert(id, entry);
        }
        result
    }

    fn error_descriptor(slug_id: &str, source_name: &str, err: AddonError) -> AddonDescriptor {
        AddonDescriptor {
            id: slug_id.to_string(),
            name: source_name.to_string(),
            version: Version::new(0, 0, 0),
            entry: String::new(),
            description: None,
            author: None,
            license: None,
            api_version: (0, 0),
            provider: "local".to_string(),
            root_kind: RootKind::Directory,
            size_bytes: 0,
            has_icons: Default::default(),
            state: "error".to_string(),
            error: Some(err.display_message()),
        }
    }

    fn slug_id(source_name: &str, fallback: &str) -> String {
        // Deterministic id for sources whose manifest cannot be read.
        let mut out = String::from("invalid.");
        for c in source_name.chars() {
            if c.is_ascii_alphanumeric() {
                out.push(c.to_ascii_lowercase());
            } else {
                out.push('.');
            }
        }
        let out = out.trim_end_matches('.').to_string();
        if out.len() > "invalid.".len() {
            out
        } else {
            format!("invalid.{fallback}")
        }
    }

    fn load_directory(
        dir: &Path,
        app: &Version,
        api: (u32, u32),
    ) -> Result<LocalEntry, (String, AddonError)> {
        let manifest = match read_validate_manifest_file(&dir.join(MANIFEST_FILE), app, api) {
            Ok(m) => m,
            Err(e) => {
                let name = dir
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_default();
                return Err((Self::slug_id(&name, "dir"), e));
            }
        };
        let root = dir.to_path_buf();
        let ignore = load_ignore_dir(&root);
        if let Err((_, e)) = validate_entry(&root, &manifest, &ignore) {
            return Err((manifest.id.clone(), e));
        }
        let descriptor = build_descriptor(
            &manifest,
            RootKind::Directory,
            "local",
            &root,
            &ignore,
            "installed",
            None,
        );
        Ok(LocalEntry {
            descriptor,
            manifest,
            ignore,
            root,
            kind: EntryKind::Directory { path: dir.to_path_buf() },
            user: false,
        })
    }

    fn load_archive(
        archive_path: &Path,
        kind: ContainerKind,
        app: &Version,
        api: (u32, u32),
        limits: &Limits,
        cache_root: &Option<PathBuf>,
    ) -> Result<LocalEntry, (String, AddonError)> {
        let source_name = archive_path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let slug = Self::slug_id(&source_name, "arc");
        let manifest = match archive::peek_manifest(archive_path, app, api, limits) {
            Ok(m) => m,
            Err(e) => return Err((slug, e)),
        };
        let id_preview = manifest.id.clone();

        let cache_dir = match ensure_extracted(archive_path, limits, cache_root) {
            Ok(d) => d,
            Err(e) => return Err((id_preview, e)),
        };

        let manifest = match read_validate_manifest_file(&cache_dir.join(MANIFEST_FILE), app, api) {
            Ok(m) => m,
            Err(e) => return Err((id_preview, e)),
        };
        let ignore = load_ignore_dir(&cache_dir);
        if let Err((_, e)) = validate_entry(&cache_dir, &manifest, &ignore) {
            return Err((manifest.id.clone(), e));
        }

        let root_kind = match kind {
            ContainerKind::Zip => RootKind::Zip,
            ContainerKind::Lzd => RootKind::Lzd,
        };
        let descriptor = build_descriptor(
            &manifest,
            root_kind,
            "local",
            &cache_dir,
            &ignore,
            "installed",
            None,
        );
        Ok(LocalEntry {
            descriptor,
            manifest,
            ignore,
            root: cache_dir.clone(),
            kind: EntryKind::Archive {
                archive_path: archive_path.to_path_buf(),
                archive_kind: kind,
                cache_dir,
            },
            user: false,
        })
    }

    fn install_to_user_root(&self, source: &PackageSource) -> Result<AddonDescriptor, AddonError> {
        let app = self.app_version();
        let api = *self.host_api.read().unwrap();
        let roots = self.roots.read().unwrap().clone();
        let user_root = roots.iter().find(|r| r.user).ok_or_else(|| {
            AddonError::provider("local", "no writable addon directory configured")
        })?;

        let manifest = match source {
            PackageSource::Directory { path } => {
                let text = fs::read_to_string(path.join(MANIFEST_FILE))
                    .map_err(|e| AddonError::internal(format!("cannot read manifest: {e}")))?;
                validate_manifest(&text, &app, api)?
            }
            PackageSource::Zip { path } | PackageSource::Lzd { path } => {
                archive::peek_manifest(path, &app, api, &self.limits)?
            }
        };

        // Replace existing older version, reject equal/newer.
        let existing = self.entries.get(&manifest.id).map(|e| e.clone());
        if let Some(e) = &existing {
            if e.manifest.version >= manifest.version {
                return Err(AddonError::provider(
                    "local",
                    format!(
                        "addon '{}' version {} is already installed (tried {})",
                        manifest.id, e.manifest.version, manifest.version
                    ),
                ));
            }
            self.remove_entry(e)?;
        }

        let user_dir = user_root.path.clone();
        fs::create_dir_all(&user_dir)
            .map_err(|e| AddonError::internal(format!("cannot create addon dir: {e}")))?;

        match source {
            PackageSource::Directory { path } => {
                let dest = user_dir.join(&manifest.id);
                if dest.exists() {
                    fs::remove_dir_all(&dest)
                        .map_err(|e| AddonError::internal(format!("cannot replace addon dir: {e}")))?;
                }
                let ignore = load_ignore_dir(path);
                copy_tree(path, &dest, &ignore)?;
            }
            PackageSource::Zip { path } => {
                let dest = user_dir.join(format!("{}.zip", manifest.id));
                fs::copy(path, &dest)
                    .map_err(|e| AddonError::internal(format!("cannot copy package: {e}")))?;
            }
            PackageSource::Lzd { path } => {
                let dest = user_dir.join(format!("{}.lzd", manifest.id));
                fs::copy(path, &dest)
                    .map_err(|e| AddonError::internal(format!("cannot copy package: {e}")))?;
            }
        }

        // Re-scan and return the winning descriptor.
        self.scan();
        self.entries
            .get(&manifest.id)
            .map(|e| e.descriptor.clone())
            .ok_or_else(|| AddonError::internal("installed addon did not validate"))
    }

    fn remove_entry(&self, entry: &LocalEntry) -> Result<(), ProviderError> {
        if !entry.user {
            return Err(ProviderError::ReadOnly(
                "system addons cannot be uninstalled; disable them instead".to_string(),
            ));
        }
        match &entry.kind {
            EntryKind::Directory { path } => {
                if path.exists() {
                    fs::remove_dir_all(path)
                        .map_err(|e| ProviderError::Io(format!("remove dir: {e}")))?;
                }
            }
            EntryKind::Archive {
                archive_path,
                cache_dir,
                ..
            } => {
                if archive_path.exists() {
                    fs::remove_file(archive_path)
                        .map_err(|e| ProviderError::Io(format!("remove archive: {e}")))?;
                }
                if cache_dir.exists() {
                    fs::remove_dir_all(cache_dir)
                        .map_err(|e| ProviderError::Io(format!("remove cache: {e}")))?;
                }
            }
        }
        Ok(())
    }
}

impl AddonProvider for LocalProvider {
    fn provider_id(&self) -> &str {
        "local"
    }

    fn display_name(&self) -> String {
        "Local".to_string()
    }

    fn load(&self) -> Result<Vec<AddonDescriptor>, ProviderError> {
        Ok(self.scan())
    }

    fn install(&self, source: &PackageSource) -> Result<AddonDescriptor, ProviderError> {
        self.install_to_user_root(source)
            .map_err(ProviderError::from_addon)
    }

    fn uninstall(&self, id: &str) -> Result<(), ProviderError> {
        let entry = self
            .entries
            .get(id)
            .map(|e| e.clone())
            .ok_or_else(|| ProviderError::InvalidAddon(format!("addon '{id}' is not installed")))?;
        self.remove_entry(&entry)?;
        let _ = self.scan();
        Ok(())
    }

    fn open(&self, id: &str) -> Result<Arc<LoadedPackage>, ProviderError> {
        let build = |e: &LocalEntry| {
            LoadedPackage::new(
                e.descriptor.id.clone(),
                e.root.clone(),
                e.ignore.clone(),
                e.manifest.clone(),
            )
            .map_err(ProviderError::from_addon)
        };
        // The DashMap read guard must be dropped before `scan()` re-locks the
        // same map, so the lookup is scoped.
        let hit = {
            let e = self.entries.get(id);
            match e.as_deref() {
                Some(e) if e.descriptor.state == "conflict" => {
                    return Err(ProviderError::Conflict(format!(
                        "addon '{}' has a conflicting duplicate",
                        id
                    )));
                }
                Some(e) => Some(build(e)),
                None => None,
            }
        };
        if let Some(result) = hit {
            return result;
        }
        // Stale cache: rescan once, then re-check.
        self.scan();
        match self.entries.get(id) {
            Some(e) => build(&e),
            None => Err(ProviderError::InvalidAddon(format!(
                "addon '{id}' is not installed"
            ))),
        }
    }
}

// ─── Helpers ───────────────────────────────────────────────

fn extension_kind(name: &str) -> Option<ContainerKind> {
    if name.ends_with(".lzd") {
        Some(ContainerKind::Lzd)
    } else if name.ends_with(".zip") {
        Some(ContainerKind::Zip)
    } else {
        None
    }
}

fn read_validate_manifest_file(
    path: &Path,
    app: &Version,
    api: (u32, u32),
) -> Result<AddonManifest, AddonError> {
    let meta = fs::metadata(path)
        .map_err(|e| AddonError::internal(format!("cannot stat manifest: {e}")))?;
    if meta.len() > crate::manifest::MAX_MANIFEST_SIZE as u64 {
        return Err(AddonError::invalid_manifest(
            None,
            Some("config.toml".to_string()),
            "manifest exceeds the 64 KiB size limit",
        ));
    }
    let text = fs::read_to_string(path)
        .map_err(|e| AddonError::internal(format!("cannot read manifest: {e}")))?;
    validate_manifest(&text, app, api)
}

fn load_ignore_dir(dir: &Path) -> IgnoreMatcher {
    match fs::read(dir.join(IGNORE_FILE)) {
        Ok(bytes) if (bytes.len() as u64) <= MAX_IGNORE_FILE => {
            IgnoreMatcher::from_lines(&String::from_utf8_lossy(&bytes))
        }
        _ => IgnoreMatcher::from_lines(""),
    }
}

/// Validate that the entry script exists, is a regular file, is inside the
/// root, and is not ignored.
fn validate_entry(
    root: &Path,
    manifest: &AddonManifest,
    ignore: &IgnoreMatcher,
) -> Result<(), (String, AddonError)> {
    let entry_path = root.join(&manifest.entry);
    let meta = match fs::symlink_metadata(&entry_path) {
        Ok(m) => m,
        Err(_) => {
            return Err((
                manifest.id.clone(),
                AddonError::invalid_addon(
                    Some(manifest.id.clone()),
                    "entry script is missing from the package",
                ),
            ))
        }
    };
    if !meta.is_file() || meta.file_type().is_symlink() {
        return Err((
            manifest.id.clone(),
            AddonError::invalid_addon(
                Some(manifest.id.clone()),
                "entry script must be a regular file",
            ),
        ));
    }
    if pathsec::canonical_within(root, &entry_path).is_err() {
        return Err((
            manifest.id.clone(),
            AddonError::path_security(
                &manifest.entry,
                "entry script escapes the package root",
            ),
        ));
    }
    if ignore.is_ignored(&manifest.entry, false) {
        return Err((
            manifest.id.clone(),
            AddonError::invalid_addon(
                Some(manifest.id.clone()),
                "entry script is excluded by ignore rules",
            ),
        ));
    }
    Ok(())
}

fn build_descriptor(
    manifest: &AddonManifest,
    root_kind: RootKind,
    provider: &str,
    root: &Path,
    ignore: &IgnoreMatcher,
    state: &str,
    error: Option<String>,
) -> AddonDescriptor {
    AddonDescriptor {
        id: manifest.id.clone(),
        name: manifest.name.clone(),
        version: manifest.version.clone(),
        entry: manifest.entry.clone(),
        description: manifest.description.clone(),
        author: manifest.author.clone(),
        license: manifest.license.clone(),
        api_version: manifest.api_version,
        provider: provider.to_string(),
        root_kind,
        size_bytes: archive::content_size(root, ignore),
        has_icons: package::compute_has_icons(root),
        state: state.to_string(),
        error,
    }
}

/// Extract an archive into the provider cache (keyed by content hash).
fn ensure_extracted(
    archive_path: &Path,
    limits: &Limits,
    cache_root: &Option<PathBuf>,
) -> Result<PathBuf, AddonError> {
    let cache_root = cache_root
        .clone()
        .ok_or_else(|| AddonError::internal("cache root is not configured"))?;
    let meta = fs::metadata(archive_path)
        .map_err(|e| AddonError::internal(format!("cannot stat package: {e}")))?;
    let key = cache_key(archive_path, &meta);
    let cache_dir = cache_root.join(key);

    if cache_dir.join(MANIFEST_FILE).is_file() {
        return Ok(cache_dir); // validated on a previous scan
    }

    fs::create_dir_all(&cache_root)
        .map_err(|e| AddonError::internal(format!("cannot create cache: {e}")))?;
    if cache_dir.exists() {
        let _ = fs::remove_dir_all(&cache_dir);
    }

    // Extract honoring the package's own ignore rules.
    let ignore_text = archive::read_ignore_content(archive_path, limits)?;
    let ignore = IgnoreMatcher::from_lines(&ignore_text);
    let result = archive::extract(archive_path, &cache_dir, &ignore, limits);
    match result {
        Ok(_) => Ok(cache_dir),
        Err(e) => {
            let _ = fs::remove_dir_all(&cache_dir);
            Err(e)
        }
    }
}

fn cache_key(path: &Path, meta: &fs::Metadata) -> String {
    // FNV-1a over (canonical path, mtime, size).
    let mut h = 0xcbf29ce484222325u64;
    let mut feed = |bytes: &[u8]| {
        for &b in bytes {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    };
    feed(path.to_string_lossy().as_bytes());
    if let Ok(t) = meta.modified() {
        if let Ok(d) = t.duration_since(std::time::UNIX_EPOCH) {
            feed(&d.as_nanos().to_le_bytes());
        }
    }
    feed(&meta.len().to_le_bytes());
    format!("{h:016x}")
}

/// Recursively copy a directory package into the user root, skipping ignored
/// files and refusing symlinks that escape the source root.
fn copy_tree(src: &Path, dest: &Path, ignore: &IgnoreMatcher) -> Result<(), AddonError> {
    fs::create_dir_all(dest)
        .map_err(|e| AddonError::internal(format!("cannot create dest: {e}")))?;
    for entry in walkdir::WalkDir::new(src).follow_links(false) {
        let e = entry.map_err(|err| AddonError::internal(format!("walk failed: {err}")))?;
        let rel = e
            .path()
            .strip_prefix(src)
            .map_err(|_| AddonError::internal("path prefix error"))?
            .to_string_lossy()
            .replace('\\', "/");
        if rel.is_empty() {
            continue;
        }
        let target = dest.join(&rel);
        if e.file_type().is_dir() {
            fs::create_dir_all(&target)
                .map_err(|err| AddonError::internal(format!("cannot create dir: {err}")))?;
        } else if e.file_type().is_symlink() {
            let link_target = fs::read_link(e.path())
                .map_err(|err| AddonError::internal(format!("cannot read symlink: {err}")))?;
            let entry_dir = e.path().parent().unwrap_or(src);
            let resolved =
                pathsec::validate_symlink_target(entry_dir, src, &link_target.to_string_lossy())
                    .map_err(|_| {
                        AddonError::path_security(&rel, "symlink escapes the package root")
                    })?;
            #[cfg(unix)]
            std::os::unix::fs::symlink(&resolved, &target)
                .map_err(|err| AddonError::internal(format!("cannot create symlink: {err}")))?;
            #[cfg(not(unix))]
            let _ = resolved;
        } else if e.file_type().is_file() {
            if ignore.is_ignored(&rel, false) {
                continue;
            }
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)
                    .map_err(|err| AddonError::internal(format!("cannot create dir: {err}")))?;
            }
            fs::copy(e.path(), &target)
                .map_err(|err| AddonError::internal(format!("cannot copy file: {err}")))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn addon_dir(root: &Path, id: &str, version: &str) {
        let dir = root.join(id);
        fs::create_dir_all(dir.join("src")).unwrap();
        fs::write(
            dir.join("config.toml"),
            format!(
                "id = \"{id}\"\nname = \"{id}\"\nversion = \"{version}\"\nentry = \"src/main.lua\"\n"
            ),
        )
        .unwrap();
        fs::write(dir.join("src/main.lua"), "return {}").unwrap();
    }

    #[test]
    fn scans_and_resolves_version_shadowing() {
        let tmp = TempDir::new().unwrap();
        let user = tmp.path().join("user");
        let sys = tmp.path().join("sys");
        fs::create_dir_all(&user).unwrap();
        fs::create_dir_all(&sys).unwrap();
        addon_dir(&user, "dupe.addon", "2.0.0");
        addon_dir(&sys, "dupe.addon", "1.0.0");
        addon_dir(&user, "only.user", "1.0.0");

        let p = LocalProvider::new(Limits::default());
        p.set_app_version(Version::new(0, 3, 0));
        p.add_root(&user, true);
        p.add_root(&sys, false);
        let list = p.scan();
        let winner = list
            .iter()
            .find(|d| d.id == "dupe.addon" && d.state == "installed")
            .unwrap();
        assert_eq!(winner.version.to_string(), "2.0.0");
        assert!(list
            .iter()
            .any(|d| d.id == "dupe.addon" && d.state == "shadowed"));
        assert!(p.open("dupe.addon").is_ok());
        assert!(p.open("only.user").is_ok());
    }

    #[test]
    fn open_after_install_and_uninstall() {
        use std::io::Write;

        let tmp = TempDir::new().unwrap();
        let user = tmp.path().join("user");
        let cache = tmp.path().join("cache");
        fs::create_dir_all(&user).unwrap();
        let p = LocalProvider::new(Limits::default());
        p.set_app_version(Version::new(0, 3, 0));
        p.set_cache_root(cache);
        p.add_root(&user, true);

        // Install a zip.
        let src = tmp.path().join("pkg.zip");
        let file = fs::File::create(&src).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default();
        zip.start_file("config.toml", opts).unwrap();
        zip.write_all(
            b"id = \"inst.addon\"\nname = \"Inst\"\nversion = \"1.0.0\"\nentry = \"main.lua\"\n",
        )
        .unwrap();
        zip.start_file("main.lua", opts).unwrap();
        zip.write_all(b"return {}").unwrap();
        zip.finish().unwrap();

        let source = PackageSource::Zip { path: src };
        let d = p.install(&source).unwrap();
        assert_eq!(d.id, "inst.addon");
        assert!(p.open("inst.addon").is_ok());
        assert!(user.join("inst.addon.zip").exists());

        p.uninstall("inst.addon").unwrap();
        assert!(!user.join("inst.addon.zip").exists());
        assert!(p.open("inst.addon").is_err());
    }

    #[test]
    fn uninstall_system_addon_is_readonly() {
        let tmp = TempDir::new().unwrap();
        let sys = tmp.path().join("sys");
        fs::create_dir_all(&sys).unwrap();
        addon_dir(&sys, "sys.addon", "1.0.0");
        let p = LocalProvider::new(Limits::default());
        p.set_app_version(Version::new(0, 3, 0));
        p.add_root(&sys, false);
        p.scan();
        assert!(matches!(
            p.uninstall("sys.addon"),
            Err(ProviderError::ReadOnly(_))
        ));
    }

    #[test]
    fn scan_loads_archive_packages_via_background_pool() {
        use std::io::Write;

        let tmp = TempDir::new().unwrap();
        let sys = tmp.path().join("sys");
        let cache = tmp.path().join("cache");
        fs::create_dir_all(&sys).unwrap();

        // A raw zip package placed directly in the (system) root. Scan must
        // dispatch `load_archive` → `ensure_extracted` on the background
        // blocking pool and surface the extracted package.
        let pkg = sys.join("pkg.addon.zip");
        let file = fs::File::create(&pkg).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default();
        zip.start_file("config.toml", opts).unwrap();
        zip.write_all(
            b"id = \"arch.addon\"\nname = \"Arch\"\nversion = \"1.0.0\"\nentry = \"main.lua\"\n",
        )
        .unwrap();
        zip.start_file("main.lua", opts).unwrap();
        zip.write_all(b"return {}").unwrap();
        zip.finish().unwrap();

        let p = LocalProvider::new(Limits::default());
        p.set_app_version(Version::new(0, 3, 0));
        p.set_cache_root(cache.clone());
        p.add_root(&sys, false);
        let list = p.scan();
        assert!(list.iter().any(|d| d.id == "arch.addon"));
        assert!(p.open("arch.addon").is_ok());
        // Extraction landed in the provider-owned cache.
        assert!(cache.exists());
    }
}
