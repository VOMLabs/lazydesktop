//! C ABI exports for the addon system (ADDON_SPEC §11).
//!
//! This is the ONLY module in the crate that may contain `unsafe` code; the
//! crate root carries `#![deny(unsafe_code)]` and this module opts out with
//! `#![allow(unsafe_code)]`. Every unsafe block here is one of the safety
//! patterns enumerated in §11.5.

#![allow(unsafe_code)]

use std::ffi::{c_char, c_int, c_void, CStr, CString};
use std::sync::{Arc, OnceLock};

use crate::error::AddonError;
use crate::manifest::HOST_API_VERSION;
use crate::registry::AddonRegistry;

// ─── Result codes (must match addons.h) ────────────────────

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(non_camel_case_types)] // matches the C enum names in addons.h exactly
pub enum lda_result {
    LDA_OK = 0,
    LDA_ERR_INVALID_ARGUMENT = 1,
    LDA_ERR_INVALID_ADDON = 2,
    LDA_ERR_INVALID_MANIFEST = 3,
    LDA_ERR_ARCHIVE = 4,
    LDA_ERR_PATH_SECURITY = 5,
    LDA_ERR_PROVIDER = 6,
    LDA_ERR_LUA_RUNTIME = 7,
    LDA_ERR_FFI = 8,
    LDA_ERR_INTERNAL = 9,
}

fn result_code(e: &AddonError) -> lda_result {
    match e {
        AddonError::InvalidAddon { .. } => lda_result::LDA_ERR_INVALID_ADDON,
        AddonError::InvalidManifest { .. } => lda_result::LDA_ERR_INVALID_MANIFEST,
        AddonError::ArchiveError { .. } => lda_result::LDA_ERR_ARCHIVE,
        AddonError::PathSecurityViolation { .. } => lda_result::LDA_ERR_PATH_SECURITY,
        AddonError::ProviderError { .. } => lda_result::LDA_ERR_PROVIDER,
        AddonError::LuaRuntimeError { .. } => lda_result::LDA_ERR_LUA_RUNTIME,
        AddonError::FfiError { .. } => lda_result::LDA_ERR_FFI,
        AddonError::Internal { .. } => lda_result::LDA_ERR_INTERNAL,
    }
}

// ─── Opaque handle ─────────────────────────────────────────

/// Opaque handle. Internally a heap-allocated `Arc<AddonRegistry>`.
#[repr(C)]
pub struct lda_registry {
    _private: [u8; 0],
}

unsafe fn handle<'a>(r: *mut lda_registry) -> Result<&'a Arc<AddonRegistry>, lda_result> {
    if r.is_null() {
        return Err(lda_result::LDA_ERR_INVALID_ARGUMENT);
    }
    // SAFETY: created only by lda_registry_create via Box::into_raw; the
    // caller promises to pass a live handle and to destroy it exactly once.
    Ok(unsafe { &*(r as *const Arc<AddonRegistry>) })
}

// ─── Panic guard ───────────────────────────────────────────

unsafe fn catch(f: impl FnOnce() -> Result<(), lda_result>) -> lda_result {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(Ok(())) => lda_result::LDA_OK,
        Ok(Err(e)) => e,
        Err(_) => lda_result::LDA_ERR_FFI,
    }
}

// ─── String helpers ────────────────────────────────────────

unsafe fn cstr<'a>(p: *const c_char) -> Result<&'a str, lda_result> {
    if p.is_null() {
        return Err(lda_result::LDA_ERR_INVALID_ARGUMENT);
    }
    // SAFETY: caller supplies a NUL-terminated string.
    let c = unsafe { CStr::from_ptr(p) };
    c.to_str().map_err(|_| lda_result::LDA_ERR_INVALID_ARGUMENT)
}

