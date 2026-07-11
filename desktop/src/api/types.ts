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

export interface StudioApi {
  /** Open the native file picker; null if the user cancelled. */
  pickImageFile(): Promise<string | null>
  /** Decode the image in the backend and get its metadata + preview path. */
  loadImage(path: string): Promise<ImageInfo>
  /** Turn a backend file path into a URL the webview can render. */
  assetUrl(path: string): string
}
