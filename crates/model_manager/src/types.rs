use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

pub type ProgressCallback = Box<dyn Fn(i64, i64) + Send>;
pub type TokenCallback = Box<dyn Fn(&str) + Send>;
pub type ErrorCallback = Box<dyn Fn(&str) + Send>;

pub struct DownloadHandle {
    pub cancel_flag: Arc<AtomicBool>,
}

pub struct ModelManagerInner {
    pub downloads: Vec<(i32, DownloadHandle)>,
    pub next_download_id: i32,
    pub runtime: tokio::runtime::Runtime,
}

pub struct ModelManager {
    pub models_dir: PathBuf,
    pub config_path: PathBuf,
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
