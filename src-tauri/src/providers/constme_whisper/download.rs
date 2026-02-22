//! Auto-download manager for Const-me/Whisper DLL and GGML models.

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use tauri::{AppHandle, Emitter};

/// GitHub release URL for the Library.zip containing Whisper.dll.
const LIBRARY_ZIP_URL: &str =
    "https://github.com/Const-me/Whisper/releases/download/1.12.0/Library.zip";

/// Available model sizes with HuggingFace download URLs.
pub struct ModelInfo {
    pub name: &'static str,
    pub filename: &'static str,
    pub url: &'static str,
    pub size_description: &'static str,
}

pub const AVAILABLE_MODELS: &[ModelInfo] = &[
    ModelInfo {
        name: "Small",
        filename: "ggml-small.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-small.bin",
        size_description: "~466 MB - faster, lower accuracy",
    },
    ModelInfo {
        name: "Medium",
        filename: "ggml-medium.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-medium.bin",
        size_description: "~1.5 GB - recommended balance",
    },
    ModelInfo {
        name: "Large v3",
        filename: "ggml-large-v3.bin",
        url: "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-large-v3.bin",
        size_description: "~3 GB - highest accuracy",
    },
];

/// Progress event emitted during downloads.
#[derive(Clone, serde::Serialize)]
pub struct DownloadProgress {
    pub item: String,
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
    pub done: bool,
    pub error: Option<String>,
}

/// Get the directory where Const-me/Whisper files are stored.
pub fn data_dir() -> Result<PathBuf> {
    let base = std::env::var("LOCALAPPDATA")
        .or_else(|_| std::env::var("APPDATA"))
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    let dir = base.join("Whisper").join("constme");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

/// Get the expected path to Whisper.dll.
pub fn dll_path() -> Result<PathBuf> {
    Ok(data_dir()?.join("Whisper.dll"))
}

/// Get the expected path to a model file.
pub fn model_path(filename: &str) -> Result<PathBuf> {
    Ok(data_dir()?.join(filename))
}

/// Check if the DLL is already downloaded.
pub fn is_dll_available() -> bool {
    dll_path().map(|p| p.exists()).unwrap_or(false)
}

/// Check if a model file is already downloaded.
pub fn is_model_available(filename: &str) -> bool {
    model_path(filename).map(|p| p.exists()).unwrap_or(false)
}

/// Download the Whisper.dll from GitHub releases.
pub async fn download_dll(app: &AppHandle) -> Result<PathBuf> {
    let dest_dir = data_dir()?;
    let dll_dest = dest_dir.join("Whisper.dll");

    if dll_dest.exists() {
        return Ok(dll_dest);
    }

    // Download Library.zip
    let zip_path = dest_dir.join("Library.zip");
    download_file(app, LIBRARY_ZIP_URL, &zip_path, "Whisper.dll").await?;

    // Extract Whisper.dll from the zip
    extract_dll_from_zip(&zip_path, &dest_dir)?;

    // Clean up the zip
    let _ = std::fs::remove_file(&zip_path);

    if !dll_dest.exists() {
        return Err(anyhow!(
            "Whisper.dll not found in Library.zip after extraction"
        ));
    }

    Ok(dll_dest)
}

/// Download a model file from HuggingFace.
pub async fn download_model(app: &AppHandle, model_filename: &str) -> Result<PathBuf> {
    let model_info = AVAILABLE_MODELS
        .iter()
        .find(|m| m.filename == model_filename)
        .ok_or_else(|| anyhow!("Unknown model: {}", model_filename))?;

    let dest = model_path(model_filename)?;

    if dest.exists() {
        return Ok(dest);
    }

    download_file(app, model_info.url, &dest, model_info.name).await?;

    Ok(dest)
}

/// Download a file from a URL with progress reporting.
async fn download_file(app: &AppHandle, url: &str, dest: &Path, item_name: &str) -> Result<()> {
    log::info!("Downloading {} from {}", item_name, url);

    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| anyhow!("Download request failed for {}: {}", item_name, e))?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Download failed for {}: HTTP {}",
            item_name,
            response.status()
        ));
    }

    let total_bytes = response.content_length();
    let mut downloaded: u64 = 0;

    // Write to a temp file first, then rename (atomic-ish)
    let temp_path = dest.with_extension("download");
    let mut file = std::fs::File::create(&temp_path)
        .map_err(|e| anyhow!("Failed to create temp file: {}", e))?;

    let mut stream = response.bytes_stream();

    while let Some(chunk_result) = stream.next().await {
        let chunk =
            chunk_result.map_err(|e| anyhow!("Download stream error: {}", e))?;
        file.write_all(&chunk)
            .map_err(|e| anyhow!("Failed to write downloaded data: {}", e))?;
        downloaded += chunk.len() as u64;

        let _ = app.emit(
            "download-progress",
            DownloadProgress {
                item: item_name.to_string(),
                downloaded_bytes: downloaded,
                total_bytes,
                done: false,
                error: None,
            },
        );
    }

    file.flush()?;
    drop(file);

    // Rename to final path
    std::fs::rename(&temp_path, dest)
        .map_err(|e| anyhow!("Failed to move downloaded file: {}", e))?;

    let _ = app.emit(
        "download-progress",
        DownloadProgress {
            item: item_name.to_string(),
            downloaded_bytes: downloaded,
            total_bytes,
            done: true,
            error: None,
        },
    );

    log::info!("Downloaded {} ({} bytes)", item_name, downloaded);
    Ok(())
}

/// Extract Whisper.dll from Library.zip.
fn extract_dll_from_zip(zip_path: &Path, dest_dir: &Path) -> Result<()> {
    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let name = entry.name().to_string();

        // Look for DLL files in the archive
        if name.ends_with(".dll") || name.ends_with(".DLL") {
            let filename = Path::new(&name)
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let dest_path = dest_dir.join(&filename);
            let mut outfile = std::fs::File::create(&dest_path)?;
            std::io::copy(&mut entry, &mut outfile)?;
            log::info!("Extracted {} from archive", filename);
        }
    }

    Ok(())
}
