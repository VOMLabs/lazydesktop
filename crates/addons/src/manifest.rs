//! `config.toml` schema, parsing, and validation (ADDON_SPEC §6).

use semver::Version;
use serde::Deserialize;

use crate::error::AddonError;
use crate::pathsec;

/// Highest manifest schema this build understands.
pub const SUPPORTED_SCHEMA_VERSION: u32 = 1;

/// Host addon API version exposed by the C++ Lua host.
pub const HOST_API_VERSION: (u32, u32) = (1, 0);

/// Maximum manifest size in bytes.
pub const MAX_MANIFEST_SIZE: usize = 64 * 1024;

const MAX_NAME_LEN: usize = 128;
const MAX_DESCRIPTION_LEN: usize = 512;
const MAX_AUTHOR_LEN: usize = 128;
const MAX_LICENSE_LEN: usize = 64;

/// Validated addon manifest. All fields are type-checked and constrained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddonManifest {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub version: Version,
    pub entry: String,
    pub description: Option<String>,
    pub author: Option<String>,
    pub license: Option<String>,
    pub min_app_version: Option<Version>,
    pub max_app_version: Option<Version>,
    pub api_version: (u32, u32),
}

/// Raw TOML shape. Unknown fields are tolerated (serde ignores them).
#[derive(Debug, Deserialize)]
struct RawManifest {
    schema_version: Option<u32>,
    id: Option<String>,
    name: Option<String>,
    version: Option<String>,
    entry: Option<String>,
    description: Option<String>,
    author: Option<String>,
    license: Option<String>,
    min_app_version: Option<String>,
    max_app_version: Option<String>,
    api_version: Option<String>,
}

