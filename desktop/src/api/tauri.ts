import { invoke, convertFileSrc, Channel } from '@tauri-apps/api/core'
import { open, save } from '@tauri-apps/plugin-dialog'
import type {
  ExperimentSession,
  FullResult,
  ImageInfo,
  MatrixEvent,
  MatrixRequest,
  Recipe,
  RunStarted,
  StudioApi,
  StudioConfig,
} from './types'

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

  async renderFull(config: StudioConfig) {
    return await invoke<FullResult>('render_full', { config })
  },

  async exportResult(resultId: number, outPath: string) {
    await invoke('export_result', { resultId, outPath })
  },

  async saveRecipe(recipe: Recipe, path: string) {
    await invoke('save_recipe', { recipe, path })
  },

  async loadRecipe(path: string) {
    return await invoke<Recipe>('load_recipe', { path })
  },

  async saveSession(session: ExperimentSession, path: string) {
    await invoke('save_session', { session, path })
  },

  async loadSession(path: string) {
    return await invoke<ExperimentSession>('load_session', { path })
  },

  async pickSavePath(defaultName: string, extName: string, extensions: string[]) {
    const picked = await save({
      defaultPath: defaultName,
      filters: [{ name: extName, extensions }],
    })
    return picked ?? null
  },

  async pickOpenPath(extName: string, extensions: string[]) {
    const picked = await open({
      multiple: false,
      directory: false,
      filters: [{ name: extName, extensions }],
    })
    return picked ?? null
  },
}
