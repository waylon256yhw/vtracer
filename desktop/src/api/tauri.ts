import { invoke, convertFileSrc } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import type { ImageInfo, StudioApi } from './types'

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
}
