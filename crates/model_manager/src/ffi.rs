use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use crate::download;
use crate::inference;
use crate::types::{DownloadHandle, ModelManager, ModelManagerInner};

pub type ProgressCb = extern "C" fn(i32, i64, i64, *mut std::ffi::c_void);
pub type TokenCb = extern "C" fn(*const c_char, *mut std::ffi::c_void);
pub type ErrorCb = extern "C" fn(*const c_char, *mut std::ffi::c_void);
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
        inner: std::sync::Mutex::new(ModelManagerInner {
            downloads: Vec::new(),
            next_download_id: 1,
            runtime: rt,
        }),
    };

    Box::into_raw(Box::new(mm))
}

#[no_mangle]
pub extern "C" fn mm_destroy(mm: *mut ModelManager) {
    if mm.is_null() {
        return;
    }
    unsafe {
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
        let inner = mm.inner.lock().unwrap();
        inner.runtime.spawn(async move {
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
        let inner = mm.inner.lock().unwrap();
        inner.runtime.spawn(async move {
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

    let models_raw = crate::discovery::discover_models(
        mm.models_dir.to_str().unwrap_or(""),
    )
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

#[no_mangle]
pub extern "C" fn mm_stream_inference(
    mm: *mut ModelManager,
    model_path: *const c_char,
    prompt: *const c_char,
    n_gpu_layers: i32,
    on_token: Option<TokenCb>,
    on_error: Option<ErrorCb>,
    on_cancelled: Option<CancelCb>,
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

    let cancel = Arc::new(AtomicBool::new(false));

    let ud_val_token = user_data as usize;
    let on_token_cb: Option<Box<dyn Fn(&str) + Send>> = on_token.map(|cb| {
        let boxed: Box<dyn Fn(&str)> = Box::new(move |text: &str| {
            if let Ok(cstr) = CString::new(text) {
                cb(cstr.as_ptr(), ud_val_token as *mut std::ffi::c_void);
            }
        });
        unsafe { std::mem::transmute(boxed) }
    });

    let ud_val_error = user_data as usize;
    let on_error_cb: Option<Box<dyn Fn(&str) + Send>> = on_error.map(|cb| {
        let boxed: Box<dyn Fn(&str)> = Box::new(move |text: &str| {
            if let Ok(cstr) = CString::new(text) {
                cb(cstr.as_ptr(), ud_val_error as *mut std::ffi::c_void);
            }
        });
        unsafe { std::mem::transmute(boxed) }
    });

    // Poll cancellation from C++ side
    if let Some(cancel_cb) = on_cancelled {
        let ud_val_cancel = user_data as usize;
        let cancel_flag = cancel.clone();
        std::thread::Builder::new()
            .name("mm-cancel-poll".into())
            .spawn(move || {
                while !cancel_flag.load(Ordering::Relaxed) {
                    if cancel_cb(ud_val_cancel as *mut std::ffi::c_void) {
                        cancel_flag.store(true, Ordering::Relaxed);
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
            })
            .expect("Failed to spawn cancel poll thread");
    }

    inference::run_inference(
        path,
        prompt_str,
        n_gpu_layers,
        cancel,
        on_token_cb.unwrap_or_else(|| Box::new(|_| {})),
        on_error_cb.unwrap_or_else(|| Box::new(|_| {})),
    );

    true
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
    let result = crate::discovery::discover_models(
        mm.models_dir.to_str().unwrap_or(""),
    )
    .unwrap_or_else(|_| "[]".to_string());

    CString::new(result)
        .unwrap_or_default()
        .into_raw()
}
