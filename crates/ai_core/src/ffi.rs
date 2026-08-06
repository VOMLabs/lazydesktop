use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::commit_message::CommitContext;
use crate::download;
use crate::inference::run_inference_blocking;
use crate::types::{DownloadHandle, ModelManager, ModelManagerInner};

pub type ProgressCb = extern "C" fn(i32, i64, i64, *mut std::ffi::c_void);
pub type TokenCb = extern "C" fn(*const c_char, *mut std::ffi::c_void);
pub type ErrorCb = extern "C" fn(*const c_char, *mut std::ffi::c_void);
pub type FinishCb = extern "C" fn(*const c_char, *mut std::ffi::c_void);
pub type CancelCb = extern "C" fn(*mut std::ffi::c_void) -> bool;

#[no_mangle]
pub extern "C" fn mm_init(
    models_dir: *const c_char,
    config_path: *const c_char,
) -> *mut ModelManager {
    let models = unsafe { CStr::from_ptr(models_dir) }
        .to_str()
        .unwrap_or("")
        .to_string();
    let config = unsafe { CStr::from_ptr(config_path) }
        .to_str()
        .unwrap_or("")
        .to_string();

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .expect("Failed to build tokio runtime");

    let mm = ModelManager {
        models_dir: PathBuf::from(&models),
        config_path: PathBuf::from(&config),
        runtime: rt,
        inner: std::sync::Mutex::new(ModelManagerInner::default()),
    };

    Box::into_raw(Box::new(mm))
}

#[no_mangle]
pub extern "C" fn mm_destroy(mm: *mut ModelManager) {
    if mm.is_null() {
        return;
    }
    unsafe {
        // Mark any in-flight inference as cancelled; the worker thread only
        // touches Arc flags, never the manager itself, so it is safe to drop.
        let mgr = &*mm;
        if let Ok(inner) = mgr.inner.lock() {
            inner.inference_cancel.store(true, Ordering::Relaxed);
        }
        let _ = Box::from_raw(mm);
    }
}

#[no_mangle]
pub extern "C" fn mm_download_model(
    mm: *mut ModelManager,
    url: *const c_char,
    dest_path: *const c_char,
    expected_sha256: *const c_char,
    progress_cb: Option<ProgressCb>,
    user_data: *mut std::ffi::c_void,
) -> i32 {
    if mm.is_null() {
        return -1;
    }
    let mm = unsafe { &*mm };
    let url = unsafe { CStr::from_ptr(url) }
        .to_str()
        .unwrap_or("")
        .to_string();
    let dest = unsafe { CStr::from_ptr(dest_path) }
        .to_str()
        .unwrap_or("")
        .to_string();
    let expected = if expected_sha256.is_null() {
        None
    } else {
        unsafe { CStr::from_ptr(expected_sha256) }
            .to_str()
            .ok()
            .map(|s| s.to_string())
    };

    let cancel_flag = Arc::new(AtomicBool::new(false));
    let cancel_clone = cancel_flag.clone();
    let dest_path = PathBuf::from(&dest);
    let url_clone = url.clone();

    let download_id;
    {
        let mut inner = mm.inner.lock().unwrap();
        download_id = inner.next_download_id;
        inner.next_download_id += 1;

        inner.downloads.push((
            download_id,
            DownloadHandle {
                cancel_flag: cancel_clone.clone(),
            },
        ));
    }

    if let Some(cb) = progress_cb {
        let id = download_id;
        let ud_val = user_data as usize;
        let progress: Box<dyn Fn(i64, i64) + Send> = Box::new(move |rcv, tot| {
            cb(id, rcv, tot, ud_val as *mut std::ffi::c_void);
        });
        mm.runtime.spawn(async move {
            let result = download::download_file(
                &url_clone,
                &dest_path,
                cancel_clone,
                Some(progress),
                expected.as_deref(),
            )
            .await;

            if let Err(ref e) = result {
                tracing::error!("Download failed: {e}");
                let _ = tokio::fs::remove_file(&dest_path).await;
            }
        });
    } else {
        mm.runtime.spawn(async move {
            let result = download::download_file(
                &url_clone,
                &dest_path,
                cancel_clone,
                None::<Box<dyn Fn(i64, i64) + Send>>,
                expected.as_deref(),
            )
            .await;

            if let Err(ref e) = result {
                tracing::error!("Download failed: {e}");
                let _ = tokio::fs::remove_file(&dest_path).await;
            }
        });
    }

    download_id
}

