//! C ABI exports for the git command crate.

#![allow(unsafe_code)]

use std::ffi::{c_char, c_int, CStr, CString};
use std::path::Path;

use crate::git;
use crate::jj;

// ─── Helpers ───────────────────────────────────────────────

unsafe fn cstr<'a>(p: *const c_char) -> Result<&'a str, c_int> {
    if p.is_null() {
        return Err(-1);
    }
    let c = unsafe { CStr::from_ptr(p) };
    c.to_str().map_err(|_| -1)
}

fn to_cstring(s: String) -> *mut c_char {
    CString::new(s)
        .unwrap_or_else(|_| CString::new("<bad>").unwrap())
        .into_raw()
}

// ─── Detection ─────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn vcs_is_git_available() -> bool {
    git::is_git_available()
}

#[no_mangle]
pub extern "C" fn vcs_is_jj_available() -> bool {
    jj::is_jj_available()
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vcs_is_git_repo(path: *const c_char) -> bool {
    unsafe {
        match cstr(path) {
            Ok(p) => git::is_git_repo(Path::new(p)),
            Err(_) => false,
        }
    }
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vcs_is_jj_repo(path: *const c_char) -> bool {
    unsafe {
        match cstr(path) {
            Ok(p) => jj::is_jj_repo(Path::new(p)),
            Err(_) => false,
        }
    }
}

// ─── Git commands ──────────────────────────────────────────

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vcs_git_status(repo_path: *const c_char) -> *mut c_char {
    unsafe {
        match cstr(repo_path) {
            Ok(p) => match git::status(Path::new(p)) {
                Ok(statuses) => {
                    let json: Vec<serde_json::Value> = statuses
                        .iter()
                        .map(|s| {
                            serde_json::json!({
                                "status": s.status.to_string(),
                                "path": s.path,
                            })
                        })
                        .collect();
                    to_cstring(serde_json::to_string(&json).unwrap_or_default())
                }
                Err(_) => std::ptr::null_mut(),
            },
            Err(_) => std::ptr::null_mut(),
        }
    }
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vcs_git_log(
    repo_path: *const c_char,
    limit: u32,
) -> *mut c_char {
    unsafe {
        match cstr(repo_path) {
            Ok(p) => match git::log(Path::new(p), limit as usize) {
                Ok(entries) => {
                    let json: Vec<serde_json::Value> = entries
                        .iter()
                        .map(|e| {
                            serde_json::json!({
                                "hash": e.hash,
                                "subject": e.subject,
                            })
                        })
                        .collect();
                    to_cstring(serde_json::to_string(&json).unwrap_or_default())
                }
                Err(_) => std::ptr::null_mut(),
            },
            Err(_) => std::ptr::null_mut(),
        }
    }
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vcs_git_branches(repo_path: *const c_char) -> *mut c_char {
    unsafe {
        match cstr(repo_path) {
            Ok(p) => match git::list_branches(Path::new(p)) {
                Ok(branches) => {
                    let json: Vec<serde_json::Value> = branches
                        .iter()
                        .map(|b| {
                            serde_json::json!({
                                "name": b.name,
                                "current": b.current,
                            })
                        })
                        .collect();
                    to_cstring(serde_json::to_string(&json).unwrap_or_default())
                }
                Err(_) => std::ptr::null_mut(),
            },
            Err(_) => std::ptr::null_mut(),
        }
    }
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vcs_git_current_branch(repo_path: *const c_char) -> *mut c_char {
    unsafe {
        match cstr(repo_path) {
            Ok(p) => match git::current_branch(Path::new(p)) {
                Ok(branch) => to_cstring(branch),
                Err(_) => std::ptr::null_mut(),
            },
            Err(_) => std::ptr::null_mut(),
        }
    }
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vcs_git_diff_file(
    repo_path: *const c_char,
    file: *const c_char,
) -> *mut c_char {
    unsafe {
        match (cstr(repo_path), cstr(file)) {
            (Ok(p), Ok(f)) => match git::diff_file(Path::new(p), f) {
                Ok(diff) => to_cstring(diff),
                Err(_) => std::ptr::null_mut(),
            },
            _ => std::ptr::null_mut(),
        }
    }
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vcs_git_is_dirty(repo_path: *const c_char) -> bool {
    unsafe {
        match cstr(repo_path) {
            Ok(p) => git::is_dirty(Path::new(p)),
            Err(_) => false,
        }
    }
}

// ─── Git mutating commands ──────────────────────────────────

/// Stage files. `files_json` is a JSON array of path strings.
/// Returns 0 on success, -1 on error.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vcs_git_add(
    repo_path: *const c_char,
    files_json: *const c_char,
) -> c_int {
    unsafe {
        match (cstr(repo_path), cstr(files_json)) {
            (Ok(p), Ok(f)) => {
                let paths: Vec<String> = match serde_json::from_str(f) {
                    Ok(v) => v,
                    Err(_) => return -1,
                };
                let refs: Vec<&str> = paths.iter().map(|s| s.as_str()).collect();
                match git::add_files(Path::new(p), &refs) {
                    Ok(r) if r.success() => 0,
                    _ => -1,
                }
            }
            _ => -1,
        }
    }
}

/// Commit with a message.
/// Returns 0 on success, -1 on error.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vcs_git_commit(
    repo_path: *const c_char,
    message: *const c_char,
) -> c_int {
    unsafe {
        match (cstr(repo_path), cstr(message)) {
            (Ok(p), Ok(m)) => match git::commit(Path::new(p), m) {
                Ok(r) if r.success() => 0,
                _ => -1,
            },
            _ => -1,
        }
    }
}

/// Push to remote.
/// Returns 0 on success, -1 on error.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vcs_git_push(repo_path: *const c_char) -> c_int {
    unsafe {
        match cstr(repo_path) {
            Ok(p) => match git::push(Path::new(p)) {
                Ok(r) if r.success() => 0,
                _ => -1,
            },
            Err(_) => -1,
        }
    }
}

/// Fetch from remote.
/// Returns 0 on success, -1 on error.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vcs_git_fetch(repo_path: *const c_char) -> c_int {
    unsafe {
        match cstr(repo_path) {
            Ok(p) => match git::fetch(Path::new(p)) {
                Ok(r) if r.success() => 0,
                _ => -1,
            },
            Err(_) => -1,
        }
    }
}

/// Pull from remote.
/// Returns 0 on success, -1 on error.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vcs_git_pull(repo_path: *const c_char) -> c_int {
    unsafe {
        match cstr(repo_path) {
            Ok(p) => match git::pull(Path::new(p)) {
                Ok(r) if r.success() => 0,
                _ => -1,
            },
            Err(_) => -1,
        }
    }
}

/// Checkout a branch.
/// Returns 0 on success, -1 on error.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vcs_git_checkout(
    repo_path: *const c_char,
    branch: *const c_char,
) -> c_int {
    unsafe {
        match (cstr(repo_path), cstr(branch)) {
            (Ok(p), Ok(b)) => match git::checkout(Path::new(p), b) {
                Ok(r) if r.success() => 0,
                _ => -1,
            },
            _ => -1,
        }
    }
}

/// Create a new branch and checkout.
/// Returns 0 on success, -1 on error.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vcs_git_create_branch(
    repo_path: *const c_char,
    name: *const c_char,
) -> c_int {
    unsafe {
        match (cstr(repo_path), cstr(name)) {
            (Ok(p), Ok(n)) => match git::create_branch(Path::new(p), n) {
                Ok(r) if r.success() => 0,
                _ => -1,
            },
            _ => -1,
        }
    }
}

