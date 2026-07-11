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

    reset() {
      this.scale = 1
      this.x = 0
      this.y = 0
    },
  },
})
