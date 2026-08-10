//! Archive parsing and secure extraction for `.zip` and `.lzd` packages
//! (ADDON_SPEC §5.3, §8.3).
//!
//! All extraction is streaming with hard limits. Nothing in this module trusts
//! archive metadata beyond structural validation; every path goes through
//! [`crate::pathsec`] and every entry through the ignore matcher.

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::path::Path;

use crate::error::AddonError;
use crate::ignore::IgnoreMatcher;
use crate::manifest::{validate_manifest, AddonManifest};
use crate::pathsec;

/// `.lzd` container magic and header size (ADDON_SPEC §5.3).
pub const LZD_MAGIC: &[u8; 8] = b"LZDPKG01";
pub const LZD_HEADER_SIZE: u64 = 24;
/// Maximum symlink target length.
const MAX_SYMLINK_TARGET: u64 = 4096;

/// Hard archive limits (defaults per §8.3; bounded and provider-configurable).
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub max_archive_size: u64,
    pub max_entries: usize,
    pub max_total_uncompressed: u64,
    pub max_entry_uncompressed: u64,
    pub max_ratio: u64,
    pub max_name_len: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_archive_size: 1024 * 1024 * 1024,     // 1 GiB
            max_entries: 4096,
            max_total_uncompressed: 512 * 1024 * 1024, // 512 MiB
            max_entry_uncompressed: 128 * 1024 * 1024, // 128 MiB
            max_ratio: 300,
            max_name_len: 1024,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerKind {
    Zip,
    Lzd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtractionStats {
    pub entries: usize,
    pub total_uncompressed: u64,
}

/// A bounded view of a `Read + Seek` source between `start` and `end`.
struct OffsetReader<R: Read + Seek> {
    inner: R,
    start: u64,
    end: u64,
    pos: u64,
}

impl<R: Read + Seek> Read for OffsetReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.pos >= self.end {
            return Ok(0);
        }
        let limit = ((self.end - self.pos) as usize).min(buf.len());
        let n = self.inner.read(&mut buf[..limit])?;
        self.pos += n as u64;
        Ok(n)
    }
}

impl<R: Read + Seek> Seek for OffsetReader<R> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        let base = match pos {
            SeekFrom::Start(p) => self
                .start
                .checked_add(p)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "seek overflow"))?,
            SeekFrom::End(p) => {
                let end_i = self.end as i128 + p as i128;
                if end_i < self.start as i128 || end_i > self.end as i128 {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, "seek out of range"));
                }
                end_i as u64
            }
            SeekFrom::Current(p) => {
                let cur = self.pos as i128 + p as i128;
                if cur < self.start as i128 || cur > self.end as i128 {
                    return Err(io::Error::new(io::ErrorKind::InvalidInput, "seek out of range"));
                }
                cur as u64
            }
        };
        self.inner.seek(SeekFrom::Start(base))?;
        self.pos = base;
        Ok(base - self.start)
    }
}

/// Extract a package into `dest`, honoring ignore rules and hard limits.
///
/// On any error the caller is responsible for deleting a partially written
/// `dest` (the provider layer does this).
pub fn extract(
    source: &Path,
    dest: &Path,
    ignore: &IgnoreMatcher,
    limits: &Limits,
) -> Result<ExtractionStats, AddonError> {
    let mut file = File::open(source)
        .map_err(|e| AddonError::internal(format!("cannot open package: {e}")))?;
    let file_len = file
        .metadata()
        .map_err(|e| AddonError::internal(format!("cannot stat package: {e}")))?
        .len();
    if file_len > limits.max_archive_size {
        return Err(AddonError::archive(
            "size".to_string(),
            "package exceeds the maximum archive size",
        ));
    }

    let kind = detect_kind(&mut file)?;
    match kind {
        ContainerKind::Zip => {
            validate_zip_tail(&mut file, 0, file_len)?;
            file.seek(SeekFrom::Start(0))
                .map_err(|e| AddonError::internal(format!("seek failed: {e}")))?;
            let mut zip = open_zip(file)?;
            extract_entries(&mut zip, dest, ignore, limits)
        }
        ContainerKind::Lzd => {
            validate_lzd_header(&mut file)?;
            validate_zip_tail(&mut file, LZD_HEADER_SIZE, file_len)?;
            let reader = OffsetReader {
                inner: file,
                start: LZD_HEADER_SIZE,
                end: file_len,
                pos: LZD_HEADER_SIZE,
            };
            let mut zip = open_zip(reader)?;
            extract_entries(&mut zip, dest, ignore, limits)
        }
    }
}

