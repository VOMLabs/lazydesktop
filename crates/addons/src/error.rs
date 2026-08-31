//! Structured error taxonomy and sanitized JSON serialization.
//!
//! Per the design spec (§12), errors never leak internal paths or secrets.
//! Every message that can reach the UI is run through [`sanitize_message`]
//! before serialization.

use serde::Serialize;

/// Canonical error taxonomy for the addon subsystem.
///
/// The variants map 1:1 to the `lda_result` codes in `addons.h`; the
/// `LuaRuntimeError` variant is produced by the C++ host and reported through
/// the same taxonomy.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AddonError {
    #[error("{message}")]
    InvalidAddon { id: Option<String>, message: String },
    #[error("{message}")]
    InvalidManifest {
        id: Option<String>,
        field: Option<String>,
        message: String,
    },
    #[error("{message}")]
    ArchiveError {
        /// Internal detail string (e.g. zip error text). Not a `#[source]`:
        /// it is a display-only String, kept out of the error chain.
        detail: String,
        message: String,
    },
    #[error("{message}")]
    PathSecurityViolation {
        /// Normalized relative path only — never an absolute host path.
        path: String,
        message: String,
    },
    #[error("{message}")]
    LuaRuntimeError { addon_id: String, message: String },
    #[error("{message}")]
    ProviderError { provider: String, message: String },
    #[error("{message}")]
    FfiError { message: String },
    #[error("{message}")]
    Internal { message: String },
}

impl AddonError {
    /// Stable machine code for the error (the `code` field in the JSON
    /// contract and the suffix of the `lda_result` enum).
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidAddon { .. } => "InvalidAddon",
            Self::InvalidManifest { .. } => "InvalidManifest",
            Self::ArchiveError { .. } => "ArchiveError",
            Self::PathSecurityViolation { .. } => "PathSecurityViolation",
            Self::LuaRuntimeError { .. } => "LuaRuntimeError",
            Self::ProviderError { .. } => "ProviderError",
            Self::FfiError { .. } => "FfiError",
            Self::Internal { .. } => "Internal",
        }
    }

    /// Sanitized, display-safe message (never contains absolute host paths).
    pub fn display_message(&self) -> String {
        sanitize_message(&self.to_string())
    }

    /// Serialize this error into the FFI error JSON contract:
    /// `{"error": {"code": "...", "message": "...", ...}}`.
    pub fn to_json(&self) -> String {
        let message = self.display_message();
        let (addon_id, field) = match self {
            Self::InvalidAddon { id, .. } => (id.clone(), None),
            Self::InvalidManifest { id, field, .. } => (id.clone(), field.clone()),
            Self::LuaRuntimeError { addon_id, .. } => (Some(addon_id.clone()), None),
            _ => (None, None),
        };
        let payload = ErrorPayload {
            error: ErrorJson {
                code: self.code().to_string(),
                message,
                addon_id,
                field,
            },
        };
        serde_json::to_string(&payload).unwrap_or_else(|_| {
            r#"{"error":{"code":"Internal","message":"serialization error"}}"#.to_string()
        })
    }

    // ─── Constructors ───────────────────────────────────

    pub fn invalid_addon(id: impl Into<Option<String>>, message: impl Into<String>) -> Self {
        Self::InvalidAddon {
            id: id.into(),
            message: message.into(),
        }
    }

    pub fn invalid_manifest(
        id: impl Into<Option<String>>,
        field: impl Into<Option<String>>,
        message: impl Into<String>,
    ) -> Self {
        Self::InvalidManifest {
            id: id.into(),
            field: field.into(),
            message: message.into(),
        }
    }

    pub fn archive(detail: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ArchiveError {
            detail: detail.into(),
            message: message.into(),
        }
    }

    pub fn path_security(path: impl Into<String>, message: impl Into<String>) -> Self {
        Self::PathSecurityViolation {
            path: path.into(),
            message: message.into(),
        }
    }

    pub fn lua(addon_id: impl Into<String>, message: impl Into<String>) -> Self {
        Self::LuaRuntimeError {
            addon_id: addon_id.into(),
            message: message.into(),
        }
    }

    pub fn provider(provider: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ProviderError {
            provider: provider.into(),
            message: message.into(),
        }
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }
}

/// JSON shape of a single error (see §12.2 of ADDON_SPEC).
#[derive(Serialize)]
pub struct ErrorJson {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub addon_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub field: Option<String>,
}

#[derive(Serialize)]
pub struct ErrorPayload {
    pub error: ErrorJson,
}

