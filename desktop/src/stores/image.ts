import { defineStore } from 'pinia'
import { api, type ImageInfo, type Rect } from '../api'

export const useImageStore = defineStore('image', {
  state: () => ({
    path: null as string | null,
    info: null as ImageInfo | null,
    previewUrl: null as string | null,
    roi: null as Rect | null,
    loading: false,
    error: null as string | null,
  }),

  getters: {
    hasImage: (s) => s.info !== null,
  },

  actions: {
    async openAndLoad() {
      const path = await api.pickImageFile()
      if (!path) return
      this.loading = true
      this.error = null
      try {
        const info = await api.loadImage(path)
        this.path = path
        this.info = info
        this.previewUrl = api.assetUrl(info.preview_path)
        this.roi = null
      } catch (e) {
        this.error = String(e)
      } finally {
        this.loading = false
      }
    },

    setRoi(roi: Rect | null) {
      this.roi = roi
    },
  },
})