/// Read and validate only the manifest from an archive (no extraction).
/// Used for install conflict detection and scan bookkeeping.
pub fn peek_manifest(
    source: &Path,
    app_version: &semver::Version,
    host_api: (u32, u32),
    limits: &Limits,
) -> Result<AddonManifest, AddonError> {
    let mut file = File::open(source)
        .map_err(|e| AddonError::internal(format!("cannot open package: {e}")))?;
    let file_len = file
        .metadata()
        .map_err(|e| AddonError::internal(format!("cannot stat package: {e}")))?
        .len();
    if file_len > limits.max_archive_size {
        return Err(AddonError::archive(
            "size".to_string(),
            "package exceeds the maximum archive size",
        ));
    }
    let kind = detect_kind(&mut file)?;
    match kind {
        ContainerKind::Zip => {
            validate_zip_tail(&mut file, 0, file_len)?;
            file.seek(SeekFrom::Start(0))
                .map_err(|e| AddonError::internal(format!("seek failed: {e}")))?;
            let mut zip = open_zip(file)?;
            read_manifest_from_zip(&mut zip, app_version, host_api)
        }
        ContainerKind::Lzd => {
            validate_lzd_header(&mut file)?;
            validate_zip_tail(&mut file, LZD_HEADER_SIZE, file_len)?;
            let reader = OffsetReader {
                inner: file,
                start: LZD_HEADER_SIZE,
                end: file_len,
                pos: LZD_HEADER_SIZE,
            };
            let mut zip = open_zip(reader)?;
            read_manifest_from_zip(&mut zip, app_version, host_api)
        }
    }
}

/// Read the `.lzdignore` content from an archive (empty string if absent).
/// Used by the provider to honor ignore rules during extraction.
pub fn read_ignore_content(source: &Path, limits: &Limits) -> Result<String, AddonError> {
    let mut file = File::open(source)
        .map_err(|e| AddonError::internal(format!("cannot open package: {e}")))?;
    let file_len = file
        .metadata()
        .map_err(|e| AddonError::internal(format!("cannot stat package: {e}")))?
        .len();
    if file_len > limits.max_archive_size {
        return Err(AddonError::archive(
            "size".to_string(),
            "package exceeds the maximum archive size",
        ));
    }
    let kind = detect_kind(&mut file)?;
    let text = match kind {
        ContainerKind::Zip => {
            validate_zip_tail(&mut file, 0, file_len)?;
            file.seek(SeekFrom::Start(0))
                .map_err(|e| AddonError::internal(format!("seek failed: {e}")))?;
            let mut zip = open_zip(file)?;
            read_ignore_from_zip(&mut zip)
        }
        ContainerKind::Lzd => {
            validate_lzd_header(&mut file)?;
            validate_zip_tail(&mut file, LZD_HEADER_SIZE, file_len)?;
            let reader = OffsetReader {
                inner: file,
                start: LZD_HEADER_SIZE,
                end: file_len,
                pos: LZD_HEADER_SIZE,
            };
            let mut zip = open_zip(reader)?;
            read_ignore_from_zip(&mut zip)
        }
    };
    text
}

fn read_ignore_from_zip<R: Read + Seek>(
    zip: &mut zip::ZipArchive<R>,
) -> Result<String, AddonError> {
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| AddonError::archive("read".to_string(), format!("entry {i}: {e}")))?;
        let name = String::from_utf8(entry.name_raw().to_vec())
            .map_err(|_| AddonError::archive("name".to_string(), "entry name is not valid UTF-8"))?;
        if pathsec::normalize_relative(&name).as_deref() == Ok(".lzdignore") {
            if entry.size() > MAX_IGNORE_FILE {
                return Err(AddonError::archive(
                    "size".to_string(),
                    ".lzdignore exceeds the 64 KiB size limit",
                ));
            }
            let mut buf = Vec::with_capacity(entry.size() as usize);
            entry
                .read_to_end(&mut buf)
                .map_err(|e| AddonError::archive("read".to_string(), format!("cannot read .lzdignore: {e}")))?;
            return Ok(String::from_utf8_lossy(&buf).into_owned());
        }
    }
    Ok(String::new())
}

const MAX_IGNORE_FILE: u64 = 64 * 1024;

fn detect_kind(file: &mut File) -> Result<ContainerKind, AddonError> {
    let mut head = [0u8; 8];
    file.seek(SeekFrom::Start(0))
        .map_err(|e| AddonError::internal(format!("seek failed: {e}")))?;
    let n = file
        .read(&mut head)
        .map_err(|e| AddonError::internal(format!("read failed: {e}")))?;
    if n >= 8 && &head == LZD_MAGIC {
        Ok(ContainerKind::Lzd)
    } else if n >= 4 && (&head[..4] == b"PK\x03\x04" || &head[..4] == b"PK\x05\x06") {
        Ok(ContainerKind::Zip)
    } else {
        Err(AddonError::invalid_addon(None, "unknown package format"))
    }
}

