//! Path security: normalization, traversal rejection, symlink policy, and
//! canonical containment (ADDON_SPEC §8.2).

use std::path::{Component, Path, PathBuf};

use crate::error::AddonError;

/// Maximum length (UTF-8 bytes) of any normalized package-relative path.
pub const MAX_PATH_LEN: usize = 1024;

/// Hard cap for a single asset read returned over the FFI.
pub const MAX_ASSET_READ: u64 = 64 * 1024 * 1024;

/// Normalize an archive/package-relative path into canonical relative form.
///
/// Rejects (as `PathSecurityViolation`):
/// - empty paths and paths that normalize to the root
/// - NUL bytes
/// - absolute paths (leading `/`, Windows drive letters)
/// - any path component equal to `.` (except a single leading `./`) or `..`
/// - backslash as a separator on all platforms
/// - paths longer than [`MAX_PATH_LEN`] bytes
///
/// Duplicated slashes are collapsed; a single leading `./` is stripped.
pub fn normalize_relative(input: &str) -> Result<String, AddonError> {
    if input.is_empty() {
        return Err(AddonError::path_security("", "empty path"));
    }
    if input.as_bytes().contains(&0) {
        return Err(AddonError::path_security(input, "path contains NUL byte"));
    }
    if input.starts_with('/') {
        return Err(AddonError::path_security(
            input,
            "absolute paths are not allowed",
        ));
    }
    if input.starts_with('\\') {
        return Err(AddonError::path_security(
            input,
            "backslash paths are not allowed",
        ));
    }
    let bytes = input.as_bytes();
    if bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':' {
        return Err(AddonError::path_security(
            input,
            "drive-letter paths are not allowed",
        ));
    }
    if input.contains('\\') {
        return Err(AddonError::path_security(
            input,
            "backslash paths are not allowed",
        ));
    }

    let mut parts: Vec<&str> = Vec::new();
    for (idx, comp) in input.split('/').enumerate() {
        match comp {
            "" => {
                // Collapse duplicated slashes and a trailing slash.
            }
            "." => {
                // A single leading "./" is stripped; anything else is rejected.
                if idx != 0 {
                    return Err(AddonError::path_security(
                        input,
                        "path component '.' is not allowed",
                    ));
                }
            }
            ".." => {
                return Err(AddonError::path_security(
                    input,
                    "path traversal ('..') is not allowed",
                ));
            }
            _ => parts.push(comp),
        }
    }

    if parts.is_empty() {
        return Err(AddonError::path_security(
            input,
            "path normalizes to the package root",
        ));
    }
    let joined = parts.join("/");
    if joined.len() > MAX_PATH_LEN {
        return Err(AddonError::path_security(input, "path is too long"));
    }
    Ok(joined)
}

/// Lexical containment check: is `candidate` inside `root` (by components)?
///
/// This is a pure path check used before creating files (extraction, symlink
/// targets). For reads, prefer [`canonical_within`], which consults the real
/// filesystem.
pub fn lexical_within(root: &Path, candidate: &Path) -> bool {
    let root_components: Vec<Component<'_>> = root.components().collect();
    let cand_components: Vec<Component<'_>> = candidate.components().collect();
    cand_components.len() >= root_components.len() && cand_components.starts_with(&root_components)
}

/// Filesystem containment check via canonicalization.
///
/// Resolves `candidate` (canonicalizing its existing parent) and verifies the
/// resolved path starts with the canonical `root`. This closes TOCTOU and
/// symlink-race vectors at read time.
pub fn canonical_within(root: &Path, candidate: &Path) -> Result<(), AddonError> {
    let root_canon = std::fs::canonicalize(root)
        .map_err(|e| AddonError::internal(format!("cannot canonicalize package root: {e}")))?;

    let parent = candidate.parent().unwrap_or_else(|| Path::new(""));
    let file_name = candidate.file_name();
    let parent_canon = if parent.as_os_str().is_empty() {
        std::fs::canonicalize(candidate)
            .map_err(|e| AddonError::internal(format!("cannot canonicalize path: {e}")))?
    } else {
        std::fs::canonicalize(parent)
            .map_err(|e| AddonError::internal(format!("cannot canonicalize path: {e}")))?
    };

    let resolved = match file_name {
        Some(f) => parent_canon.join(f),
        None => parent_canon,
    };

    if resolved.starts_with(&root_canon) {
        Ok(())
    } else {
        Err(AddonError::path_security(
            candidate.display().to_string(),
            "path escapes the package root",
        ))
    }
}

