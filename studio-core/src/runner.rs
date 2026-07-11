use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use vtracer::ColorImage;

use crate::{CandidateParams, Metrics, StudioConfig};

/// Set to true to stop scheduling new candidates. In-flight candidates finish
/// (vtracer::convert has no internal cancellation), so after cancellation at
/// most `worker_count()` more CandidateDone events may still arrive.
pub type CancelToken = Arc<AtomicBool>;

#[derive(Debug, Clone)]
pub struct CandidateResult {
    pub params: CandidateParams,
    pub config: StudioConfig,
    pub svg_text: String,
    pub thumb_png: Vec<u8>,
    pub metrics: Metrics,
}

/// Receives runner results. Implemented by the Tauri shell (which forwards to
/// a per-run Channel) and by test collectors — keeps every GUI type out of
/// this crate.
pub trait CandidateSink: Send + Sync + 'static {
    fn candidate_done(&self, result: CandidateResult);
    fn candidate_error(&self, id: &str, message: &str);
    /// Emitted exactly once per run, after every scheduled candidate has
    /// either completed or been skipped due to cancellation.
    fn finished(&self, completed: usize, cancelled: bool);
}

/// One matrix run over a pre-cropped ROI. The caller (shell) resolves cache
/// hits BEFORE building the job — `candidates` here are only the misses that
/// actually need computing.
pub struct MatrixJob {
    pub roi_image: ColorImage,
    pub base: StudioConfig,
    /// Scheduling order (sparse set first — see `MatrixAxes::ordered`).
    pub candidates: Vec<CandidateParams>,
    pub thumb_max_edge: u32,
}

/// Process-wide bounded worker pool (bounded CONCURRENCY — the queue itself
/// is unbounded but never holds more than one matrix's candidates). A single
/// pool serves every run, so an auto-cancelled old run and its replacement
/// can never oversubscribe the CPU together.
pub fn worker_count() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(2)
        .min(4)
}

type Task = Box<dyn FnOnce() + Send>;

fn pool() -> &'static crossbeam_channel::Sender<Task> {
    static POOL: OnceLock<crossbeam_channel::Sender<Task>> = OnceLock::new();
    POOL.get_or_init(|| {
        let (tx, rx) = crossbeam_channel::unbounded::<Task>();
        for i in 0..worker_count() {
            let rx = rx.clone();
            std::thread::Builder::new()
                .name(format!("studio-worker-{i}"))
                .spawn(move || {
                    while let Ok(task) = rx.recv() {
                        task();
                    }
                })
                .expect("failed to spawn worker thread");
        }
        tx
    })
}

/// Enqueue a matrix run. Returns immediately; results stream through `sink`
/// from the worker pool. An empty `candidates` list emits `finished(0, …)`
/// right away (the all-cache-hits case).
pub fn run_matrix(job: MatrixJob, sink: Arc<dyn CandidateSink>, cancel: CancelToken) {
    let total = job.candidates.len();
    if total == 0 {
        sink.finished(0, cancel.load(Ordering::SeqCst));
        return;
    }

    struct RunState {
        drained: AtomicUsize,
        completed: AtomicUsize,
        total: usize,
    }
    let state = Arc::new(RunState {
        drained: AtomicUsize::new(0),
        completed: AtomicUsize::new(0),
        total,
    });
    let roi = Arc::new(job.roi_image);
    let base = Arc::new(job.base);

    for params in job.candidates {
        let (sink, cancel, state, roi, base) =
            (sink.clone(), cancel.clone(), state.clone(), roi.clone(), base.clone());
        let thumb_max_edge = job.thumb_max_edge;
        pool()
            .send(Box::new(move || {
                if !cancel.load(Ordering::SeqCst) {
                    // A converter panic must not kill the worker or swallow the
                    // Finished event — downgrade it to a CandidateError.
                    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        compute(&roi, &base, &params, thumb_max_edge)
                    }));
                    match outcome {
                        Ok(Ok(result)) => {
                            state.completed.fetch_add(1, Ordering::SeqCst);
                            sink.candidate_done(result);
                        }
                        Ok(Err(message)) => {
                            state.completed.fetch_add(1, Ordering::SeqCst);
                            sink.candidate_error(&params.id, &message);
                        }
                        Err(_) => {
                            state.completed.fetch_add(1, Ordering::SeqCst);
                            sink.candidate_error(&params.id, "转换器内部错误（panic），已跳过该参数组合");
                        }
                    }
                }
                if state.drained.fetch_add(1, Ordering::SeqCst) + 1 == state.total {
                    sink.finished(
                        state.completed.load(Ordering::SeqCst),
                        cancel.load(Ordering::SeqCst),
                    );
                }
            }))
            .expect("worker pool channel closed");
    }
}

/// Convert the FULL image synchronously (blocking; no progress callback —
/// callers show an indeterminate progress UI). Clones the decoded RGBA once.
/// Returns the SVG text plus its metrics.
pub fn render_full(
    image: &crate::LoadedImage,
    config: &StudioConfig,
) -> Result<(String, Metrics), String> {
    config.validate().map_err(|e| e.to_string())?;
    let input = image.to_color_image();
    let start = Instant::now();
    let svg = vtracer::convert(input, config.into())?;
    let elapsed_ms = start.elapsed().as_millis() as u64;
    let svg_text = svg.to_string();
    let metrics = Metrics::from_svg(&svg, &svg_text, elapsed_ms);
    Ok((svg_text, metrics))
}