fn validate_lzd_header(file: &mut File) -> Result<(), AddonError> {
    let mut hdr = [0u8; LZD_HEADER_SIZE as usize];
    file.seek(SeekFrom::Start(0))
        .map_err(|e| AddonError::internal(format!("seek failed: {e}")))?;
    file.read_exact(&mut hdr)
        .map_err(|e| AddonError::internal(format!("read failed: {e}")))?;
    if &hdr[..8] != LZD_MAGIC {
        return Err(AddonError::invalid_addon(None, "invalid lzd magic"));
    }
    let version = u32::from_le_bytes(hdr[8..12].try_into().unwrap());
    if version != 1 {
        return Err(AddonError::invalid_addon(
            None,
            format!("unsupported lzd container version {version}"),
        ));
    }
    let flags = u32::from_le_bytes(hdr[12..16].try_into().unwrap());
    if flags & 1 != 0 {
        return Err(AddonError::invalid_addon(
            None,
            "signed packages are not supported by this version",
        ));
    }
    if hdr[16..24] != [0u8; 8] {
        return Err(AddonError::invalid_addon(None, "invalid lzd reserved bytes"));
    }
    Ok(())
}

/// Locate the zip end-of-central-directory record within `[start, end)` and
/// verify it ends exactly at `end` (rejects trailing data).
fn validate_zip_tail(file: &mut File, start: u64, end: u64) -> Result<(), AddonError> {
    const EOCD_MIN: usize = 22;
    const EOCD_MAX_SCAN: u64 = 22 + 65535; // comment <= 65535 bytes

    let total = end - start;
    if total < EOCD_MIN as u64 {
        return Err(AddonError::archive("layout".to_string(), "archive too small"));
    }
    let scan_len = total.min(EOCD_MAX_SCAN) as usize;
    let mut buf = vec![0u8; scan_len];
    file.seek(SeekFrom::Start(end - scan_len as u64))
        .map_err(|e| AddonError::internal(format!("seek failed: {e}")))?;
    file.read_exact(&mut buf)
        .map_err(|e| AddonError::internal(format!("read failed: {e}")))?;

    let mut found = None;
    let mut idx = scan_len;
    while idx >= EOCD_MIN {
        let i = idx - EOCD_MIN;
        if &buf[i..i + 4] == b"PK\x05\x06" {
            found = Some(i);
            break;
        }
        idx -= 1;
    }
    let Some(eocd_rel) = found else {
        return Err(AddonError::archive("layout".to_string(), "zip end-of-central-directory not found"));
    };
    let comment_len = u16::from_le_bytes([buf[eocd_rel + 20], buf[eocd_rel + 21]]) as u64;
    if start + eocd_rel as u64 + EOCD_MIN as u64 + comment_len != end {
        return Err(AddonError::archive(
            "layout".to_string(),
            "trailing data after zip payload",
        ));
    }
    Ok(())
}

fn open_zip<R: Read + Seek>(reader: R) -> Result<zip::ZipArchive<R>, AddonError> {
    zip::ZipArchive::new(reader)
        .map_err(|e| AddonError::archive("read".to_string(), format!("cannot read zip: {e}")))
}

fn read_manifest_from_zip<R: Read + Seek>(
    zip: &mut zip::ZipArchive<R>,
    app_version: &semver::Version,
    host_api: (u32, u32),
) -> Result<AddonManifest, AddonError> {
    let mut found: Option<Vec<u8>> = None;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| AddonError::archive("read".to_string(), format!("entry {i}: {e}")))?;
        let name = String::from_utf8(entry.name_raw().to_vec())
            .map_err(|_| AddonError::archive("name".to_string(), "entry name is not valid UTF-8"))?;
        if pathsec::normalize_relative(&name).as_deref() == Ok("config.toml") {
            if entry.encrypted() {
                return Err(AddonError::archive(
                    "encryption".to_string(),
                    "manifest is encrypted",
                ));
            }
            if entry.size() > crate::manifest::MAX_MANIFEST_SIZE as u64 {
                return Err(AddonError::archive(
                    "size".to_string(),
                    "manifest exceeds the 64 KiB size limit",
                ));
            }
            let mut buf = Vec::with_capacity(entry.size() as usize);
            entry
                .read_to_end(&mut buf)
                .map_err(|e| AddonError::archive("read".to_string(), format!("cannot read manifest: {e}")))?;
            found = Some(buf);
            break;
        }
    }
    match found {
        Some(bytes) => {
            let text = String::from_utf8(bytes).map_err(|_| {
                AddonError::invalid_manifest(None, None, "config.toml is not valid UTF-8")
            })?;
            validate_manifest(&text, app_version, host_api)
        }
        None => Err(AddonError::invalid_manifest(
            None,
            None,
            "package is missing config.toml",
        )),
    }
}