/// Validate a manifest string against the schema (ADDON_SPEC §6.2).
///
/// `app_version` is the host application version; `host_api` is the host's
/// addon API `(major, minor)`.
pub fn validate_manifest(
    text: &str,
    app_version: &Version,
    host_api: (u32, u32),
) -> Result<AddonManifest, AddonError> {
    if text.len() > MAX_MANIFEST_SIZE {
        return Err(AddonError::invalid_manifest(
            None,
            Some("config.toml".to_string()),
            "manifest exceeds the 64 KiB size limit",
        ));
    }

    let raw: RawManifest = toml::from_str(text).map_err(|e| {
        AddonError::invalid_manifest(None, None, format!("config.toml is not valid TOML: {e}"))
    })?;

    let schema_version = raw.schema_version.unwrap_or(1);
    if schema_version > SUPPORTED_SCHEMA_VERSION {
        return Err(AddonError::invalid_manifest(
            None,
            Some("schema_version".to_string()),
            format!(
                "schema_version {schema_version} is not supported (max {SUPPORTED_SCHEMA_VERSION})"
            ),
        ));
    }

    let id = match raw.id {
        Some(id) => {
            if !validate_addon_id(&id) {
                return Err(AddonError::invalid_manifest(
                    Some(id.clone()),
                    Some("id".to_string()),
                    "field 'id' must match ^[a-z0-9]+(\\.[a-z0-9]+)+$ and be at most 128 chars",
                ));
            }
            id
        }
        None => {
            return Err(AddonError::invalid_manifest(
                None,
                Some("id".to_string()),
                "missing required field 'id'",
            ))
        }
    };

    let name = match raw.name {
        Some(name) => {
            let name = name.trim().to_string();
            if name.is_empty() || name.len() > MAX_NAME_LEN {
                return Err(AddonError::invalid_manifest(
                    Some(id.clone()),
                    Some("name".to_string()),
                    "field 'name' must be 1..=128 characters",
                ));
            }
            name
        }
        None => {
            return Err(AddonError::invalid_manifest(
                Some(id.clone()),
                Some("name".to_string()),
                "missing required field 'name'",
            ))
        }
    };

    let version_str = match raw.version {
        Some(v) => v,
        None => {
            return Err(AddonError::invalid_manifest(
                Some(id.clone()),
                Some("version".to_string()),
                "missing required field 'version'",
            ))
        }
    };
    let version = semver::Version::parse(&version_str).map_err(|_| {
        AddonError::invalid_manifest(
            Some(id.clone()),
            Some("version".to_string()),
            format!("field 'version' is not valid semver: {version_str:?}"),
        )
    })?;

    let entry_raw = match raw.entry {
        Some(e) => e,
        None => {
            return Err(AddonError::invalid_manifest(
                Some(id.clone()),
                Some("entry".to_string()),
                "missing required field 'entry'",
            ))
        }
    };
    let entry = pathsec::normalize_relative(&entry_raw).map_err(|_| {
        AddonError::invalid_manifest(
            Some(id.clone()),
            Some("entry".to_string()),
            "field 'entry' must be a relative path inside the package",
        )
    })?;
    if !entry.ends_with(".lua") {
        return Err(AddonError::invalid_manifest(
            Some(id.clone()),
            Some("entry".to_string()),
            "field 'entry' must end in '.lua'",
        ));
    }

    if let Some(d) = &raw.description {
        if d.len() > MAX_DESCRIPTION_LEN {
            return Err(AddonError::invalid_manifest(
                Some(id.clone()),
                Some("description".to_string()),
                "field 'description' exceeds 512 characters",
            ));
        }
    }
    if let Some(a) = &raw.author {
        if a.len() > MAX_AUTHOR_LEN {
            return Err(AddonError::invalid_manifest(
                Some(id.clone()),
                Some("author".to_string()),
                "field 'author' exceeds 128 characters",
            ));
        }
    }
    if let Some(l) = &raw.license {
        if l.len() > MAX_LICENSE_LEN {
            return Err(AddonError::invalid_manifest(
                Some(id.clone()),
                Some("license".to_string()),
                "field 'license' exceeds 64 characters",
            ));
        }
    }

    let min_app_version = parse_optional_version(&raw.min_app_version, &id, "min_app_version")?;
    let max_app_version = parse_optional_version(&raw.max_app_version, &id, "max_app_version")?;
    if let Some(min) = &min_app_version {
        if min > app_version {
            return Err(AddonError::invalid_manifest(
                Some(id.clone()),
                Some("min_app_version".to_string()),
                format!("addon requires app version >= {min}, host is {app_version}"),
            ));
        }
    }
    if let Some(max) = &max_app_version {
        if max < app_version {
            return Err(AddonError::invalid_manifest(
                Some(id.clone()),
                Some("max_app_version".to_string()),
                format!("addon requires app version <= {max}, host is {app_version}"),
            ));
        }
    }

    let api_version = match &raw.api_version {
        Some(s) => parse_api_version(s, &id)?,
        None => HOST_API_VERSION,
    };
    if api_version.0 != host_api.0 {
        return Err(AddonError::invalid_manifest(
            Some(id.clone()),
            Some("api_version".to_string()),
            format!(
                "addon targets addon API major {}, host provides major {}",
                api_version.0, host_api.0
            ),
        ));
    }
    if api_version.1 > host_api.1 {
        return Err(AddonError::invalid_manifest(
            Some(id.clone()),
            Some("api_version".to_string()),
            format!(
                "addon requires addon API {}.{}, host provides {}.{}",
                api_version.0, api_version.1, host_api.0, host_api.1
            ),
        ));
    }

    Ok(AddonManifest {
        schema_version,
        id,
        name,
        version,
        entry,
        description: raw.description,
        author: raw.author,
        license: raw.license,
        min_app_version,
        max_app_version,
        api_version,
    })
}