/// Delete a branch.
/// Returns 0 on success, -1 on error.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vcs_git_delete_branch(
    repo_path: *const c_char,
    name: *const c_char,
) -> c_int {
    unsafe {
        match (cstr(repo_path), cstr(name)) {
            (Ok(p), Ok(n)) => match git::delete_branch(Path::new(p), n) {
                Ok(r) if r.success() => 0,
                _ => -1,
            },
            _ => -1,
        }
    }
}

/// Clone a repository. Returns 0 on success, -1 on error.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vcs_git_clone(
    url: *const c_char,
    dest: *const c_char,
) -> c_int {
    unsafe {
        match (cstr(url), cstr(dest)) {
            (Ok(u), Ok(d)) => match git::clone(u, Path::new(d)) {
                Ok(r) if r.success() => 0,
                _ => -1,
            },
            _ => -1,
        }
    }
}

/// Initialize a new git repository. Returns 0 on success, -1 on error.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vcs_git_init(path: *const c_char) -> c_int {
    unsafe {
        match cstr(path) {
            Ok(p) => match git::init(Path::new(p)) {
                Ok(r) if r.success() => 0,
                _ => -1,
            },
            Err(_) => -1,
        }
    }
}

/// List files changed in a commit as JSON array of {status, path}.
/// Caller frees with vcs_free_string.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vcs_git_commit_files(
    repo_path: *const c_char,
    hash: *const c_char,
) -> *mut c_char {
    unsafe {
        match (cstr(repo_path), cstr(hash)) {
            (Ok(p), Ok(h)) => match git::commit_files(Path::new(p), h) {
                Ok(statuses) => {
                    let json: Vec<serde_json::Value> = statuses
                        .iter()
                        .map(|s| {
                            serde_json::json!({
                                "status": s.status.to_string(),
                                "path": s.path,
                            })
                        })
                        .collect();
                    to_cstring(serde_json::to_string(&json).unwrap_or_default())
                }
                Err(_) => std::ptr::null_mut(),
            },
            _ => std::ptr::null_mut(),
        }
    }
}