/// Write a Rust string into `*dst` as a NUL-terminated heap string the caller
/// frees with `lda_free_string`.
unsafe fn write_cstring(dst: *mut *mut c_char, s: String) -> Result<(), lda_result> {
    if dst.is_null() {
        return Err(lda_result::LDA_ERR_INVALID_ARGUMENT);
    }
    let c = CString::new(s).unwrap_or_else(|_| CString::new("<invalid utf-8>").unwrap());
    unsafe { *dst = c.into_raw() };
    Ok(())
}

// ─── Byte-buffer helpers ───────────────────────────────────
//
// Byte buffers returned to C are prefixed with a little-endian u64 length so
// that `lda_free_bytes` can reconstruct the allocation from the pointer alone.

const BUF_HDR: usize = std::mem::size_of::<u64>();

fn alloc_bytes(data: Vec<u8>) -> (*mut u8, usize) {
    let data_len = data.len();
    let mut buf = Vec::with_capacity(data_len + BUF_HDR);
    buf.extend_from_slice(&(data_len as u64).to_le_bytes());
    buf.extend_from_slice(&data);
    let ptr = buf.as_mut_ptr().wrapping_add(BUF_HDR);
    std::mem::forget(buf); // ownership moves to C
    (ptr, data_len)
}

unsafe fn free_bytes(ptr: *mut u8) {
    if ptr.is_null() {
        return;
    }
    // SAFETY: ptr must have been produced by alloc_bytes.
    unsafe {
        let base = ptr.sub(BUF_HDR);
        let len = u64::from_le_bytes((base as *const [u8; BUF_HDR]).read()) as usize;
        let cap = len + BUF_HDR;
        drop(Vec::from_raw_parts(base, cap, cap));
    }
}

unsafe fn write_bytes(
    dst: *mut *mut u8,
    out_len: *mut usize,
    data: Vec<u8>,
) -> Result<(), lda_result> {
    if dst.is_null() || out_len.is_null() {
        return Err(lda_result::LDA_ERR_INVALID_ARGUMENT);
    }
    let (ptr, len) = alloc_bytes(data);
    unsafe {
        *dst = ptr;
        *out_len = len;
    }
    Ok(())
}

// ─── Logging sink ──────────────────────────────────────────

/// Callback installed by `lda_set_log_sink`. Contract (§15): invoked on the
/// calling thread, never re-enters the lda API, never blocks, never uses the
/// Rust allocator.
pub type LdaLogFn = unsafe extern "C" fn(c_int, *const c_char, *mut c_void);

struct LogSink {
    cb: Option<LdaLogFn>,
    userdata: *mut c_void,
}

// SAFETY: the callback contract requires `userdata` to remain valid for the
// process lifetime once installed; we only pass it back to the callback and
// never dereference it. `cb` is a function pointer (Send + Sync).
unsafe impl Send for LogSink {}
unsafe impl Sync for LogSink {}

static LOG_SINK: OnceLock<LogSink> = OnceLock::new();

#[allow(dead_code)] // exercised by lib::emit_log (host logging plumbing)
pub(crate) fn emit_log_impl(level: u8, msg: &str) {
    if let Some(sink) = LOG_SINK.get() {
        if let Some(cb) = sink.cb {
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let cmsg =
                    CString::new(msg).unwrap_or_else(|_| CString::new("<bad message>").unwrap());
                // SAFETY: the callback was installed by the host; the message
                // is a valid NUL-terminated C string for the duration of the call.
                unsafe {
                    cb(level as c_int, cmsg.as_ptr(), sink.userdata);
                }
            }));
        }
    }
}

// ─── Version / memory ──────────────────────────────────────

#[no_mangle]
/// Query the host addon API version implemented by this library.
///
/// # Safety
///
/// `major` and `minor` must either be null or point to writable `u32`
/// storage for the duration of the call.
pub unsafe extern "C" fn lda_api_version(major: *mut u32, minor: *mut u32) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if !major.is_null() {
            unsafe { *major = HOST_API_VERSION.0 };
        }
        if !minor.is_null() {
            unsafe { *minor = HOST_API_VERSION.1 };
        }
    }));
}

