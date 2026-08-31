use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures_util::StreamExt;
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;
use tracing::info;

use crate::error::AiError;

/// Downloads to `<dest>.part` and only renames it to `dest` once the stream is
/// fully written and (optionally) verified. This keeps partial downloads
/// invisible to consumers that check `Path::exists()` and prevents a failed
/// or cancelled download from leaving a corrupt file in place of a good one.
fn part_path(dest: &Path) -> PathBuf {
    let mut name = dest
        .file_name()
        .map(|s| s.to_os_string())
        .unwrap_or_default();
    name.push(".part");
    dest.with_file_name(name)
}

pub async fn download_file(
    url: &str,
    dest: &Path,
    cancel: Arc<AtomicBool>,
    progress: Option<Box<dyn Fn(i64, i64) + Send>>,
    expected_sha256: Option<&str>,
) -> Result<String, AiError> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3600))
        .build()
        .map_err(|e| AiError::Other(format!("failed to build HTTP client: {e}")))?;

    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| AiError::Other(format!("failed to start download: {e}")))?;

    let total = resp.content_length().unwrap_or(0) as i64;
    let mut received: i64 = 0;

    let part = part_path(dest);
    let mut file = tokio::fs::File::create(&part).await?;

    let mut hasher = Sha256::new();
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        if cancel.load(Ordering::Relaxed) {
            let _ = tokio::fs::remove_file(&part).await;
            return Err(AiError::Cancelled);
        }

        let chunk = chunk.map_err(|e| AiError::Other(format!("download error: {e}")))?;
        hasher.update(&chunk);
        received += chunk.len() as i64;

        file.write_all(&chunk).await?;

        if let Some(ref cb) = progress {
            cb(received, total);
        }
    }

    file.flush().await?;

    let actual_hash = hex::encode(hasher.finalize());
    if let Some(expected) = expected_sha256 {
        if !actual_hash.eq_ignore_ascii_case(expected) {
            let _ = tokio::fs::remove_file(&part).await;
            return Err(AiError::Other(format!(
                "SHA-256 mismatch: expected {expected}, got {actual_hash}"
            )));
        }
    }

    tokio::fs::rename(&part, dest).await?;

    info!(
        "Downloaded {} -> {} (sha256: {})",
        url,
        dest.display(),
        actual_hash
    );
    Ok(actual_hash)
}