/// Read a git config value. Returns the value string or null.
/// Caller frees with vcs_free_string.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vcs_git_config_get(
    repo_path: *const c_char,
    key: *const c_char,
) -> *mut c_char {
    unsafe {
        match (cstr(repo_path), cstr(key)) {
            (Ok(p), Ok(k)) => {
                let result = git::run_git(Path::new(p), &["config", k]);
                match result {
                    Ok(r) if r.success() => to_cstring(r.stdout.trim().to_string()),
                    _ => std::ptr::null_mut(),
                }
            }
            _ => std::ptr::null_mut(),
        }
    }
}

/// Set a git config value. Returns 0 on success, -1 on error.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vcs_git_config_set(
    repo_path: *const c_char,
    key: *const c_char,
    value: *const c_char,
) -> c_int {
    unsafe {
        match (cstr(repo_path), cstr(key), cstr(value)) {
            (Ok(p), Ok(k), Ok(v)) => {
                let result = git::run_git(Path::new(p), &["config", k, v]);
                match result {
                    Ok(r) if r.success() => 0,
                    _ => -1,
                }
            }
            _ => -1,
        }
    }
}

/// Get staged diff (cached). Caller frees with vcs_free_string.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vcs_git_diff_staged(repo_path: *const c_char) -> *mut c_char {
    unsafe {
        match cstr(repo_path) {
            Ok(p) => {
                let result = git::run_git(Path::new(p), &["diff", "--cached"]);
                match result {
                    Ok(r) => to_cstring(r.stdout),
                    Err(_) => std::ptr::null_mut(),
                }
            }
            Err(_) => std::ptr::null_mut(),
        }
    }
}

/// Restore a file to HEAD state (discard changes). Returns 0 on success.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vcs_git_restore_file(
    repo_path: *const c_char,
    file: *const c_char,
) -> c_int {
    unsafe {
        match (cstr(repo_path), cstr(file)) {
            (Ok(p), Ok(f)) => {
                let result = git::run_git(Path::new(p), &["checkout", "--", f]);
                match result {
                    Ok(r) if r.success() => 0,
                    _ => -1,
                }
            }
            _ => -1,
        }
    }
}

/// Run an arbitrary git command and return raw stdout.
/// Caller frees with vcs_free_string. Returns null on error.
#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vcs_git_run_raw(
    repo_path: *const c_char,
    args_json: *const c_char,
) -> *mut c_char {
    unsafe {
        match (cstr(repo_path), cstr(args_json)) {
            (Ok(p), Ok(a)) => {
                let args: Vec<String> = match serde_json::from_str(a) {
                    Ok(v) => v,
                    Err(_) => return std::ptr::null_mut(),
                };
                let refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                match git::run_git(Path::new(p), &refs) {
                    Ok(r) => to_cstring(r.stdout),
                    Err(_) => std::ptr::null_mut(),
                }
            }
            _ => std::ptr::null_mut(),
        }
    }
}

// ─── jj commands ───────────────────────────────────────────

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vcs_jj_status(repo_path: *const c_char) -> *mut c_char {
    unsafe {
        match cstr(repo_path) {
            Ok(p) => {
                let result = jj::run_jj(Path::new(p), &["status", "--color", "never", "--config", "ui.pagination=never"]);
                match result {
                    Ok(r) => {
                        let statuses = jj::parse_status(&r.stdout);
                        let json: Vec<serde_json::Value> = statuses
                            .iter()
                            .map(|s| {
                                serde_json::json!({
                                    "status": s.status.to_string(),
                                    "path": s.path,
                                })
                            })
                            .collect();
                        to_cstring(serde_json::to_string(&json).unwrap_or_default())
                    }
                    Err(_) => std::ptr::null_mut(),
                }
            }
            Err(_) => std::ptr::null_mut(),
        }
    }
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vcs_jj_diff_file(
    repo_path: *const c_char,
    file: *const c_char,
) -> *mut c_char {
    unsafe {
        match (cstr(repo_path), cstr(file)) {
            (Ok(p), Ok(f)) => match jj::diff_file(Path::new(p), f) {
                Ok(diff) => to_cstring(diff),
                Err(_) => std::ptr::null_mut(),
            },
            _ => std::ptr::null_mut(),
        }
    }
}

// ─── Free ──────────────────────────────────────────────────

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn vcs_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}