/// Redact anything that looks like an absolute filesystem path or a control
/// character from a display message.
///
/// This is a backstop; the crate's own messages are written path-free. It is
/// intentionally conservative: an over-redaction is safe, an under-redaction
/// of an absolute path is not.
pub fn sanitize_message(input: &str) -> String {
    let chars: Vec<char> = input.chars().collect();
    let mut out = String::with_capacity(input.len());
    let mut i = 0;
    let at_token_start = |idx: usize| idx == 0 || chars[idx - 1].is_whitespace();
    while i < chars.len() {
        let c = chars[i];

        // Windows drive prefix (e.g. "C:\Users\x" or "C:/x"). Only when the
        // letter starts a token — otherwise "https:" would be redacted.
        if c.is_ascii_alphabetic() && chars.get(i + 1) == Some(&':') && at_token_start(i) {
            let mut j = i + 2;
            let mut has_sep = false;
            while j < chars.len() && is_path_char(chars[j]) {
                if chars[j] == '/' || chars[j] == '\\' {
                    has_sep = true;
                }
                j += 1;
            }
            if has_sep && j > i + 2 {
                out.push_str("<internal>");
                i = j;
                continue;
            }
        }

        // Absolute POSIX or backslash path (starts with '/' or '\'). Only when
        // the separator starts a token — "https://..." must be preserved.
        if (c == '/' || c == '\\') && at_token_start(i) {
            let mut j = i;
            let mut separators = 0usize;
            while j < chars.len() && is_path_char(chars[j]) {
                if chars[j] == '/' || chars[j] == '\\' {
                    separators += 1;
                }
                j += 1;
            }
            if separators >= 1 && j > i + 1 {
                out.push_str("<internal>");
                i = j;
                continue;
            }
        }

        // Strip control characters (keep newline/tab).
        if (c as u32) < 0x20 && c != '\n' && c != '\t' {
            out.push(' ');
        } else {
            out.push(c);
        }
        i += 1;
    }
    out
}

fn is_path_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '/' | '\\' | '.' | '_' | '-' | '~' | '+' | ':' | '@')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizer_redacts_posix_absolute_paths() {
        let msg = "failed to read /home/me/.config/lazydesktop/x for addon";
        let out = sanitize_message(msg);
        assert!(!out.contains("/home"), "got: {out}");
        assert!(out.contains("<internal>"), "got: {out}");
        assert!(out.contains("failed to read"), "got: {out}");
    }

    #[test]
    fn sanitizer_redacts_windows_paths() {
        let msg = "path C:\\Users\\alice\\AppData\\Roaming leaked";
        let out = sanitize_message(msg);
        assert!(!out.contains("C:"), "got: {out}");
        assert!(out.contains("<internal>"), "got: {out}");
    }

    #[test]
    fn sanitizer_keeps_normal_text_and_relative_paths() {
        let msg = "field 'version' is not valid semver (expected 1.2.3)";
        let out = sanitize_message(msg);
        assert_eq!(out, msg);
    }

    #[test]
    fn sanitizer_keeps_urls() {
        // URLs start with a letter, not a separator, and are not drive paths.
        let msg = "see https://example.com/addon for details";
        let out = sanitize_message(msg);
        assert!(out.contains("https://example.com/addon"), "got: {out}");
    }

    #[test]
    fn sanitizer_strips_control_characters() {
        let msg = "bad\u{0000}\u{0007}message";
        let out = sanitize_message(msg);
        assert!(!out.contains('\u{0000}'));
        assert!(!out.contains('\u{0007}'));
        assert!(out.contains("bad"), "got: {out}");
    }

    #[test]
    fn sanitizer_keeps_newlines_and_tabs() {
        let msg = "line1\nline2\tcol";
        let out = sanitize_message(msg);
        assert_eq!(out, msg);
    }

    #[test]
    fn json_error_shape_matches_contract() {
        let e = AddonError::invalid_manifest(
            Some("example.addon".into()),
            Some("version".into()),
            "field 'version' is not valid semver",
        );
        let v: serde_json::Value = serde_json::from_str(&e.to_json()).unwrap();
        assert_eq!(v["error"]["code"], "InvalidManifest");
        assert_eq!(v["error"]["addon_id"], "example.addon");
        assert_eq!(v["error"]["field"], "version");
    }

    #[test]
    fn json_error_has_no_internal_paths() {
        let e = AddonError::internal("io error opening /tmp/secret/file.bin");
        let json = e.to_json();
        assert!(!json.contains("/tmp/secret"), "got: {json}");
        assert!(json.contains("<internal>"), "got: {json}");
    }

    #[test]
    fn codes_are_stable() {
        assert_eq!(AddonError::invalid_addon(None, "x").code(), "InvalidAddon");
        assert_eq!(
            AddonError::invalid_manifest(None, None, "x").code(),
            "InvalidManifest"
        );
        assert_eq!(AddonError::archive("a", "b").code(), "ArchiveError");
        assert_eq!(
            AddonError::path_security("x", "y").code(),
            "PathSecurityViolation"
        );
        assert_eq!(AddonError::lua("id", "m").code(), "LuaRuntimeError");
        assert_eq!(AddonError::provider("p", "m").code(), "ProviderError");
        assert_eq!(
            AddonError::FfiError {
                message: "m".into()
            }
            .code(),
            "FfiError"
        );
        assert_eq!(AddonError::internal("m").code(), "Internal");
    }
}