fn extract_entries<R: Read + Seek>(
    zip: &mut zip::ZipArchive<R>,
    dest: &Path,
    ignore: &IgnoreMatcher,
    limits: &Limits,
) -> Result<ExtractionStats, AddonError> {
    fs::create_dir_all(dest)
        .map_err(|e| AddonError::internal(format!("cannot create extraction dir: {e}")))?;

    // Sanity guard: never iterate absurdly many central-directory entries,
    // even if most are ignored (ignore rules must not become a DoS vector).
    if zip.len() > limits.max_entries.saturating_mul(16).max(limits.max_entries) {
        return Err(AddonError::archive(
            "count".to_string(),
            "archive contains too many entries",
        ));
    }

    let mut seen: HashSet<String> = HashSet::new();
    let mut total = 0u64;
    let mut compressed_total = 0u64;
    let mut entries = 0usize;

    for i in 0..zip.len() {
        if entries >= limits.max_entries {
            return Err(AddonError::archive(
                "count".to_string(),
                "archive exceeds the maximum entry count",
            ));
        }
        let mut entry = zip
            .by_index(i)
            .map_err(|e| AddonError::archive("read".to_string(), format!("entry {i}: {e}")))?;
        let name = String::from_utf8(entry.name_raw().to_vec())
            .map_err(|_| AddonError::archive("name".to_string(), "entry name is not valid UTF-8"))?;
        if name.len() > limits.max_name_len {
            return Err(AddonError::archive(
                "name".to_string(),
                "entry name exceeds the length limit",
            ));
        }
        let norm = pathsec::normalize_relative(&name)?;
        if !seen.insert(norm.clone()) {
            return Err(AddonError::archive(
                "name".to_string(),
                format!("duplicate entry '{norm}'"),
            ));
        }
        if entry.encrypted() {
            return Err(AddonError::archive(
                "encryption".to_string(),
                format!("encrypted entry '{norm}' is not supported"),
            ));
        }

        let is_dir = entry.is_dir();
        if ignore.is_ignored(&norm, is_dir) {
            continue;
        }
        entries += 1;

        let target = dest.join(&norm);
        if !pathsec::lexical_within(dest, &target) {
            return Err(AddonError::path_security(&norm, "entry escapes the package root"));
        }

        if is_dir {
            fs::create_dir_all(&target)
                .map_err(|e| AddonError::internal(format!("cannot create dir: {e}")))?;
            continue;
        }

        if entry.is_symlink() {
            extract_symlink_entry(&mut entry, &norm, dest, &target, limits)?;
            continue;
        }

        // Regular file.
        let size = entry.size();
        if size > limits.max_entry_uncompressed {
            return Err(AddonError::archive(
                "size".to_string(),
                format!("entry '{norm}' exceeds the per-entry size limit"),
            ));
        }
        total = total.checked_add(size).ok_or_else(|| {
            AddonError::archive("size".to_string(), "size overflow while extracting")
        })?;
        if total > limits.max_total_uncompressed {
            return Err(AddonError::archive(
                "size".to_string(),
                "package exceeds the total uncompressed size limit",
            ));
        }
        let csize = entry.compressed_size();
        compressed_total = compressed_total.saturating_add(csize);
        // Per-entry and rolling compression ratio checks (archive-bomb guard).
        if csize > 0 && size / csize > limits.max_ratio {
            return Err(AddonError::archive(
                "ratio".to_string(),
                format!("entry '{norm}' has a suspicious compression ratio"),
            ));
        }
        if compressed_total > 0 && total / compressed_total.max(1) > limits.max_ratio {
            return Err(AddonError::archive(
                "ratio".to_string(),
                "package has a suspicious compression ratio",
            ));
        }

        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| AddonError::internal(format!("cannot create dir: {e}")))?;
        }
        let out = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)
            .map_err(|e| AddonError::internal(format!("cannot write entry: {e}")))?;
        let mut writer = CountingWriter::new(out, limits.max_entry_uncompressed);
        let copied = io::copy(&mut entry, &mut writer).map_err(|e| {
            AddonError::archive(
                "read".to_string(),
                format!("failed reading entry '{norm}': {e}"),
            )
        })?;
        if copied != size {
            return Err(AddonError::archive(
                "size".to_string(),
                format!("entry '{norm}' size mismatch (expected {size}, got {copied})"),
            ));
        }
    }

    Ok(ExtractionStats {
        entries,
        total_uncompressed: total,
    })
}

