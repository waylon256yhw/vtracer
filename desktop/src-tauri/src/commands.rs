use std::sync::atomic::Ordering;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use studio_core::{
    CacheKey, CandidateParams, CandidateResult, CandidateSink, CancelToken, LoadedImage,
    MatrixAxes, MatrixJob, Metrics, Rect, RunMode, StudioConfig, THUMB_MAX_EDGE,
};
use tauri::ipc::Channel;
use tauri::State;

use crate::state::{cache_dir, AppState, CandidateRecord, Records, RunHandle, RunIndex};

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
    let file_size = std::fs::metadata(&path)
        .map_err(|e| format!("无法读取文件：{e}"))
        .map(|m| m.len())?;

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
    state.reset_for_new_image(Arc::new(loaded));
    Ok(info)
}

#[derive(Debug, Deserialize)]
pub struct MatrixRequest {
    pub roi: Rect,
    pub base: StudioConfig,
    pub axes: MatrixAxes,
    pub mode: RunMode,
}

#[derive(Debug, Serialize)]
pub struct RunStarted {
    pub run_id: u64,
    pub total: usize,
    /// Candidate ids in scheduling order (sparse set first) — the frontend
    /// pre-renders grid placeholders from this.
    pub order: Vec<String>,
    /// Full-resolution ROI crop PNG — the faithful "original" side of the
    /// candidate inspector (the whole-image preview is downscaled and must
    /// not be used for quality judgment).
    pub roi_reference_path: String,
}

/// Typed per-run event stream. The channel itself scopes events to this run;
/// `run_id` is included as belt-and-braces for the frontend store.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum MatrixEvent {
    CandidateDone {
        run_id: u64,
        id: String,
        params: CandidateParams,
        metrics: Metrics,
        thumb_path: String,
        /// True when served from cache (a previous run already computed it).
        cached: bool,
    },
    CandidateError {
        run_id: u64,
        id: String,
        message: String,
    },
    Finished {
        run_id: u64,
        completed: usize,
        cancelled: bool,
    },
}

