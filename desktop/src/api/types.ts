/** Shared IPC types — mirror of the Rust structs in desktop/src-tauri. */

export interface ImageInfo {
  width: number
  height: number
  file_size: number
  hash: string
  preview_path: string
}

/** Rectangle in original-image pixel coordinates. */
export interface Rect {
  x: number
  y: number
  w: number
  h: number
}

/** Mirror of studio_core::StudioConfig (UI-facing parameter names). */
export interface StudioConfig {
  color_mode: 'color' | 'binary'
  hierarchical: 'stacked' | 'cutout'
  mode: 'spline' | 'polygon' | 'pixel'
  filter_speckle: number
  color_precision: number
  gradient_step: number
  corner_threshold: number
  segment_length: number
  max_iterations: number
  splice_threshold: number
  path_precision: number | null
}

export function defaultConfig(): StudioConfig {
  return {
    color_mode: 'color',
    hierarchical: 'stacked',
    mode: 'spline',
    filter_speckle: 4,
    color_precision: 6,
    gradient_step: 16,
    corner_threshold: 60,
    segment_length: 4.0,
    max_iterations: 10,
    splice_threshold: 45,
    path_precision: 2,
  }
}

export interface MatrixAxes {
  gradient_step: number[]
  filter_speckle: number[]
  color_precision: number[]
}

export type RunMode = 'sparse' | 'full'

export interface MatrixRequest {
  roi: Rect
  base: StudioConfig
  axes: MatrixAxes
  mode: RunMode
}

export interface CandidateParams {
  id: string
  gradient_step: number
  filter_speckle: number
  color_precision: number
}

export interface Metrics {
  /** `<path>` element count (stacked layers), not Bézier segments. */
  paths: number
  colors: number
  svg_bytes: number
  elapsed_ms: number
}

export type MatrixEvent =
  | {
      type: 'CandidateDone'
      run_id: number
      id: string
      params: CandidateParams
      metrics: Metrics
      thumb_path: string
      cached: boolean
    }
  | { type: 'CandidateError'; run_id: number; id: string; message: string }
  | { type: 'Finished'; run_id: number; completed: number; cancelled: boolean }

export interface RunStarted {
  run_id: number
  total: number
  /** Candidate ids in scheduling order (sparse set first). */
  order: string[]
  /** Full-resolution ROI crop — the faithful original for the inspector. */
  roi_reference_path: string
}

export interface StudioApi {
  /** Open the native file picker; null if the user cancelled. */
  pickImageFile(): Promise<string | null>
  /** Decode the image in the backend and get its metadata + preview path. */
  loadImage(path: string): Promise<ImageInfo>
  /** Turn a backend file path into a URL the webview can render. */
  assetUrl(path: string): string
  /** Start a matrix run; events stream to `onEvent` until Finished. */
  runMatrix(req: MatrixRequest, onEvent: (e: MatrixEvent) => void): Promise<RunStarted>
  cancelRun(runId: number): Promise<void>
  /** ROI SVG text of a completed candidate (inspector live view). */
  getCandidateSvg(runId: number, id: string): Promise<string>
}

/** Deterministic candidate id — must match studio_core::CandidateParams. */
export function candidateId(g: number, f: number, p: number): string {
  return `g${g}_f${f}_p${p}`
}

/** Sorted + deduplicated copy of axis values (mirror of MatrixAxes::normalized). */
export function normalizedAxes(axes: MatrixAxes): MatrixAxes {
  const norm = (v: number[]) => [...new Set(v)].sort((a, b) => a - b)
  return {
    gradient_step: norm(axes.gradient_step),
    filter_speckle: norm(axes.filter_speckle),
    color_precision: norm(axes.color_precision),
  }
}
