use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

pub type ProgressCallback = Box<dyn Fn(i64, i64) + Send>;
pub type TokenCallback = Box<dyn Fn(&str) + Send>;
pub type ErrorCallback = Box<dyn Fn(&str) + Send>;
pub type FinishCallback = Box<dyn Fn() + Send>;
pub type JsonCallback = Box<dyn Fn(&str) + Send>;

/// Handle for an in-flight model download.
pub struct DownloadHandle {
    pub cancel_flag: Arc<AtomicBool>,
}

/// Mutable state guarded by `ModelManager::inner`.
pub struct ModelManagerInner {
    pub downloads: Vec<(i32, DownloadHandle)>,
    pub next_download_id: i32,
    /// Set to cancel an active inference. Shared with the cancel-poll thread.
    pub inference_cancel: Arc<AtomicBool>,
    /// True while an inference worker thread is running (busy guard).
    pub inference_running: Arc<AtomicBool>,
    pub inference_thread: Option<std::thread::JoinHandle<()>>,
    pub cancel_poll_thread: Option<std::thread::JoinHandle<()>>,
}

impl Default for ModelManagerInner {
    fn default() -> Self {
        Self {
            downloads: Vec::new(),
            next_download_id: 1,
            inference_cancel: Arc::new(AtomicBool::new(false)),
            inference_running: Arc::new(AtomicBool::new(false)),
            inference_thread: None,
            cancel_poll_thread: None,
        }
    }
}

/// The opaque AI core handle handed to the host application. Owns the tokio
/// runtime so async VCS/download work never blocks the Qt main thread.
pub struct ModelManager {
    pub models_dir: PathBuf,
    pub config_path: PathBuf,
    pub runtime: tokio::runtime::Runtime,
    pub inner: Mutex<ModelManagerInner>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelMetadata {
    pub name: String,
    pub url: String,
    pub path: String,
    pub size_bytes: i64,
    pub size_label: String,
    pub sha256: Option<String>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct DiscoveredModel {
    pub name: String,
    pub url: String,
    pub path: String,
    pub size_bytes: i64,
    pub size_label: String,
    pub sha256: Option<String>,
    #[serde(default)]
    pub hf_id: String,
    #[serde(default)]
    pub downloads: u64,
}