/// Start a matrix run over the ROI. Cache hits are emitted immediately;
/// only misses are scheduled on the worker pool. Any previous run is
/// cancelled (no new candidates start; in-flight ones finish).
#[tauri::command]
pub async fn run_matrix(
    req: MatrixRequest,
    on_event: Channel<MatrixEvent>,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<RunStarted, String> {
    let image = state.image_handle().ok_or("请先打开图片")?;
    req.base.validate().map_err(|e| e.to_string())?;
    let axes = req.axes.normalized().map_err(|e| e.to_string())?;
    image.check_roi(req.roi).map_err(|e| e.to_string())?;

    let run_id = state.next_run_id.fetch_add(1, Ordering::SeqCst) + 1;
    let cancel = CancelToken::default();
    state.replace_run(RunHandle { id: run_id, cancel: cancel.clone() });

    let dir = cache_dir(&app)?;

    // Full-resolution ROI reference for the inspector, reused across runs on
    // the same ROI.
    let roi_reference_path = dir.join(format!(
        "roi-ref-{}-{}-{}-{}-{}.png",
        &image.hash[..12],
        req.roi.x,
        req.roi.y,
        req.roi.w,
        req.roi.h
    ));
    if !roi_reference_path.exists() {
        let png = image.roi_reference_png(req.roi).map_err(|e| e.to_string())?;
        std::fs::write(&roi_reference_path, png).map_err(|e| format!("无法写入 ROI 参照图：{e}"))?;
    }

    let ordered = axes.ordered(req.mode);
    let order: Vec<String> = ordered.iter().map(|c| c.id.clone()).collect();
    let total = ordered.len();

    // Split into cache hits (emit right away) and misses (compute).
    let mut misses = Vec::new();
    let mut hits = Vec::new();
    {
        let records = state.records.lock().expect("records mutex poisoned");
        let mut run_index = state.run_index.lock().expect("run_index mutex poisoned");
        for candidate in ordered {
            let key = CacheKey::new(&image.hash, req.roi, &candidate.apply_to(&req.base));
            run_index.insert((run_id, candidate.id.clone()), key.as_str().to_owned());
            match records.get(key.as_str()) {
                Some(record) => hits.push((candidate, record.clone())),
                None => misses.push(candidate),
            }
        }
    }
    let hit_count = hits.len();
    for (candidate, record) in hits {
        let _ = on_event.send(MatrixEvent::CandidateDone {
            run_id,
            id: candidate.id,
            params: candidate_params_of(&record),
            metrics: record.metrics.clone(),
            thumb_path: record.thumb_path.to_string_lossy().into_owned(),
            cached: true,
        });
    }

    let sink = Arc::new(ChannelSink {
        run_id,
        channel: on_event,
        records: state.records.clone(),
        run_index: state.run_index.clone(),
        image_hash: image.hash.clone(),
        roi: req.roi,
        thumb_dir: dir,
        hit_count,
    });

    let roi_image = image.crop_roi(req.roi).map_err(|e| e.to_string())?;
    studio_core::run_matrix(
        MatrixJob {
            roi_image,
            base: req.base,
            candidates: misses,
            thumb_max_edge: THUMB_MAX_EDGE,
        },
        sink,
        cancel,
    );

    Ok(RunStarted {
        run_id,
        total,
        order,
        roi_reference_path: roi_reference_path.to_string_lossy().into_owned(),
    })
}

/// Cancel a run: no new candidate starts; at most `worker_count()` in-flight
/// candidates still finish; exactly one Finished{cancelled:true} follows.
#[tauri::command]
pub fn cancel_run(run_id: u64, state: State<'_, AppState>) -> Result<(), String> {
    let current = state.current_run.lock().expect("run mutex poisoned");
    if let Some(run) = current.as_ref() {
        if run.id == run_id {
            run.cancel.store(true, Ordering::SeqCst);
        }
    }
    Ok(())
}

/// ROI SVG text of one candidate, for the inspector's live-SVG side.
#[tauri::command]
pub fn get_candidate_svg(
    run_id: u64,
    id: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let key = {
        let run_index = state.run_index.lock().expect("run_index mutex poisoned");
        run_index.get(&(run_id, id.clone())).cloned()
    }
    .ok_or_else(|| format!("找不到候选 {id}（run {run_id}）"))?;
    let records = state.records.lock().expect("records mutex poisoned");
    let record = records.get(&key).ok_or_else(|| format!("候选 {id} 尚未完成"))?;
    Ok(record.svg_text.clone())
}

/// Forwards runner callbacks into the per-run channel and persists results
/// into the shared record cache.
struct ChannelSink {
    run_id: u64,
    channel: Channel<MatrixEvent>,
    records: Records,
    run_index: RunIndex,
    image_hash: String,
    roi: Rect,
    thumb_dir: std::path::PathBuf,
    hit_count: usize,
}

impl CandidateSink for ChannelSink {
    fn candidate_done(&self, result: CandidateResult) {
        let key = CacheKey::new(&self.image_hash, self.roi, &result.config);
        let thumb_path = self.thumb_dir.join(format!("thumb-{}.png", &key.as_str()[..24]));
        if let Err(e) = std::fs::write(&thumb_path, &result.thumb_png) {
            let _ = self.channel.send(MatrixEvent::CandidateError {
                run_id: self.run_id,
                id: result.params.id.clone(),
                message: format!("无法写入缩略图：{e}"),
            });
            return;
        }
        let record = Arc::new(CandidateRecord {
            config: result.config,
            svg_text: result.svg_text,
            thumb_path: thumb_path.clone(),
            metrics: result.metrics.clone(),
        });
        self.records
            .lock()
            .expect("records mutex poisoned")
            .insert(key.as_str().to_owned(), record);
        self.run_index
            .lock()
            .expect("run_index mutex poisoned")
            .insert((self.run_id, result.params.id.clone()), key.as_str().to_owned());
        let _ = self.channel.send(MatrixEvent::CandidateDone {
            run_id: self.run_id,
            id: result.params.id.clone(),
            params: result.params,
            metrics: result.metrics,
            thumb_path: thumb_path.to_string_lossy().into_owned(),
            cached: false,
        });
    }

    fn candidate_error(&self, id: &str, message: &str) {
        let _ = self.channel.send(MatrixEvent::CandidateError {
            run_id: self.run_id,
            id: id.to_owned(),
            message: message.to_owned(),
        });
    }

    fn finished(&self, completed: usize, cancelled: bool) {
        let _ = self.channel.send(MatrixEvent::Finished {
            run_id: self.run_id,
            completed: completed + self.hit_count,
            cancelled,
        });
    }
}

fn candidate_params_of(record: &CandidateRecord) -> CandidateParams {
    CandidateParams {
        id: format!(
            "g{}_f{}_p{}",
            record.config.gradient_step, record.config.filter_speckle, record.config.color_precision
        ),
        gradient_step: record.config.gradient_step,
        filter_speckle: record.config.filter_speckle,
        color_precision: record.config.color_precision,
    }
}

fn clear_dir(dir: &std::path::Path) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}
