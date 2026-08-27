//! C ABI exports for the file watcher.
//!
//! This module is the single location for `unsafe` code in the crate.

#![allow(unsafe_code)]

use std::ffi::{c_char, c_int, CStr, CString};
use std::path::PathBuf;

use crate::{RepoWatcher, WatcherConfig, VcsKind, FileEvent};

// ─── Opaque handle ─────────────────────────────────────────

#[repr(C)]
pub struct watcher_handle {
    _private: [u8; 0],
}

// ─── Panic guard ───────────────────────────────────────────

unsafe fn catch(f: impl FnOnce() -> Result<(), c_int>) -> c_int {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(Ok(())) => 0,
        Ok(Err(e)) => e,
        Err(_) => -1,
    }
}

// ─── String helpers ────────────────────────────────────────

unsafe fn cstr<'a>(p: *const c_char) -> Result<&'a str, c_int> {
    if p.is_null() {
        return Err(-1);
    }
    let c = unsafe { CStr::from_ptr(p) };
    c.to_str().map_err(|_| -1)
}

// ─── Public C API ──────────────────────────────────────────

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn watcher_create(
    repo_path: *const c_char,
    vcs_kind: *const c_char,
    debounce_ms: u32,
    out_handle: *mut *mut watcher_handle,
) -> c_int {
    unsafe {
        catch(|| {
            if out_handle.is_null() {
                return Err(-1);
            }
            let path = cstr(repo_path)?;
            let kind_str = cstr(vcs_kind)?;
            let vcs = VcsKind::from_str(kind_str).ok_or(-1)?;
            let debounce = if debounce_ms == 0 {
                std::time::Duration::from_secs(2)
            } else {
                std::time::Duration::from_millis(debounce_ms as u64)
            };
            let config = WatcherConfig { debounce, vcs_kind: vcs };
            let watcher = RepoWatcher::watch(PathBuf::from(path).as_path(), &config)
                .map_err(|_| -1)?;
            let boxed = Box::new(watcher);
            let raw = Box::into_raw(boxed) as *mut watcher_handle;
            *out_handle = raw;
            Ok(())
        })
    }
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn watcher_destroy(handle: *mut *mut watcher_handle) -> c_int {
    unsafe {
        catch(|| {
            if handle.is_null() {
                return Err(-1);
            }
            let h = *handle;
            if !h.is_null() {
                drop(Box::from_raw(h as *mut RepoWatcher));
                *handle = std::ptr::null_mut();
            }
            Ok(())
        })
    }
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn watcher_poll(
    handle: *mut watcher_handle,
    timeout_ms: u32,
    out_event: *mut *mut watcher_event,
) -> c_int {
    unsafe {
        catch(|| {
            if handle.is_null() || out_event.is_null() {
                return Err(-1);
            }
            let watcher = &*(handle as *const RepoWatcher);
            let timeout = if timeout_ms == 0 {
                // Block indefinitely — use a channel recv instead.
                match watcher.recv() {
                    Ok(event) => {
                        *out_event = Box::into_raw(Box::new(event_to_ffi(event)));
                        return Ok(());
                    }
                    Err(_) => return Err(-1),
                }
            } else {
                std::time::Duration::from_millis(timeout_ms as u64)
            };
            // Poll with timeout via try_recv in a loop.
            let deadline = std::time::Instant::now() + timeout;
            loop {
                match watcher.try_recv() {
                    Ok(event) => {
                        *out_event = Box::into_raw(Box::new(event_to_ffi(event)));
                        return Ok(());
                    }
                    Err(std::sync::mpsc::TryRecvError::Empty) => {
                        if std::time::Instant::now() >= deadline {
                            return Err(1); // timeout
                        }
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                    Err(std::sync::mpsc::TryRecvError::Disconnected) => return Err(-1),
                }
            }
        })
    }
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn watcher_free_event(event: *mut watcher_event) {
    if !event.is_null() {
        unsafe {
            drop(Box::from_raw(event));
        }
    }
}

#[no_mangle]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn watcher_free_string(s: *mut c_char) {
    if !s.is_null() {
        unsafe {
            drop(CString::from_raw(s));
        }
    }
}

// ─── Event mapping ─────────────────────────────────────────

#[repr(C)]
pub struct watcher_event {
    pub kind: watcher_event_kind,
    pub path: *mut c_char,
    pub rename_from: *mut c_char,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum watcher_event_kind {
    WATCHER_EVENT_CREATED = 0,
    WATCHER_EVENT_MODIFIED = 1,
    WATCHER_EVENT_REMOVED = 2,
    WATCHER_EVENT_RENAMED = 3,
    WATCHER_EVENT_RESCAN = 4,
}

impl Drop for watcher_event {
    fn drop(&mut self) {
        unsafe {
            if !self.path.is_null() {
                drop(CString::from_raw(self.path));
            }
            if !self.rename_from.is_null() {
                drop(CString::from_raw(self.rename_from));
            }
        }
    }
}

fn to_cstring(s: String) -> *mut c_char {
    CString::new(s).unwrap_or_else(|_| CString::new("<bad>").unwrap()).into_raw()
}

fn event_to_ffi(event: FileEvent) -> watcher_event {
    match event {
        FileEvent::Created(path) => watcher_event {
            kind: watcher_event_kind::WATCHER_EVENT_CREATED,
            path: to_cstring(path.to_string_lossy().into_owned()),
            rename_from: std::ptr::null_mut(),
        },
        FileEvent::Modified(path) => watcher_event {
            kind: watcher_event_kind::WATCHER_EVENT_MODIFIED,
            path: to_cstring(path.to_string_lossy().into_owned()),
            rename_from: std::ptr::null_mut(),
        },
        FileEvent::Removed(path) => watcher_event {
            kind: watcher_event_kind::WATCHER_EVENT_REMOVED,
            path: to_cstring(path.to_string_lossy().into_owned()),
            rename_from: std::ptr::null_mut(),
        },
        FileEvent::Renamed { from, to } => watcher_event {
            kind: watcher_event_kind::WATCHER_EVENT_RENAMED,
            path: to_cstring(to.to_string_lossy().into_owned()),
            rename_from: to_cstring(from.to_string_lossy().into_owned()),
        },
        FileEvent::Rescan => watcher_event {
            kind: watcher_event_kind::WATCHER_EVENT_RESCAN,
            path: std::ptr::null_mut(),
            rename_from: std::ptr::null_mut(),
        },
    }
}