#[no_mangle]
pub extern "C" fn mm_cancel_download(mm: *mut ModelManager, download_id: i32) {
    if mm.is_null() {
        return;
    }
    let mm = unsafe { &*mm };
    let mut inner = mm.inner.lock().unwrap();
    if let Some(pos) = inner.downloads.iter().position(|(id, _)| *id == download_id) {
        let (_, handle) = inner.downloads.remove(pos);
        handle.cancel_flag.store(true, Ordering::Relaxed);
    }
}

#[no_mangle]
pub extern "C" fn mm_list_local_models(mm: *mut ModelManager) -> *mut c_char {
    if mm.is_null() {
        return std::ptr::null_mut();
    }
    let mm = unsafe { &*mm };

    let models_raw = crate::discovery::discover_models(mm.models_dir.to_str().unwrap_or(""))
        .unwrap_or_else(|_| "[]".to_string());

    let models = if let Ok(mut parsed) =
        serde_json::from_str::<serde_json::Value>(&models_raw)
    {
        if let Some(arr) = parsed.as_array_mut() {
            for item in arr.iter_mut() {
                let path = item["path"].as_str().unwrap_or("");
                let downloaded = std::path::Path::new(path).exists();
                item["downloaded"] = serde_json::Value::Bool(downloaded);
            }
        }
        serde_json::to_string(&parsed).unwrap_or_else(|_| "[]".to_string())
    } else {
        models_raw
    };

    CString::new(models)
        .unwrap_or_default()
        .into_raw()
}

#[no_mangle]
pub extern "C" fn mm_delete_model(mm: *mut ModelManager, path: *const c_char) -> bool {
    if mm.is_null() || path.is_null() {
        return false;
    }
    let p = unsafe { CStr::from_ptr(path) }
        .to_str()
        .unwrap_or("");
    let p_path = std::path::Path::new(p);
    if p_path.exists() {
        std::fs::remove_file(p_path).is_ok()
    } else {
        false
    }
}

