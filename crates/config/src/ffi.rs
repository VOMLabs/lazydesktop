//! C ABI exports for the configuration crate.

#![allow(unsafe_code)]

use std::ffi::{c_char, c_int, CStr, CString};
use std::path::Path;

use crate::projects::RecentProjects;
use crate::settings::Settings;

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

// ─── Settings FFI ──────────────────────────────────────────

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn config_settings_load(path: *const c_char) -> *mut c_char {
    unsafe {
        let path_str = match cstr(path) {
            Ok(s) => s,
            Err(_) => return std::ptr::null_mut(),
        };
        let settings = match Settings::load(Path::new(path_str)) {
            Ok(s) => s,
            Err(_) => return std::ptr::null_mut(),
        };
        let mut map = serde_json::Map::new();
        for key in settings.keys_with_prefix("") {
            if let Some(val) = settings.get(key) {
                map.insert(key.to_string(), serde_json::Value::String(val.to_string()));
            }
        }
        match serde_json::to_string(&map) {
            Ok(json) => to_cstring(json),
            Err(_) => std::ptr::null_mut(),
        }
    }
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn config_settings_get(
    settings_json: *const c_char,
    key: *const c_char,
) -> *mut c_char {
    unsafe {
        let json = match cstr(settings_json) {
            Ok(s) => s,
            Err(_) => return std::ptr::null_mut(),
        };
        let k = match cstr(key) {
            Ok(s) => s,
            Err(_) => return std::ptr::null_mut(),
        };
        let map: serde_json::Value = match serde_json::from_str(json) {
            Ok(v) => v,
            Err(_) => return std::ptr::null_mut(),
        };
        let val = map.get(k).and_then(|v| v.as_str()).unwrap_or("");
        to_cstring(val.to_string())
    }
}

// ─── Projects FFI ──────────────────────────────────────────

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn config_projects_load(path: *const c_char) -> *mut c_char {
    unsafe {
        let path_str = match cstr(path) {
            Ok(s) => s,
            Err(_) => return std::ptr::null_mut(),
        };
        let projects = RecentProjects::load(Path::new(path_str));
        let json = serde_json::to_string(projects.paths()).unwrap_or_else(|_| "[]".to_string());
        to_cstring(json)
    }
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn config_projects_save(path: *const c_char, projects_json: *const c_char) -> c_int {
    unsafe {
        let path_str = match cstr(path) {
            Ok(s) => s,
            Err(_) => return -1,
        };
        let json = match cstr(projects_json) {
            Ok(s) => s,
            Err(_) => return -1,
        };
        let paths: Vec<String> = match serde_json::from_str(json) {
            Ok(p) => p,
            Err(_) => return -1,
        };
        let mut projects = RecentProjects::new();
        for p in paths {
            projects.add(&p);
        }
        match projects.save(Path::new(path_str)) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn config_projects_add(path: *const c_char, project_path: *const c_char) -> c_int {
    unsafe {
        let path_str = match cstr(path) {
            Ok(s) => s,
            Err(_) => return -1,
        };
        let proj = match cstr(project_path) {
            Ok(s) => s,
            Err(_) => return -1,
        };
        let mut projects = RecentProjects::load(Path::new(path_str));
        projects.add(proj);
        match projects.save(Path::new(path_str)) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn config_projects_remove(
    path: *const c_char,
    project_path: *const c_char,
) -> c_int {
    unsafe {
        let path_str = match cstr(path) {
            Ok(s) => s,
            Err(_) => return -1,
        };
        let proj = match cstr(project_path) {
            Ok(s) => s,
            Err(_) => return -1,
        };
        let mut projects = RecentProjects::load(Path::new(path_str));
        projects.remove(proj);
        match projects.save(Path::new(path_str)) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn config_projects_clear(path: *const c_char) -> c_int {
    unsafe {
        let path_str = match cstr(path) {
            Ok(s) => s,
            Err(_) => return -1,
        };
        let mut projects = RecentProjects::load(Path::new(path_str));
        projects.clear();
        match projects.save(Path::new(path_str)) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }
}

// ─── Free helpers ──────────────────────────────────────────

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn config_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}
