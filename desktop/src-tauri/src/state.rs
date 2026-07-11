use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use studio_core::{CancelToken, LoadedImage, Metrics, StudioConfig};

/// One computed candidate — the single source of truth the inspector and
/// export read from. Keyed by content-addressed `CacheKey`, so a later run
/// over the same image/ROI/config reuses it instantly.
pub struct CandidateRecord {
    pub config: StudioConfig,
    pub svg_text: String,
    pub thumb_path: PathBuf,
    pub metrics: Metrics,
}

pub struct RunHandle {
    pub id: u64,
    pub cancel: CancelToken,
}

pub type Records = Arc<Mutex<HashMap<String, Arc<CandidateRecord>>>>;
/// (run_id, candidate_id) → cache key, so IPC lookups don't need to re-derive
/// the config.
pub type RunIndex = Arc<Mutex<HashMap<(u64, String), String>>>;

#[derive(Default)]
pub struct AppState {
    pub image: Mutex<Option<Arc<LoadedImage>>>,
    pub next_run_id: AtomicU64,
    pub current_run: Mutex<Option<RunHandle>>,
    pub records: Records,
    pub run_index: RunIndex,
    pub next_result_id: AtomicU64,
    /// Full-resolution render outputs; export copies these exact bytes so
    /// what the user inspected is byte-identical to what lands on disk.
    pub full_results: Mutex<HashMap<u64, Arc<FullRecord>>>,
}

pub struct FullRecord {
    /// Exact rendered bytes live at this path; export copies them verbatim.
    pub svg_path: PathBuf,
}

impl AppState {
    /// Take a handle to the current image without holding the lock.
    pub fn image_handle(&self) -> Option<Arc<LoadedImage>> {
        self.image.lock().expect("image mutex poisoned").clone()
    }

    /// Cancel the current run (if any) and register the new one. The single
    /// process-wide worker pool bounds old in-flight candidates and the new
    /// run together, so replacement can't oversubscribe the CPU.
    pub fn replace_run(&self, handle: RunHandle) {
        let mut current = self.current_run.lock().expect("run mutex poisoned");
        if let Some(old) = current.take() {
            old.cancel.store(true, Ordering::SeqCst);
        }
        *current = Some(handle);
    }

    /// New image → every cached result and run mapping is stale.
    pub fn reset_for_new_image(&self, image: Arc<LoadedImage>) {
        let mut current = self.current_run.lock().expect("run mutex poisoned");
        if let Some(old) = current.take() {
            old.cancel.store(true, Ordering::SeqCst);
        }
        drop(current);
        self.records.lock().expect("records mutex poisoned").clear();
        self.run_index.lock().expect("run_index mutex poisoned").clear();
        self.full_results
            .lock()
            .expect("full_results mutex poisoned")
            .clear();
        *self.image.lock().expect("image mutex poisoned") = Some(image);
    }
}

/// Root cache dir for generated assets (previews, ROI references,
/// thumbnails). Cleared per image load; fully cleaned up on exit in M5.
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
