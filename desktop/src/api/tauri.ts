import { invoke, convertFileSrc, Channel } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import type { ImageInfo, MatrixEvent, MatrixRequest, RunStarted, StudioApi } from './types'

export const tauriApi: StudioApi = {
  async pickImageFile() {
    const picked = await open({
      multiple: false,
      directory: false,
      filters: [
        { name: '图片', extensions: ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp', 'tiff', 'tif'] },
      ],
    })
    return picked ?? null
  },

  async loadImage(path: string) {
    return await invoke<ImageInfo>('load_image', { path })
  },

  assetUrl(path: string) {
    return convertFileSrc(path)
  },

  async runMatrix(req: MatrixRequest, onEvent: (e: MatrixEvent) => void) {
    const channel = new Channel<MatrixEvent>()
    channel.onmessage = onEvent
    return await invoke<RunStarted>('run_matrix', { req, onEvent: channel })
  },

  async cancelRun(runId: number) {
    await invoke('cancel_run', { runId })
  },

  async getCandidateSvg(runId: number, id: string) {
    return await invoke<string>('get_candidate_svg', { runId, id })
  },
}