#[no_mangle]
pub extern "C" fn lda_abi_version() -> *const c_char {
    // Static, never freed by the caller.
    c"1".as_ptr()
}

#[no_mangle]
/// Free a string previously returned by the library.
///
/// # Safety
///
/// `s` must be null or a pointer previously produced by `write_cstring`
/// (i.e. returned through an `out_*` string argument) and must not be freed
/// more than once.
pub unsafe extern "C" fn lda_free_string(s: *mut c_char) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        if !s.is_null() {
            // SAFETY: s must have been produced by write_cstring.
            unsafe {
                drop(CString::from_raw(s));
            }
        }
    }));
}

#[no_mangle]
/// Free a byte buffer previously returned by the library.
///
/// # Safety
///
/// `p` must be null or a pointer previously produced by `alloc_bytes`
/// (i.e. returned through an `out_bytes` argument) and must not be freed
/// more than once.
pub unsafe extern "C" fn lda_free_bytes(p: *mut u8) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| unsafe {
        free_bytes(p);
    }));
}

#[no_mangle]
/// Install the host logging callback.
///
/// # Safety
///
/// `userdata` must remain valid for the process lifetime once installed (it
/// is stored globally and passed back to the callback); `cb` must conform to
/// the [`LdaLogFn`] contract (never re-enters the API, never blocks, never
/// uses the Rust allocator). Installing a second sink is a no-op.
pub unsafe extern "C" fn lda_set_log_sink(cb: Option<LdaLogFn>, userdata: *mut c_void) {
    let _ = LOG_SINK.set(LogSink { cb, userdata });
}

// ─── Registry lifecycle ────────────────────────────────────