/// Validate the reverse-DNS addon id shape `^[a-z0-9]+(\.[a-z0-9]+)+$`,
/// at most 128 bytes total, at most 64 bytes per segment, and at least one
/// ASCII letter per segment (all-numeric segments are rejected so ids cannot
/// be mistaken for versions).
pub fn validate_addon_id(id: &str) -> bool {
    if id.is_empty() || id.len() > 128 {
        return false;
    }
    const MAX_SEGMENT_LEN: usize = 64;
    let bytes = id.as_bytes();
    let mut segments = 0usize;
    let mut segment_len = 0usize;
    let mut segment_has_letter = false;
    for &b in bytes {
        if b == b'.' {
            if segment_len == 0 || !segment_has_letter {
                return false;
            }
            segments += 1;
            segment_len = 0;
            segment_has_letter = false;
        } else if b.is_ascii_lowercase() || b.is_ascii_digit() {
            segment_len += 1;
            if segment_len > MAX_SEGMENT_LEN {
                return false;
            }
            segment_has_letter |= b.is_ascii_lowercase();
        } else {
            return false;
        }
    }
    if segment_len == 0 || !segment_has_letter {
        return false;
    }
    segments + 1 >= 2
}

fn parse_optional_version(
    v: &Option<String>,
    id: &str,
    field: &str,
) -> Result<Option<Version>, AddonError> {
    match v {
        None => Ok(None),
        Some(s) => semver::Version::parse(s)
            .map(Some)
            .map_err(|_| {
                AddonError::invalid_manifest(
                    Some(id.to_string()),
                    Some(field.to_string()),
                    format!("field '{field}' is not valid semver: {s:?}"),
                )
            }),
    }
}