/// Spawns an inference worker thread and wires up streaming, error, finish and
/// cancellation callbacks. Returns false when the manager is null or another
/// inference is already running.
///
/// The worker receives the shared cancel flag and a token sink and must return
/// the full generated text. The worker thread only touches `Arc` flags (never
/// the manager), so it is safe to destroy the manager while it runs.
fn start_inference<F>(
    mm: *mut ModelManager,
    on_token: Option<TokenCb>,
    on_error: Option<ErrorCb>,
    on_cancelled: Option<CancelCb>,
    on_finish: Option<FinishCb>,
    user_data: *mut std::ffi::c_void,
    worker: F,
) -> bool
where
    F: FnOnce(&AtomicBool, &dyn Fn(&str)) -> Result<String, String> + Send + 'static,
{
    if mm.is_null() {
        return false;
    }
    let mm_ref = unsafe { &*mm };

    let cancel;
    let running;
    {
        let inner = mm_ref.inner.lock().unwrap();
        if inner.inference_running.load(Ordering::Relaxed) {
            return false;
        }
        cancel = inner.inference_cancel.clone();
        running = inner.inference_running.clone();
        cancel.store(false, Ordering::Relaxed);
        running.store(true, Ordering::Relaxed);
    }

    let on_token_cb: Option<Box<dyn Fn(&str) + Send>> = on_token.map(|cb| {
        let ud = user_data as usize;
        let boxed: Box<dyn Fn(&str) + Send> = Box::new(move |text: &str| {
            if let Ok(cstr) = CString::new(text) {
                cb(cstr.as_ptr(), ud as *mut std::ffi::c_void);
            }
        });
        boxed
    });
    let on_error_cb: Option<Box<dyn Fn(&str) + Send>> = on_error.map(|cb| {
        let ud = user_data as usize;
        let boxed: Box<dyn Fn(&str) + Send> = Box::new(move |text: &str| {
            if let Ok(cstr) = CString::new(text) {
                cb(cstr.as_ptr(), ud as *mut std::ffi::c_void);
            }
        });
        boxed
    });
    let on_finish_cb: Option<Box<dyn Fn(&str) + Send>> = on_finish.map(|cb| {
        let ud = user_data as usize;
        let boxed: Box<dyn Fn(&str) + Send> = Box::new(move |text: &str| {
            if let Ok(cstr) = CString::new(text) {
                cb(cstr.as_ptr(), ud as *mut std::ffi::c_void);
            }
        });
        boxed
    });

    if let Some(cancel_cb) = on_cancelled {
        let ud = user_data as usize;
        let flag = cancel.clone();
        let handle = std::thread::Builder::new()
            .name("mm-cancel-poll".into())
            .spawn(move || {
                while !flag.load(Ordering::Relaxed) {
                    if cancel_cb(ud as *mut std::ffi::c_void) {
                        flag.store(true, Ordering::Relaxed);
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            })
            .ok();
        mm_ref.inner.lock().unwrap().cancel_poll_thread = handle;
    }

    let running_clone = running.clone();
    let handle = std::thread::Builder::new()
        .name("ai-inference".into())
        .spawn(move || {
            let token_sink: Box<dyn Fn(&str) + Send> =
                on_token_cb.unwrap_or_else(|| Box::new(|_| {}));
            let error_sink: Box<dyn Fn(&str) + Send> =
                on_error_cb.unwrap_or_else(|| Box::new(|_| {}));

            let result = worker(&cancel, &token_sink);
            let message = match result {
                Ok(text) => text,
                Err(e) => {
                    tracing::error!("Inference failed: {e}");
                    error_sink(&e);
                    String::new()
                }
            };
            if let Some(finish) = on_finish_cb {
                finish(&message);
            }
            // Stop the cancel-poll thread and release the busy slot.
            cancel.store(true, Ordering::Relaxed);
            running_clone.store(false, Ordering::Relaxed);
        })
        .ok();

    match handle {
        Some(h) => {
            mm_ref.inner.lock().unwrap().inference_thread = Some(h);
            true
        }
        None => {
            running.store(false, Ordering::Relaxed);
            false
        }
    }
}

#[no_mangle]
pub extern "C" fn mm_stream_inference(
    mm: *mut ModelManager,
    model_path: *const c_char,
    prompt: *const c_char,
    n_gpu_layers: i32,
    on_token: Option<TokenCb>,
    on_error: Option<ErrorCb>,
    on_cancelled: Option<CancelCb>,
    on_finish: Option<FinishCb>,
    user_data: *mut std::ffi::c_void,
) -> bool {
    if mm.is_null() || model_path.is_null() || prompt.is_null() {
        return false;
    }

    let path = unsafe { CStr::from_ptr(model_path) }
        .to_str()
        .unwrap_or("")
        .to_string();
    let prompt_str = unsafe { CStr::from_ptr(prompt) }
        .to_str()
        .unwrap_or("")
        .to_string();

    start_inference(mm, on_token, on_error, on_cancelled, on_finish, user_data, move |cancel, on_token| {
        run_inference_blocking(&path, &prompt_str, n_gpu_layers, &cancel, on_token)
    })
}

/// Generates a Conventional Commits message for the changes described in
/// `context_json` (see `ai_core.h` for the schema). Streams raw tokens through
/// `on_token` and delivers the normalized message through `on_finish`.
#[no_mangle]
pub extern "C" fn mm_generate_commit_message(
    mm: *mut ModelManager,
    model_path: *const c_char,
    context_json: *const c_char,
    n_gpu_layers: i32,
    on_token: Option<TokenCb>,
    on_error: Option<ErrorCb>,
    on_cancelled: Option<CancelCb>,
    on_finish: Option<FinishCb>,
    user_data: *mut std::ffi::c_void,
) -> bool {
    if mm.is_null() || model_path.is_null() || context_json.is_null() {
        return false;
    }

    let path = unsafe { CStr::from_ptr(model_path) }
        .to_str()
        .unwrap_or("")
        .to_string();
    let ctx_json = unsafe { CStr::from_ptr(context_json) }
        .to_str()
        .unwrap_or("")
        .to_string();

    let ctx: CommitContext = match serde_json::from_str(&ctx_json) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("mm_generate_commit_message: invalid context JSON: {e}");
            return false;
        }
    };

    start_inference(
        mm,
        on_token,
        on_error,
        on_cancelled,
        on_finish,
        user_data,
        move |cancel, on_token| {
            crate::commit_message::generate_commit_message(
                &ctx,
                &path,
                n_gpu_layers,
                &cancel,
                on_token,
            )
        },
    )
}

#[no_mangle]
pub extern "C" fn mm_free_string(s: *mut c_char) {
    if s.is_null() {
        return;
    }
    unsafe {
        let _ = CString::from_raw(s);
    }
}

#[no_mangle]
pub extern "C" fn mm_discover_models(mm: *mut ModelManager) -> *mut c_char {
    if mm.is_null() {
        return std::ptr::null_mut();
    }
    let mm = unsafe { &*mm };
    let result = crate::discovery::discover_models(mm.models_dir.to_str().unwrap_or(""))
        .unwrap_or_else(|_| "[]".to_string());

    CString::new(result)
        .unwrap_or_default()
        .into_raw()
}
