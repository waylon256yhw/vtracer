use std::sync::Arc;

use serde::Serialize;
use studio_core::LoadedImage;
use tauri::State;

use crate::state::{cache_dir, AppState};

#[derive(Debug, Serialize)]
pub struct ImageInfo {
    pub width: u32,
    pub height: u32,
    pub file_size: u64,
    pub hash: String,
    /// Path of the downscaled whole-image preview PNG (served to the webview
    /// via the asset protocol). Display-only; never a basis for parameter
    /// judgment.
    pub preview_path: String,
}

/// Decode an image once, keep pixels in Rust state, write a preview PNG into
/// the app cache dir. Pixels never cross IPC.
#[tauri::command]
pub async fn load_image(
    path: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<ImageInfo, String> {
    let file_size = std::fs::metadata(&path).map_err(|e| format!("无法读取文件：{e}")).map(|m| m.len())?;

    let loaded = tauri::async_runtime::spawn_blocking(move || {
        LoadedImage::open(std::path::Path::new(&path)).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| format!("后台任务失败：{e}"))??;

    let dir = cache_dir(&app)?;
    // New image invalidates every generated asset (previews + thumbnails).
    clear_dir(&dir);
    let preview = loaded.preview_png().map_err(|e| e.to_string())?;
    let preview_path = dir.join(format!("preview-{}.png", &loaded.hash[..12]));
    std::fs::write(&preview_path, preview).map_err(|e| format!("无法写入预览图：{e}"))?;

    let info = ImageInfo {
        width: loaded.width(),
        height: loaded.height(),
        file_size,
        hash: loaded.hash.clone(),
        preview_path: preview_path.to_string_lossy().into_owned(),
    };
    *state.image.lock().expect("image mutex poisoned") = Some(Arc::new(loaded));
    Ok(info)
}

fn clear_dir(dir: &std::path::Path) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}