fn parse_api_version(s: &str, id: &str) -> Result<(u32, u32), AddonError> {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 2 {
        return Err(AddonError::invalid_manifest(
            Some(id.to_string()),
            Some("api_version".to_string()),
            "field 'api_version' must be MAJOR.MINOR",
        ));
    }
    let major = parts[0].parse::<u32>().map_err(|_| {
        AddonError::invalid_manifest(
            Some(id.to_string()),
            Some("api_version".to_string()),
            "field 'api_version' major is not a valid number",
        )
    })?;
    let minor = parts[1].parse::<u32>().map_err(|_| {
        AddonError::invalid_manifest(
            Some(id.to_string()),
            Some("api_version".to_string()),
            "field 'api_version' minor is not a valid number",
        )
    })?;
    Ok((major, minor))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn app() -> Version {
        Version::from_str("0.3.0").unwrap()
    }

    fn valid_toml() -> String {
        r#"
id = "example.addon"
name = "Example Addon"
version = "1.0.0"
entry = "src/main.lua"
description = "Demonstrates the addon contract."
author = "LazyDesktop Team"
license = "MIT"
api_version = "1.0"
min_app_version = "0.3.0"
"#
        .to_string()
    }

    fn validate(s: &str) -> Result<AddonManifest, AddonError> {
        validate_manifest(s, &app(), HOST_API_VERSION)
    }

    #[test]
    fn accepts_valid_manifest() {
        let m = validate(&valid_toml()).unwrap();
        assert_eq!(m.id, "example.addon");
        assert_eq!(m.name, "Example Addon");
        assert_eq!(m.version.to_string(), "1.0.0");
        assert_eq!(m.entry, "src/main.lua");
        assert_eq!(m.api_version, (1, 0));
    }

    #[test]
    fn leading_dot_slash_entry_is_normalized() {
        let mut toml = valid_toml();
        toml = toml.replace("src/main.lua", "./src/main.lua");
        let m = validate(&toml).unwrap();
        assert_eq!(m.entry, "src/main.lua");
    }

    #[test]
    fn rejects_missing_required_fields() {
        for field in ["id", "name", "version", "entry"] {
            let mut toml = valid_toml();
            toml = toml.lines().filter(|l| !l.starts_with(&format!("{field} ="))).collect::<Vec<_>>().join("\n");
            let e = validate(&toml).unwrap_err();
            assert_eq!(e.code(), "InvalidManifest", "{field}");
            assert!(e.to_string().contains(field), "{field}: {e}");
        }
    }

    #[test]
    fn rejects_bad_ids() {
        for bad in ["bad", "Bad.Addon", "1.2", "a..b", ".a.b", "a.b.", "a b.c", "é.c"] {
            let mut toml = valid_toml();
            toml = toml.replace("example.addon", bad);
            let e = validate(&toml).unwrap_err();
            assert_eq!(e.code(), "InvalidManifest", "{bad}");
        }
    }

    #[test]
    fn accepts_minimum_two_segment_id() {
        let mut toml = valid_toml();
        toml = toml.replace("example.addon", "a.b");
        assert!(validate(&toml).is_ok());
    }

    #[test]
    fn rejects_bad_semver() {
        for bad in ["1.0", "1.0.0.0", "abc"] {
            let mut toml = valid_toml();
            toml = toml.replace("1.0.0", bad);
            let e = validate(&toml).unwrap_err();
            assert_eq!(e.code(), "InvalidManifest", "{bad}");
        }
    }

    #[test]
    fn accepts_full_semver_with_metadata() {
        // "1.2.3-beta+ok" is valid strict SemVer 2.0.0 (prerelease + build
        // metadata), so it must be accepted.
        let mut toml = valid_toml();
        toml = toml.replace("version = \"1.0.0\"", "version = \"1.2.3-beta+ok\"");
        let m = validate(&toml).unwrap();
        assert_eq!(m.version.to_string(), "1.2.3-beta+ok");
    }

    #[test]
    fn accepts_prerelease_semver() {
        let mut toml = valid_toml();
        toml = toml.replace("version = \"1.0.0\"", "version = \"1.2.3-rc.1\"");
        let m = validate(&toml).unwrap();
        assert_eq!(m.version.to_string(), "1.2.3-rc.1");
    }

    #[test]
    fn rejects_entry_escape() {
        for bad in ["../main.lua", "/etc/x.lua", "C:\\x.lua", "src\\main.lua", "src/main.txt"] {
            let mut toml = valid_toml();
            toml = toml.replace("src/main.lua", bad);
            assert!(validate(&toml).is_err(), "{bad}");
        }
    }

    #[test]
    fn rejects_unsupported_schema_version() {
        let mut toml = valid_toml();
        toml = format!("schema_version = 2\n{toml}");
        let e = validate(&toml).unwrap_err();
        assert!(e.to_string().contains("schema_version"));
    }

    #[test]
    fn app_version_range_checks() {
        // min_app_version above host -> reject
        let toml = format!("{}\nmin_app_version = \"9.9.9\"\n", valid_toml());
        assert!(validate(&toml).is_err());

        // max_app_version below host -> reject
        let mut toml = valid_toml();
        toml = toml.replace("min_app_version = \"0.3.0\"", "");
        toml = format!("{}\nmax_app_version = \"0.1.0\"\n", toml);
        assert!(validate(&toml).is_err());
    }

    #[test]
    fn api_version_compatibility() {
        // wrong major -> reject
        let mut toml = valid_toml();
        toml = toml.replace("api_version = \"1.0\"", "api_version = \"2.0\"");
        assert!(validate(&toml).is_err());

        // minor beyond host -> reject
        let mut toml = valid_toml();
        toml = toml.replace("api_version = \"1.0\"", "api_version = \"1.5\"");
        assert!(validate(&toml).is_err());

        // Default api_version (host API) is within host -> ok.
        let mut toml = valid_toml();
        toml = toml
            .lines()
            .filter(|l| !l.starts_with("api_version ="))
            .collect::<Vec<_>>()
            .join("\n");
        assert!(validate(&toml).is_ok());
    }

    #[test]
    fn unknown_fields_tolerated() {
        let mut toml = valid_toml();
        toml = format!("{}\n[permissions]\nnetwork = false\n", toml);
        assert!(validate(&toml).is_ok());
    }

    #[test]
    fn invalid_toml_rejected() {
        assert!(validate("id = [").is_err());
    }

    #[test]
    fn id_validator() {
        assert!(validate_addon_id("example.addon"));
        assert!(validate_addon_id("a.b"));
        assert!(validate_addon_id("a0.b1.c2"));
        assert!(!validate_addon_id("a"));
        assert!(!validate_addon_id("a."));
        assert!(!validate_addon_id(".a"));
        assert!(!validate_addon_id("a..b"));
        assert!(!validate_addon_id("A.b"));
        assert!(!validate_addon_id("a.b-c"));
        assert!(!validate_addon_id(&("a".repeat(65) + ".b")));
    }
}
