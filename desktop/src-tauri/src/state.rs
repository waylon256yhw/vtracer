use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use studio_core::LoadedImage;

/// Shared app state. The decoded image lives behind an `Arc` so conversion
/// work can hold a cheap clone of the handle without keeping the mutex locked.
#[derive(Default)]
pub struct AppState {
    pub image: Mutex<Option<Arc<LoadedImage>>>,
}

impl AppState {
    /// Take a handle to the current image without holding the lock.
    pub fn image_handle(&self) -> Option<Arc<LoadedImage>> {
        self.image.lock().expect("image mutex poisoned").clone()
    }
}

/// Root cache dir for generated assets (previews, thumbnails). Cleared per
/// image load; fully cleaned up on exit in M5.
pub fn cache_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    use tauri::Manager;
    let dir = app
        .path()
        .app_cache_dir()
        .map_err(|e| format!("无法定位缓存目录：{e}"))?
        .join("assets");
    std::fs::create_dir_all(&dir).map_err(|e| format!("无法创建缓存目录：{e}"))?;
    Ok(dir)
}
