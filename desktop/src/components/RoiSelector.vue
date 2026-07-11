<script setup lang="ts">
import { computed, onMounted, onUnmounted, reactive, ref } from 'vue'
import { useImageStore } from '../stores/image'
import type { Rect } from '../api'

/**
 * Shows the (downscaled) whole-image preview and lets the user drag a
 * rectangle. Screen coordinates are converted to original-image pixel
 * coordinates using the true dimensions from load_image — the preview scale
 * never leaks into the ROI.
 */
const image = useImageStore()

const imgEl = ref<HTMLImageElement | null>(null)
/** Rendered size of the preview <img>, kept reactive via @load +
 * ResizeObserver — window resizes must move the committed-ROI overlay. */
const rendered = ref({ w: 0, h: 0 })
let resizeObserver: ResizeObserver | null = null

function updateRendered() {
  const el = imgEl.value
  rendered.value = { w: el?.clientWidth ?? 0, h: el?.clientHeight ?? 0 }
}

onMounted(() => {
  resizeObserver = new ResizeObserver(updateRendered)
  if (imgEl.value) resizeObserver.observe(imgEl.value)
  updateRendered()
})

onUnmounted(() => {
  resizeObserver?.disconnect()
})

const drag = reactive({
  active: false,
  startX: 0,
  startY: 0,
  curX: 0,
  curY: 0,
})

function clientToImg(e: MouseEvent): { x: number; y: number } | null {
  const el = imgEl.value
  if (!el) return null
  const box = el.getBoundingClientRect()
  const x = Math.min(Math.max(e.clientX - box.left, 0), box.width)
  const y = Math.min(Math.max(e.clientY - box.top, 0), box.height)
  return { x, y }
}

function onDown(e: MouseEvent) {
  const p = clientToImg(e)
  if (!p) return
  drag.active = true
  drag.startX = drag.curX = p.x
  drag.startY = drag.curY = p.y
  window.addEventListener('mousemove', onMove)
  window.addEventListener('mouseup', onUp)
}

function onMove(e: MouseEvent) {
  if (!drag.active) return
  const p = clientToImg(e)
  if (!p) return
  drag.curX = p.x
  drag.curY = p.y
}

function onUp() {
  window.removeEventListener('mousemove', onMove)
  window.removeEventListener('mouseup', onUp)
  if (!drag.active) return
  drag.active = false
  const el = imgEl.value
  const info = image.info
  if (!el || !info) return
  const box = el.getBoundingClientRect()
  const scaleX = info.width / box.width
  const scaleY = info.height / box.height
  const left = Math.min(drag.startX, drag.curX)
  const top = Math.min(drag.startY, drag.curY)
  const w = Math.abs(drag.curX - drag.startX)
  const h = Math.abs(drag.curY - drag.startY)
  if (w < 4 || h < 4) return // 误触忽略
  const roi: Rect = {
    x: Math.round(left * scaleX),
    y: Math.round(top * scaleY),
    w: Math.max(1, Math.round(w * scaleX)),
    h: Math.max(1, Math.round(h * scaleY)),
  }
  // Clamp到原图边界，防止取整越界
  roi.x = Math.min(roi.x, info.width - 1)
  roi.y = Math.min(roi.y, info.height - 1)
  roi.w = Math.min(roi.w, info.width - roi.x)
  roi.h = Math.min(roi.h, info.height - roi.y)
  image.setRoi(roi)
}

/** Displayed rectangle: live drag box, or the committed ROI mapped back to screen. */
const displayRect = computed(() => {
  if (drag.active) {
    return {
      left: Math.min(drag.startX, drag.curX),
      top: Math.min(drag.startY, drag.curY),
      width: Math.abs(drag.curX - drag.startX),
      height: Math.abs(drag.curY - drag.startY),
    }
  }
  const info = image.info
  const roi = image.roi
  const { w, h } = rendered.value
  if (!info || !roi || !w || !h) return null
  const sx = w / info.width
  const sy = h / info.height
  return { left: roi.x * sx, top: roi.y * sy, width: roi.w * sx, height: roi.h * sy }
})
</script>

<template>
  <div class="roi-wrap" @mousedown.prevent="onDown">
    <img ref="imgEl" :src="image.previewUrl ?? ''" alt="预览" draggable="false" @load="updateRendered" />
    <div
      v-if="displayRect"
      class="roi-box"
      :style="{
        left: displayRect.left + 'px',
        top: displayRect.top + 'px',
        width: displayRect.width + 'px',
        height: displayRect.height + 'px',
      }"
    />
  </div>
</template>

<style scoped>
.roi-wrap {
  position: relative;
  max-width: 100%;
  max-height: 100%;
  cursor: crosshair;
  line-height: 0;
}

.roi-wrap img {
  max-width: 100%;
  max-height: calc(100vh - 120px);
  display: block;
}

.roi-box {
  position: absolute;
  border: 2px solid var(--accent);
  background: rgba(79, 140, 255, 0.15);
  pointer-events: none;
}
</style>