#[cfg(unix)]
fn extract_symlink_entry(
    entry: &mut zip::read::ZipFile<'_>,
    norm: &str,
    dest: &Path,
    target: &Path,
    limits: &Limits,
) -> Result<(), AddonError> {
    if entry.size() > MAX_SYMLINK_TARGET {
        return Err(AddonError::path_security(norm, "symlink target too long"));
    }
    let mut buf = Vec::new();
    entry
        .read_to_end(&mut buf)
        .map_err(|e| AddonError::archive("read".to_string(), format!("cannot read symlink: {e}")))?;
    if buf.len() > MAX_SYMLINK_TARGET as usize {
        return Err(AddonError::path_security(norm, "symlink target too long"));
    }
    let target_str = String::from_utf8(buf)
        .map_err(|_| AddonError::archive("symlink".to_string(), "symlink target is not UTF-8"))?;
    let entry_dir = target.parent().unwrap_or_else(|| Path::new(""));
    let resolved = pathsec::validate_symlink_target(entry_dir, dest, &target_str)?;
    if target.exists() || std::fs::symlink_metadata(target).is_ok() {
        return Err(AddonError::archive(
            "name".to_string(),
            format!("entry '{norm}' collides with an existing path"),
        ));
    }
    std::os::unix::fs::symlink(&resolved, target)
        .map_err(|e| AddonError::internal(format!("cannot create symlink: {e}")))?;
    let _ = limits;
    Ok(())
}

#[cfg(not(unix))]
fn extract_symlink_entry(
    _entry: &mut zip::read::ZipFile<'_>,
    _norm: &str,
    _dest: &Path,
    _target: &Path,
    _limits: &Limits,
) -> Result<(), AddonError> {
    // Non-Unix platforms cannot create symlinks portably/privilege-free;
    // symlink entries are skipped (never followed, never a security issue).
    Ok(())
}

/// A writer that enforces a byte cap; used to keep per-entry memory bounded
/// while streaming to disk.
struct CountingWriter<W: Write> {
    inner: W,
    limit: u64,
    written: u64,
}

impl<W: Write> CountingWriter<W> {
    fn new(inner: W, limit: u64) -> Self {
        Self {
            inner,
            limit,
            written: 0,
        }
    }
}

impl<W: Write> Write for CountingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.written = self.written.saturating_add(buf.len() as u64);
        if self.written > self.limit {
            return Err(io::Error::other("entry size limit exceeded"));
        }
        self.inner.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

