import { defineStore } from 'pinia'

/**
 * The single shared pan/zoom state. Both inspector layers (original ROI
 * reference and live SVG) bind their CSS transform to this, and it survives
 * candidate switches — so the view stays glued to the same spot while the
 * user flips through parameters.
 */
export const useViewportStore = defineStore('viewport', {
  state: () => ({
    scale: 1,
    x: 0,
    y: 0,
  }),

  getters: {
    cssTransform: (s) => `translate(${s.x}px, ${s.y}px) scale(${s.scale})`,
    /** True until anyone fits/zooms/restores — the inspector auto-fits then. */
    isDefault: (s) => s.scale === 1 && s.x === 0 && s.y === 0,
  },

  actions: {
    /** Zoom about a point given in container coordinates. */
    zoomAt(cx: number, cy: number, factor: number) {
      const next = Math.min(Math.max(this.scale * factor, 0.1), 64)
      const f = next / this.scale
      // Keep the content point under (cx, cy) stationary.
      this.x = cx - (cx - this.x) * f
      this.y = cy - (cy - this.y) * f
      this.scale = next
    },

    panBy(dx: number, dy: number) {
      this.x += dx
      this.y += dy
    },

    /** Center the content in the pane at the largest scale that fits (≤1). */
    fitTo(paneW: number, paneH: number, contentW: number, contentH: number) {
      if (!paneW || !paneH || !contentW || !contentH) return
      const scale = Math.min(paneW / contentW, paneH / contentH, 1)
      this.scale = scale
      this.x = (paneW - contentW * scale) / 2
      this.y = (paneH - contentH * scale) / 2
    },

    /** 100% pixel scale, centered. */
    oneToOne(paneW: number, paneH: number, contentW: number, contentH: number) {
      this.scale = 1
      this.x = (paneW - contentW) / 2
      this.y = (paneH - contentH) / 2
    },

    reset() {
      this.scale = 1
      this.x = 0
      this.y = 0
    },
  },
})