/// Validate a symlink target at extraction time.
///
/// The target must be relative, must not traverse out of the archive (after
/// lexically joining with `entry_dir`), and must not be absolute or use
/// backslashes. Returns the resolved in-package path.
pub fn validate_symlink_target(
    entry_dir: &Path,
    root: &Path,
    target: &str,
) -> Result<PathBuf, AddonError> {
    if target.starts_with('/') || target.starts_with('\\') {
        return Err(AddonError::path_security(
            target,
            "symlink target must be relative",
        ));
    }
    if target.contains('\0') {
        return Err(AddonError::path_security(
            target,
            "symlink target contains NUL",
        ));
    }
    if target.contains('\\') {
        return Err(AddonError::path_security(
            target,
            "symlink target uses backslashes",
        ));
    }

    let joined = if entry_dir.as_os_str().is_empty() {
        PathBuf::from(target)
    } else {
        entry_dir.join(target)
    };

    let mut parts: Vec<PathBuf> = Vec::new();
    for comp in joined.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                if parts.pop().is_none() {
                    return Err(AddonError::path_security(
                        target,
                        "symlink target escapes the package root",
                    ));
                }
            }
            Component::Normal(c) => parts.push(PathBuf::from(c)),
            Component::RootDir | Component::Prefix(_) => {
                // `joined` is derived from `entry_dir` (an absolute extraction
                // directory) joined with a relative target, so it legitimately
                // starts with a root component. Preserve it as the path's
                // anchor; a second root mid-path cannot happen for validated
                // relative targets and is rejected defensively.
                if parts.is_empty() {
                    parts.push(PathBuf::from(comp.as_os_str()));
                } else {
                    return Err(AddonError::path_security(
                        target,
                        "symlink target must be relative",
                    ));
                }
            }
        }
    }

    let normalized: PathBuf = parts.iter().collect();
    // The resolved path must be strictly inside `root`: pointing at the root
    // itself (`..` from the top-level entry dir) is also rejected.
    if lexical_within(root, &normalized) && normalized != *root {
        Ok(normalized)
    } else {
        Err(AddonError::path_security(
            target,
            "symlink target escapes the package root",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normalize_ok(input: &str) -> String {
        normalize_relative(input).unwrap()
    }

    #[test]
    fn normalizes_simple_paths() {
        assert_eq!(normalize_ok("config.toml"), "config.toml");
        assert_eq!(normalize_ok("src/main.lua"), "src/main.lua");
        assert_eq!(normalize_ok("./src/main.lua"), "src/main.lua");
        assert_eq!(normalize_ok("assets/128.ico"), "assets/128.ico");
    }

    #[test]
    fn collapses_duplicate_slashes() {
        assert_eq!(normalize_ok("a//b"), "a/b");
        assert_eq!(normalize_ok("a/b/"), "a/b");
    }

    #[test]
    fn rejects_traversal() {
        for p in [
            "../x",
            "a/../x",
            "..",
            "a/../../x",
            "./../x",
            "a/b/../../../x",
        ] {
            let r = normalize_relative(p);
            assert!(r.is_err(), "{p} should be rejected");
            assert_eq!(r.unwrap_err().code(), "PathSecurityViolation");
        }
    }

    #[test]
    fn rejects_dot_components() {
        for p in ["a/./b", "././x"] {
            let r = normalize_relative(p);
            assert!(r.is_err(), "{p} should be rejected");
        }
    }

    #[test]
    fn rejects_absolute_and_drive_paths() {
        for p in [
            "/etc/passwd",
            "/",
            "C:\\x",
            "C:/x",
            "c:/windows",
            "\\server\\share",
        ] {
            let r = normalize_relative(p);
            assert!(r.is_err(), "{p} should be rejected");
        }
    }

    #[test]
    fn rejects_nul_and_backslash() {
        assert!(normalize_relative("a\0b").is_err());
        assert!(normalize_relative("a\\b").is_err());
        assert!(normalize_relative("a/b\\c").is_err());
    }

    #[test]
    fn rejects_empty_and_root_normalizing() {
        assert!(normalize_relative("").is_err());
        assert!(normalize_relative(".").is_err());
        assert!(normalize_relative("./").is_err());
    }

    #[test]
    fn rejects_overlong_paths() {
        let long = "a".repeat(MAX_PATH_LEN + 1);
        assert!(normalize_relative(&long).is_err());
    }

    #[test]
    fn lexical_containment() {
        let root = Path::new("/pkg");
        assert!(lexical_within(root, Path::new("/pkg/src/main.lua")));
        assert!(lexical_within(root, Path::new("/pkg")));
        assert!(!lexical_within(root, Path::new("/pkg2/x")));
        assert!(!lexical_within(root, Path::new("/other/pkg")));
    }

    #[test]
    fn symlink_target_validation() {
        let root = Path::new("/pkg");
        let entry_dir = Path::new("/pkg/assets");
        // Inside -> allowed
        let ok = validate_symlink_target(entry_dir, root, "../src/main.lua").unwrap();
        assert_eq!(ok, Path::new("/pkg/src/main.lua"));
        // Escapes -> rejected
        for bad in [
            "../../etc/passwd",
            "/etc/passwd",
            "\\\\server\\share",
            "..\\..\\win",
            "..",
        ] {
            assert!(
                validate_symlink_target(entry_dir, root, bad).is_err(),
                "{bad} should be rejected"
            );
        }
    }
}