/// Recursively compute the total size of non-ignored regular files under
/// `root` (used for `size_bytes` in descriptors).
pub fn content_size(root: &Path, ignore: &IgnoreMatcher) -> u64 {
    use rayon::prelude::*;

    let entries: Vec<_> = walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(Result::ok)
        .collect();
    entries
        .par_iter()
        .map(|e| {
            if e.file_type().is_symlink() || !e.file_type().is_file() {
                return 0u64;
            }
            let rel = e
                .path()
                .strip_prefix(root)
                .unwrap_or(e.path())
                .to_string_lossy()
                .replace('\\', "/");
            if ignore.is_ignored(&rel, false) {
                return 0u64;
            }
            e.metadata().map(|m| m.len()).unwrap_or(0)
        })
        .reduce(|| 0u64, u64::saturating_add)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ignore::IgnoreMatcher;
    use tempfile::TempDir;

    fn write_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name, data) in entries {
            zip.start_file(*name, opts).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.finish().unwrap();
    }

    fn write_dir_zip(path: &Path, entries: &[(&str, &[u8])]) {
        let file = File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name, data) in entries {
            if name.ends_with('/') {
                zip.add_directory(*name, opts).unwrap();
            } else {
                zip.start_file(*name, opts).unwrap();
                zip.write_all(data).unwrap();
            }
        }
        zip.finish().unwrap();
    }

    /// Write a minimal STORE-method zip from raw parts so we can set fields the
    /// `ZipWriter` API cannot express (Unix symlink modes, duplicate raw names).
    /// `entries` is `(name, data, unix_mode)` where `None` means "no unix mode".
    fn write_raw_zip(path: &Path, entries: &[(&str, &[u8], Option<u32>)]) {
        let mut out: Vec<u8> = Vec::new();
        let mut central: Vec<u8> = Vec::new();
        let mut offset: u32 = 0;
        for (name, data, unix_mode) in entries {
            let name_b = name.as_bytes();
            let crc = crc32fast::hash(data);
            let size = data.len() as u32;
            let unix_attrs = unix_mode.unwrap_or(0) << 16;
            // Local file header.
            let mut lh = Vec::new();
            lh.extend_from_slice(&0x0403_4b50u32.to_le_bytes());
            lh.extend_from_slice(&20u16.to_le_bytes()); // version needed
            lh.extend_from_slice(&0u16.to_le_bytes()); // flags
            lh.extend_from_slice(&0u16.to_le_bytes()); // method: store
            lh.extend_from_slice(&0u16.to_le_bytes()); // mod time
            lh.extend_from_slice(&0x21u16.to_le_bytes()); // mod date
            lh.extend_from_slice(&crc.to_le_bytes());
            lh.extend_from_slice(&size.to_le_bytes());
            lh.extend_from_slice(&size.to_le_bytes());
            lh.extend_from_slice(&(name_b.len() as u16).to_le_bytes());
            lh.extend_from_slice(&0u16.to_le_bytes()); // extra len
            lh.extend_from_slice(name_b);
            out.extend_from_slice(&lh);
            out.extend_from_slice(data);
            offset += lh.len() as u32 + size;
            // Central directory header.
            let mut ch = Vec::new();
            ch.extend_from_slice(&0x0201_4b50u32.to_le_bytes());
            ch.extend_from_slice(&((3u16 << 8) | 20u16).to_le_bytes()); // made by: Unix 3.0
            ch.extend_from_slice(&20u16.to_le_bytes()); // version needed
            ch.extend_from_slice(&0u16.to_le_bytes()); // flags
            ch.extend_from_slice(&0u16.to_le_bytes()); // method: store
            ch.extend_from_slice(&0u16.to_le_bytes()); // mod time
            ch.extend_from_slice(&0x21u16.to_le_bytes()); // mod date
            ch.extend_from_slice(&crc.to_le_bytes());
            ch.extend_from_slice(&size.to_le_bytes());
            ch.extend_from_slice(&size.to_le_bytes());
            ch.extend_from_slice(&(name_b.len() as u16).to_le_bytes());
            ch.extend_from_slice(&0u16.to_le_bytes()); // extra len
            ch.extend_from_slice(&0u16.to_le_bytes()); // comment len
            ch.extend_from_slice(&0u16.to_le_bytes()); // disk
            ch.extend_from_slice(&0u16.to_le_bytes()); // internal attrs
            ch.extend_from_slice(&unix_attrs.to_le_bytes());
            ch.extend_from_slice(&(offset - lh.len() as u32 - size).to_le_bytes());
            ch.extend_from_slice(name_b);
            central.extend_from_slice(&ch);
        }
        let cd_offset = offset;
        out.extend_from_slice(&central);
        let cd_size = central.len() as u32;
        // End of central directory.
        out.extend_from_slice(&0x0605_4b50u32.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // disk
        out.extend_from_slice(&0u16.to_le_bytes()); // cd disk
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&(entries.len() as u16).to_le_bytes());
        out.extend_from_slice(&cd_size.to_le_bytes());
        out.extend_from_slice(&cd_offset.to_le_bytes());
        out.extend_from_slice(&0u16.to_le_bytes()); // comment len
        fs::write(path, out).unwrap();
    }

    #[test]
    fn extracts_valid_zip() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("pkg.zip");
        write_zip(
            &src,
            &[
                ("config.toml", b"id = \"a.b\"\nname = \"A\"\nversion = \"1.0.0\"\nentry = \"src/main.lua\"\n"),
                ("src/main.lua", b"return {}"),
                ("assets/128.ico", b"icon"),
            ],
        );
        let dest = tmp.path().join("out");
        let stats = extract(&src, &dest, &IgnoreMatcher::from_lines(""), &Limits::default()).unwrap();
        assert_eq!(stats.entries, 3);
        assert!(dest.join("config.toml").is_file());
        assert!(dest.join("src/main.lua").is_file());
        assert!(dest.join("assets/128.ico").is_file());
    }

    #[test]
    fn extract_rejects_traversal_entries() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("evil.zip");
        write_zip(&src, &[("../escape.txt", b"pwn"), ("ok.txt", b"x")]);
        let dest = tmp.path().join("out");
        let e = extract(&src, &dest, &IgnoreMatcher::from_lines(""), &Limits::default()).unwrap_err();
        assert_eq!(e.code(), "PathSecurityViolation");
        assert!(!tmp.path().join("escape.txt").exists());
    }

    #[test]
    fn extract_rejects_absolute_entries() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("evil.zip");
        write_zip(&src, &[("/etc/passwd", b"x")]);
        let dest = tmp.path().join("out");
        let e = extract(&src, &dest, &IgnoreMatcher::from_lines(""), &Limits::default()).unwrap_err();
        assert_eq!(e.code(), "PathSecurityViolation");
    }

    #[test]
    fn extract_applies_ignore_rules() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("pkg.zip");
        write_zip(
            &src,
            &[
                ("config.toml", b"x"),
                (".git/config", b"secret"),
                ("keep.txt", b"k"),
            ],
        );
        let dest = tmp.path().join("out");
        extract(&src, &dest, &IgnoreMatcher::from_lines(""), &Limits::default()).unwrap();
        assert!(dest.join("config.toml").is_file());
        assert!(!dest.join(".git/config").exists());
        assert!(dest.join("keep.txt").is_file());
    }

    #[test]
    fn extract_rejects_duplicate_entries() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("dup.zip");
        // The writer rejects identical *raw* names, so use two spellings that
        // normalize to the same path (x.txt and ./x.txt) to build a real
        // duplicate-normalized archive.
        write_raw_zip(&src, &[("x.txt", b"a", None), ("./x.txt", b"b", None)]);
        let dest = tmp.path().join("out");
        let e = extract(&src, &dest, &IgnoreMatcher::from_lines(""), &Limits::default()).unwrap_err();
        assert_eq!(e.code(), "ArchiveError");
        assert!(e.to_string().contains("duplicate"));
    }

    #[test]
    fn extract_rejects_entry_count_explosion() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("many.zip");
        let file = File::create(&src).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let opts = zip::write::SimpleFileOptions::default();
        let limit = Limits::default();
        for i in 0..limit.max_entries + 1 {
            zip.start_file(format!("f{i}.txt"), opts).unwrap();
            zip.write_all(b"x").unwrap();
        }
        zip.finish().unwrap();
        let dest = tmp.path().join("out");
        let e = extract(&src, &dest, &IgnoreMatcher::from_lines(""), &limit).unwrap_err();
        assert_eq!(e.code(), "ArchiveError");
    }

    #[test]
    fn extract_rejects_ratio_bomb() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("bomb.zip");
        let data = vec![0u8; 64 * 1024 * 1024]; // highly compressible
        write_zip(&src, &[("big.bin", &data)]);
        let dest = tmp.path().join("out");
        let e = extract(&src, &dest, &IgnoreMatcher::from_lines(""), &Limits::default()).unwrap_err();
        assert_eq!(e.code(), "ArchiveError");
        assert!(e.to_string().contains("ratio"));
        assert!(!dest.exists() || dest.read_dir().map(|mut d| d.next().is_none()).unwrap_or(true));
    }

    #[test]
    fn lzd_container_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("pkg.lzd");
        // Build a zip payload, then wrap it with the container header.
        let payload = tmp.path().join("payload.zip");
        write_zip(
            &payload,
            &[
                ("config.toml", b"id = \"a.b\"\nname = \"A\"\nversion = \"1.0.0\"\nentry = \"main.lua\"\n"),
                ("main.lua", b"return {}"),
            ],
        );
        let mut out = File::create(&src).unwrap();
        out.write_all(LZD_MAGIC).unwrap();
        out.write_all(&1u32.to_le_bytes()).unwrap();
        out.write_all(&0u32.to_le_bytes()).unwrap();
        out.write_all(&[0u8; 8]).unwrap();
        let payload_bytes = std::fs::read(&payload).unwrap();
        out.write_all(&payload_bytes).unwrap();
        out.flush().unwrap();

        let dest = tmp.path().join("out");
        let stats = extract(&src, &dest, &IgnoreMatcher::from_lines(""), &Limits::default()).unwrap();
        assert_eq!(stats.entries, 2);
        assert!(dest.join("main.lua").is_file());
    }

    #[test]
    fn lzd_rejects_signed_flag() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("signed.lzd");
        let payload = tmp.path().join("p.zip");
        write_zip(&payload, &[("config.toml", b"x")]);
        let mut out = File::create(&src).unwrap();
        out.write_all(LZD_MAGIC).unwrap();
        out.write_all(&1u32.to_le_bytes()).unwrap();
        out.write_all(&1u32.to_le_bytes()).unwrap(); // signed bit set
        out.write_all(&[0u8; 8]).unwrap();
        let pb = std::fs::read(&payload).unwrap();
        out.write_all(&pb).unwrap();
        out.flush().unwrap();
        let dest = tmp.path().join("out");
        let e = extract(&src, &dest, &IgnoreMatcher::from_lines(""), &Limits::default()).unwrap_err();
        assert_eq!(e.code(), "InvalidAddon");
        assert!(e.to_string().contains("signed"));
    }

    #[test]
    fn lzd_rejects_trailing_data() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("trailing.lzd");
        let payload = tmp.path().join("p.zip");
        write_zip(&payload, &[("config.toml", b"x")]);
        let mut out = File::create(&src).unwrap();
        out.write_all(LZD_MAGIC).unwrap();
        out.write_all(&1u32.to_le_bytes()).unwrap();
        out.write_all(&0u32.to_le_bytes()).unwrap();
        out.write_all(&[0u8; 8]).unwrap();
        let pb = std::fs::read(&payload).unwrap();
        out.write_all(&pb).unwrap();
        out.write_all(b"EXTRA").unwrap();
        out.flush().unwrap();
        let dest = tmp.path().join("out");
        let e = extract(&src, &dest, &IgnoreMatcher::from_lines(""), &Limits::default()).unwrap_err();
        assert_eq!(e.code(), "ArchiveError");
        assert!(e.to_string().contains("trailing"));
    }

    #[test]
    fn symlink_within_root_is_extracted() {
        #[cfg(unix)]
        {
            let tmp = TempDir::new().unwrap();
            let src = tmp.path().join("links.zip");
            // Raw fixture: only raw bytes preserve the S_IFLNK (0o120000) type
            // bit in external_attributes; ZipWriter::unix_permissions masks it
            // to 0o777 and the entry would extract as a regular file.
            write_raw_zip(
                &src,
                &[
                    ("real.txt", b"data", Some(0o644)),
                    ("link", b"real.txt", Some(0o120777)),
                ],
            );

            let dest = tmp.path().join("out");
            extract(&src, &dest, &IgnoreMatcher::from_lines(""), &Limits::default()).unwrap();
            let link = dest.join("link");
            let meta = std::fs::symlink_metadata(&link).unwrap();
            assert!(meta.file_type().is_symlink());
            assert_eq!(std::fs::read_to_string(&link).unwrap(), "data");
        }
    }

    #[test]
    fn symlink_escaping_root_is_rejected() {
        #[cfg(unix)]
        {
            let tmp = TempDir::new().unwrap();
            let src = tmp.path().join("evillink.zip");
            write_raw_zip(&src, &[("link", b"../../etc/passwd", Some(0o120777))]);
            let dest = tmp.path().join("out");
            let e = extract(&src, &dest, &IgnoreMatcher::from_lines(""), &Limits::default()).unwrap_err();
            assert_eq!(e.code(), "PathSecurityViolation");
        }
    }

    #[test]
    fn peek_manifest_reads_config() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("pkg.zip");
        write_zip(
            &src,
            &[
                ("config.toml", b"id = \"a.b\"\nname = \"A\"\nversion = \"1.0.0\"\nentry = \"main.lua\"\n"),
                ("main.lua", b"return {}"),
            ],
        );
        let app = semver::Version::new(0, 3, 0);
        let m = peek_manifest(&src, &app, crate::manifest::HOST_API_VERSION, &Limits::default()).unwrap();
        assert_eq!(m.id, "a.b");
        assert_eq!(m.entry, "main.lua");
    }

    #[test]
    fn peek_manifest_missing_config_is_invalid() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("pkg.zip");
        write_zip(&src, &[("main.lua", b"return {}")]);
        let app = semver::Version::new(0, 3, 0);
        let e = peek_manifest(&src, &app, crate::manifest::HOST_API_VERSION, &Limits::default()).unwrap_err();
        assert_eq!(e.code(), "InvalidManifest");
    }

    #[test]
    fn content_size_counts_only_non_ignored() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().join("pkg");
        fs::create_dir_all(root.join("sub")).unwrap();
        fs::create_dir_all(root.join(".git")).unwrap();
        fs::write(root.join("a.txt"), b"12345").unwrap();
        fs::write(root.join(".git/config"), b"1234567890").unwrap();
        fs::write(root.join("sub/b.txt"), b"123").unwrap();
        let m = IgnoreMatcher::from_lines("");
        assert_eq!(content_size(&root, &m), 8);
    }

    #[test]
    fn normalized_zip_roundtrip_uses_write_dir() {
        let tmp = TempDir::new().unwrap();
        let src = tmp.path().join("dirs.zip");
        write_dir_zip(
            &src,
            &[("src/", &[]), ("src/main.lua", b"return {}"), ("assets/128.ico", b"i")],
        );
        let dest = tmp.path().join("out");
        extract(&src, &dest, &IgnoreMatcher::from_lines(""), &Limits::default()).unwrap();
        assert!(dest.join("src/main.lua").is_file());
        assert!(dest.join("assets/128.ico").is_file());
    }
}
