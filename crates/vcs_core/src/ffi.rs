//! C ABI for vcs_core (see `vcs_core.h` for the C/C++-consumable contract).
//!
//! Conventions mirror crates/ai_core/src/ffi.rs:
//! - Functions are `#[no_mangle] pub extern "C" fn`.
//! - Strings returned to the caller are heap-allocated `*mut c_char` and MUST
//!   be freed with `vcs_free_string`.
//! - All functions validate null/empty input and return errors instead of
//!   panicking across the boundary.
//!
//! Security invariant: private key material never crosses the FFI boundary.
//! Only public keys, fingerprints, paths, and status/error strings do.

use std::ffi::{CStr, CString};
use std::os::raw::c_char;

// ─── String helpers ────────────────────────────────────

/// Free a string previously returned by a vcs_* function.
///
/// # Safety
/// `s` must be null or a pointer previously returned by this crate (never a
/// string literal or a borrowed buffer).
#[no_mangle]
pub unsafe extern "C" fn vcs_free_string(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    unsafe {
        drop(CString::from_raw(s));
    }
}

fn cstr_or(ptr: *const c_char, fallback: &str) -> String {
    if ptr.is_null() {
        return fallback.to_string();
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_str()
        .unwrap_or(fallback)
        .to_string()
}

// ─── SSH keys ──────────────────────────────────────────
/// Returns a JSON array of absolute paths of all `*.pub` keys in ~/.ssh
/// (sorted by name), or a JSON error object. Caller frees with
/// `vcs_free_string`.
#[no_mangle]
pub extern "C" fn vcs_ssh_list_public_keys() -> *mut c_char {
    let result = crate::ssh::list_public_keys().and_then(|keys| {
        serde_json::to_string(&keys).map_err(|e| crate::error::VcsError::Other(e.to_string()))
    });
    let json = match result {
        Ok(s) => s,
        Err(e) => format!("{{\"error\":{}}}", serde_json::json!(e.to_string())),
    };
    CString::new(json).unwrap_or_default().into_raw()
}

/// Generate an SSH keypair. Returns "ok" on success or an error message.
#[no_mangle]
pub extern "C" fn vcs_ssh_generate_key(
    key_type: *const c_char,
    path: *const c_char,
    comment: *const c_char,
    passphrase: *const c_char,
) -> *mut c_char {
    let pass_str;
    let pass_opt = if passphrase.is_null() {
        None
    } else {
        pass_str = cstr_or(passphrase, "");
        Some(pass_str.as_str())
    };
    let result = crate::ssh::generate_key(
        &cstr_or(key_type, "ed25519"),
        std::path::Path::new(&cstr_or(path, "")),
        &cstr_or(comment, ""),
        pass_opt,
    );
    let out = match result {
        Ok(()) => "ok".to_string(),
        Err(e) => e.to_string(),
    };
    CString::new(out).unwrap_or_default().into_raw()
}

/// Compute the SHA256 fingerprint of a public key file. Returns the
/// fingerprint or an error message.
#[no_mangle]
pub extern "C" fn vcs_ssh_fingerprint(pub_path: *const c_char) -> *mut c_char {
    let result = crate::ssh::fingerprint_public_key(std::path::Path::new(&cstr_or(pub_path, "")));
    let out = match result {
        Ok(s) => s,
        Err(e) => e.to_string(),
    };
    CString::new(out).unwrap_or_default().into_raw()
}

/// Read a public key file. Returns its trimmed text or an error message.
#[no_mangle]
pub extern "C" fn vcs_ssh_read_public_key(pub_path: *const c_char) -> *mut c_char {
    let result = crate::ssh::read_public_key(std::path::Path::new(&cstr_or(pub_path, "")));
    let out = match result {
        Ok(s) => s,
        Err(e) => e.to_string(),
    };
    CString::new(out).unwrap_or_default().into_raw()
}

// ─── Connection test ───────────────────────────────────

/// Test an SSH connection to `host` on `port`. Returns a human-readable
/// success/error message. Caller frees with `vcs_free_string`.
#[no_mangle]
pub extern "C" fn vcs_ssh_test_connection(host: *const c_char, port: u16) -> *mut c_char {
    let result = crate::connect::test_ssh_connection(&cstr_or(host, ""), port);
    let out = match result {
        Ok(s) => s,
        Err(e) => e.to_string(),
    };
    CString::new(out).unwrap_or_default().into_raw()
}

// ─── Remotes ───────────────────────────────────────────

fn remote_json(result: VcsJsonResult) -> *mut c_char {
    let json = match result {
        VcsJsonResult::Ok(s) => s,
        VcsJsonResult::Err(e) => format!("{{\"error\":{}}}", serde_json::json!(e)),
    };
    CString::new(json).unwrap_or_default().into_raw()
}

enum VcsJsonResult {
    Ok(String),
    Err(String),
}

/// List remotes as a JSON array of `{name, url}` objects.
#[no_mangle]
pub extern "C" fn vcs_remote_list(repo_path: *const c_char) -> *mut c_char {
    let result = crate::remote::list_remotes(std::path::Path::new(&cstr_or(repo_path, "")))
        .and_then(|remotes| {
            serde_json::to_string(
                &remotes
                    .iter()
                    .map(|r| serde_json::json!({"name": r.name, "url": r.url}))
                    .collect::<Vec<_>>(),
            )
            .map_err(|e| crate::error::VcsError::Other(e.to_string()))
        });
    remote_json(match result {
        Ok(s) => VcsJsonResult::Ok(s),
        Err(e) => VcsJsonResult::Err(e.to_string()),
    })
}

/// Add a remote. Returns "ok" or an error message.
#[no_mangle]
pub extern "C" fn vcs_remote_add(
    repo_path: *const c_char,
    name: *const c_char,
    url: *const c_char,
) -> *mut c_char {
    let result = crate::remote::add_remote(
        std::path::Path::new(&cstr_or(repo_path, "")),
        &cstr_or(name, ""),
        &cstr_or(url, ""),
    );
    let out = match result {
        Ok(()) => "ok".to_string(),
        Err(e) => e.to_string(),
    };
    CString::new(out).unwrap_or_default().into_raw()
}

/// Remove a remote. Returns "ok" or an error message.
#[no_mangle]
pub extern "C" fn vcs_remote_remove(repo_path: *const c_char, name: *const c_char) -> *mut c_char {
    let result = crate::remote::remove_remote(
        std::path::Path::new(&cstr_or(repo_path, "")),
        &cstr_or(name, ""),
    );
    let out = match result {
        Ok(()) => "ok".to_string(),
        Err(e) => e.to_string(),
    };
    CString::new(out).unwrap_or_default().into_raw()
}

/// Set a remote's URL. Returns "ok" or an error message.
#[no_mangle]
pub extern "C" fn vcs_remote_set_url(
    repo_path: *const c_char,
    name: *const c_char,
    url: *const c_char,
) -> *mut c_char {
    let result = crate::remote::set_remote_url(
        std::path::Path::new(&cstr_or(repo_path, "")),
        &cstr_or(name, ""),
        &cstr_or(url, ""),
    );
    let out = match result {
        Ok(()) => "ok".to_string(),
        Err(e) => e.to_string(),
    };
    CString::new(out).unwrap_or_default().into_raw()
}

/// Rename a remote. Returns "ok" or an error message.
#[no_mangle]
pub extern "C" fn vcs_remote_rename(
    repo_path: *const c_char,
    old_name: *const c_char,
    new_name: *const c_char,
) -> *mut c_char {
    let result = crate::remote::rename_remote(
        std::path::Path::new(&cstr_or(repo_path, "")),
        &cstr_or(old_name, ""),
        &cstr_or(new_name, ""),
    );
    let out = match result {
        Ok(()) => "ok".to_string(),
        Err(e) => e.to_string(),
    };
    CString::new(out).unwrap_or_default().into_raw()
}