#[no_mangle]
pub extern "C" fn lda_registry_create() -> *mut lda_registry {
    match std::panic::catch_unwind(AddonRegistry::new) {
        Ok(arc) => {
            let raw = Box::into_raw(Box::new(arc));
            raw as *mut lda_registry
        }
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
/// Destroy a registry previously created by [`lda_registry_create`] and null
/// the caller's handle.
///
/// # Safety
///
/// `r` must point to a valid `*mut lda_registry` slot whose value is either
/// null or was produced by [`lda_registry_create`]. After the call the slot is
/// nulled so a second destroy is a no-op.
pub unsafe extern "C" fn lda_registry_destroy(r: *mut *mut lda_registry) -> lda_result {
    unsafe {
        catch(|| {
            if r.is_null() {
                return Err(lda_result::LDA_ERR_INVALID_ARGUMENT);
            }
            let handle = *r;
            if !handle.is_null() {
                // SAFETY: created by lda_registry_create; we null the caller's
                // handle so a second destroy is a no-op.
                drop(Box::from_raw(handle as *mut Arc<AddonRegistry>));
                *r = std::ptr::null_mut();
            }
            Ok(())
        })
    }
}

#[no_mangle]
/// Set the host application version used for manifest compatibility checks.
///
/// # Safety
///
/// `r` must be a live registry handle from [`lda_registry_create`]; `version`
/// must be a NUL-terminated UTF-8 string for the duration of the call.
pub unsafe extern "C" fn lda_registry_set_app_version(
    r: *mut lda_registry,
    version: *const c_char,
) -> lda_result {
    unsafe {
        catch(|| {
            let reg = handle(r)?;
            let v = cstr(version)?;
            reg.set_app_version(v).map_err(|e| result_code(&e))
        })
    }
}

#[no_mangle]
/// Add an addon root directory.
///
/// # Safety
///
/// `r` must be a live registry handle from [`lda_registry_create`]; `path`
/// must be a NUL-terminated UTF-8 string for the duration of the call.
pub unsafe extern "C" fn lda_registry_add_root(
    r: *mut lda_registry,
    path: *const c_char,
    user_priority: c_int,
) -> lda_result {
    unsafe {
        catch(|| {
            let reg = handle(r)?;
            let p = cstr(path)?;
            reg.add_root(p, user_priority != 0)
                .map_err(|e| result_code(&e))
        })
    }
}

#[no_mangle]
/// Load/scan all roots and rebuild the registry snapshot.
///
/// # Safety
///
/// `r` must be a live registry handle from [`lda_registry_create`].
pub unsafe extern "C" fn lda_registry_load(r: *mut lda_registry) -> lda_result {
    unsafe {
        catch(|| {
            let reg = handle(r)?;
            reg.load().map_err(|e| result_code(&e))
        })
    }
}

#[no_mangle]
/// Re-scan all roots (alias of [`lda_registry_load`]).
///
/// # Safety
///
/// `r` must be a live registry handle from [`lda_registry_create`].
pub unsafe extern "C" fn lda_registry_refresh(r: *mut lda_registry) -> lda_result {
    unsafe {
        catch(|| {
            let reg = handle(r)?;
            reg.refresh().map_err(|e| result_code(&e))
        })
    }
}

// ─── Listing & lookup ──────────────────────────────────────

#[no_mangle]
/// Serialize the full addon list to JSON into a caller-freed string.
///
/// # Safety
///
/// `r` must be a live registry handle from [`lda_registry_create`];
/// `out_json` must point to writable `*mut c_char` storage. The caller frees
/// the result with [`lda_free_string`].
pub unsafe extern "C" fn lda_registry_list(
    r: *mut lda_registry,
    out_json: *mut *mut c_char,
) -> lda_result {
    unsafe {
        catch(|| {
            let reg = handle(r)?;
            let list = reg.list();
            let json = crate::json::list_json(&list);
            write_cstring(out_json, json)
        })
    }
}

#[no_mangle]
/// Serialize a single addon (or an error JSON) into a caller-freed string.
///
/// # Safety
///
/// `r` must be a live registry handle from [`lda_registry_create`]; `id` must
/// be a NUL-terminated UTF-8 string; `out_json` must point to writable
/// `*mut c_char` storage. The caller frees the result with
/// [`lda_free_string`].
pub unsafe extern "C" fn lda_registry_get(
    r: *mut lda_registry,
    id: *const c_char,
    out_json: *mut *mut c_char,
) -> lda_result {
    unsafe {
        catch(|| {
            let reg = handle(r)?;
            let id = cstr(id)?;
            match reg.get(id) {
                Some(d) => write_cstring(out_json, crate::json::single_json(&d)),
                None => {
                    let err =
                        AddonError::invalid_addon(Some(id.to_string()), "addon is not installed");
                    write_cstring(out_json, err.to_json())?;
                    Err(result_code(&err))
                }
            }
        })
    }
}

// ─── Install / uninstall ───────────────────────────────────

#[no_mangle]
/// Install an addon from a package path.
///
/// # Safety
///
/// `r` must be a live registry handle from [`lda_registry_create`];
/// `source_path` must be a NUL-terminated UTF-8 string; `out_json` must point
/// to writable `*mut c_char` storage freed with [`lda_free_string`].
pub unsafe extern "C" fn lda_registry_install(
    r: *mut lda_registry,
    source_path: *const c_char,
    out_json: *mut *mut c_char,
) -> lda_result {
    unsafe {
        catch(|| {
            let reg = handle(r)?;
            let path = cstr(source_path)?;
            let path = std::path::Path::new(path);
            match reg.install(path) {
                Ok(d) => write_cstring(out_json, crate::json::single_json(&d)),
                Err(e) => {
                    write_cstring(out_json, e.to_json())?;
                    Err(result_code(&e))
                }
            }
        })
    }
}

#[no_mangle]
/// Uninstall an addon by id.
///
/// # Safety
///
/// `r` must be a live registry handle from [`lda_registry_create`]; `id` must
/// be a NUL-terminated UTF-8 string for the duration of the call.
pub unsafe extern "C" fn lda_registry_uninstall(
    r: *mut lda_registry,
    id: *const c_char,
) -> lda_result {
    unsafe {
        catch(|| {
            let reg = handle(r)?;
            let id = cstr(id)?;
            reg.uninstall(id).map_err(|e| result_code(&e))
        })
    }
}

// ─── Resource access ───────────────────────────────────────

#[no_mangle]
/// Test whether an addon has an asset at a relative path.
///
/// # Safety
///
/// `r` must be a live registry handle from [`lda_registry_create`]; `id` and
/// `rel_path` must be NUL-terminated UTF-8 strings; `out_has` must point to
/// writable `c_int` storage.
pub unsafe extern "C" fn lda_addon_has_asset(
    r: *mut lda_registry,
    id: *const c_char,
    rel_path: *const c_char,
    out_has: *mut c_int,
) -> lda_result {
    unsafe {
        catch(|| {
            if out_has.is_null() {
                return Err(lda_result::LDA_ERR_INVALID_ARGUMENT);
            }
            let reg = handle(r)?;
            let id = cstr(id)?;
            let rel = cstr(rel_path)?;
            let has = reg.has_asset(id, rel);
            *out_has = if has { 1 } else { 0 };
            Ok(())
        })
    }
}

#[no_mangle]
/// Read an addon asset into a caller-freed byte buffer.
///
/// # Safety
///
/// `r` must be a live registry handle from [`lda_registry_create`]; `id` and
/// `rel_path` must be NUL-terminated UTF-8 strings; `out_bytes` and `out_len`
/// must point to writable storage. The caller frees the buffer with
/// [`lda_free_bytes`].
pub unsafe extern "C" fn lda_addon_read_asset(
    r: *mut lda_registry,
    id: *const c_char,
    rel_path: *const c_char,
    out_bytes: *mut *mut u8,
    out_len: *mut usize,
) -> lda_result {
    unsafe {
        catch(|| {
            let reg = handle(r)?;
            let id = cstr(id)?;
            let rel = cstr(rel_path)?;
            match reg.read_asset(id, rel) {
                Ok(bytes) => write_bytes(out_bytes, out_len, bytes),
                Err(e) => Err(result_code(&e)),
            }
        })
    }
}

#[no_mangle]
/// Read an addon's entry script into a caller-freed byte buffer.
///
/// # Safety
///
/// `r` must be a live registry handle from [`lda_registry_create`]; `id` must
/// be a NUL-terminated UTF-8 string; `out_bytes` and `out_len` must point to
/// writable storage. The caller frees the buffer with [`lda_free_bytes`].
pub unsafe extern "C" fn lda_addon_entry_script(
    r: *mut lda_registry,
    id: *const c_char,
    out_bytes: *mut *mut u8,
    out_len: *mut usize,
) -> lda_result {
    unsafe {
        catch(|| {
            let reg = handle(r)?;
            let id = cstr(id)?;
            match reg.entry_script(id) {
                Ok(bytes) => write_bytes(out_bytes, out_len, bytes),
                Err(e) => Err(result_code(&e)),
            }
        })
    }
}

#[no_mangle]
/// Resolve an addon's storage directory into a caller-freed string.
///
/// # Safety
///
/// `r` must be a live registry handle from [`lda_registry_create`]; `id` must
/// be a NUL-terminated UTF-8 string; `out_path` must point to writable
/// `*mut c_char` storage freed with [`lda_free_string`].
pub unsafe extern "C" fn lda_addon_storage_dir(
    r: *mut lda_registry,
    id: *const c_char,
    out_path: *mut *mut c_char,
) -> lda_result {
    unsafe {
        catch(|| {
            let reg = handle(r)?;
            let id = cstr(id)?;
            match reg.storage_dir(id) {
                Ok(p) => write_cstring(out_path, p.to_string_lossy().into_owned()),
                Err(e) => {
                    write_cstring(out_path, e.to_json())?;
                    Err(result_code(&e))
                }
            }
        })
    }
}
