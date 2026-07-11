import type { StudioApi } from './types'
import { tauriApi } from './tauri'
import { mockApi } from './mock'

/**
 * Real Tauri backend inside the app; mock in a plain browser (`npm run dev`
 * on the headless dev box) so the whole UI flow stays exercisable without a
 * Windows build.
 */
const isTauri = '__TAURI_INTERNALS__' in window

export const api: StudioApi = isTauri ? tauriApi : mockApi
export * from './types'