fn compute(
    roi: &ColorImage,
    base: &StudioConfig,
    params: &CandidateParams,
    thumb_max_edge: u32,
) -> Result<CandidateResult, String> {
    let config = params.apply_to(base);
    config.validate().map_err(|e| e.to_string())?;

    let input = ColorImage {
        pixels: roi.pixels.clone(),
        width: roi.width,
        height: roi.height,
    };
    let start = Instant::now();
    let svg = vtracer::convert(input, (&config).into())?;
    let elapsed_ms = start.elapsed().as_millis() as u64;

    let svg_text = svg.to_string();
    let metrics = Metrics::from_svg(&svg, &svg_text, elapsed_ms);
    let thumb_png =
        crate::render_thumbnail(&svg_text, thumb_max_edge).map_err(|e| e.to_string())?;

    Ok(CandidateResult {
        params: params.clone(),
        config,
        svg_text,
        thumb_png,
        metrics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{LoadedImage, MatrixAxes, Rect, RunMode};
    use std::sync::Mutex;
    use std::time::Duration;

    struct Collector {
        done: Mutex<Vec<CandidateResult>>,
        errors: Mutex<Vec<String>>,
        finished: Mutex<Vec<(usize, bool)>>,
        notify: crossbeam_channel::Sender<()>,
    }

    impl CandidateSink for Collector {
        fn candidate_done(&self, result: CandidateResult) {
            self.done.lock().unwrap().push(result);
        }
        fn candidate_error(&self, id: &str, message: &str) {
            self.errors.lock().unwrap().push(format!("{id}: {message}"));
        }
        fn finished(&self, completed: usize, cancelled: bool) {
            self.finished.lock().unwrap().push((completed, cancelled));
            let _ = self.notify.send(());
        }
    }

    fn collector() -> (Arc<Collector>, crossbeam_channel::Receiver<()>) {
        let (tx, rx) = crossbeam_channel::bounded(1);
        (
            Arc::new(Collector {
                done: Mutex::new(vec![]),
                errors: Mutex::new(vec![]),
                finished: Mutex::new(vec![]),
                notify: tx,
            }),
            rx,
        )
    }

    fn job(candidates: Vec<CandidateParams>) -> MatrixJob {
        let img = LoadedImage::from_rgba(crate::image_io::test_image(48, 48));
        MatrixJob {
            roi_image: img.crop_roi(Rect { x: 0, y: 0, w: 48, h: 48 }).unwrap(),
            base: StudioConfig::default(),
            candidates,
            thumb_max_edge: 32,
        }
    }

    fn small_axes() -> MatrixAxes {
        MatrixAxes {
            gradient_step: vec![16, 32],
            filter_speckle: vec![2],
            color_precision: vec![4, 6],
        }
        .normalized()
        .unwrap()
    }

    #[test]
    fn render_full_returns_svg_and_metrics() {
        let img = LoadedImage::from_rgba(crate::image_io::test_image(64, 48));
        let (svg_text, metrics) = render_full(&img, &StudioConfig::default()).unwrap();
        assert!(svg_text.contains("<path"));
        assert!(metrics.paths > 0);
        assert_eq!(metrics.svg_bytes, svg_text.len());
    }

    #[test]
    fn runs_all_candidates_and_finishes_once() {
        let (sink, rx) = collector();
        let candidates = small_axes().ordered(RunMode::Full);
        let total = candidates.len();
        run_matrix(job(candidates), sink.clone(), CancelToken::default());
        rx.recv_timeout(Duration::from_secs(60)).unwrap();

        assert_eq!(sink.done.lock().unwrap().len(), total);
        assert!(sink.errors.lock().unwrap().is_empty());
        assert_eq!(*sink.finished.lock().unwrap(), vec![(total, false)]);
        for r in sink.done.lock().unwrap().iter() {
            assert!(r.metrics.paths > 0);
            assert!(!r.thumb_png.is_empty());
            assert!(r.svg_text.contains("<path"));
        }
    }

    #[test]
    fn pre_cancelled_run_skips_everything() {
        let (sink, rx) = collector();
        let cancel = CancelToken::default();
        cancel.store(true, Ordering::SeqCst);
        run_matrix(job(small_axes().ordered(RunMode::Full)), sink.clone(), cancel);
        rx.recv_timeout(Duration::from_secs(60)).unwrap();

        assert!(sink.done.lock().unwrap().is_empty(), "取消后不得启动新候选");
        assert_eq!(*sink.finished.lock().unwrap(), vec![(0, true)]);
    }

    #[test]
    fn empty_job_finishes_immediately() {
        let (sink, rx) = collector();
        run_matrix(job(vec![]), sink.clone(), CancelToken::default());
        rx.recv_timeout(Duration::from_secs(5)).unwrap();
        assert_eq!(*sink.finished.lock().unwrap(), vec![(0, false)]);
    }

    #[test]
    fn invalid_candidate_reports_error_not_panic() {
        let (sink, rx) = collector();
        // Bypass axes validation to simulate a bad config reaching the runner.
        let bad = CandidateParams {
            id: "g16_f200_p6".into(),
            gradient_step: 16,
            filter_speckle: 200,
            color_precision: 6,
        };
        run_matrix(job(vec![bad]), sink.clone(), CancelToken::default());
        rx.recv_timeout(Duration::from_secs(60)).unwrap();
        assert_eq!(sink.errors.lock().unwrap().len(), 1);
        assert_eq!(*sink.finished.lock().unwrap(), vec![(1, false)]);
    }
}
