<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { api } from '../api'
import { useExperimentStore } from '../stores/experiment'
import { useViewportStore } from '../stores/viewport'

/**
 * Modal inspector for one candidate. Left pane: the FULL-RESOLUTION ROI
 * reference crop (the faithful original — never the downscaled whole-image
 * preview). Right pane: the live SVG, the only one mounted at any time.
 * Both panes bind to the shared viewport store, so pan/zoom stays in sync
 * between panes and across candidate switches (←/→). Holding Space hides
 * the SVG layer and shows the original in its place — instant A/B.
 */
const props = defineProps<{ candidateId: string }>()
const emit = defineEmits<{ close: []; adopt: [id: string]; navigate: [id: string] }>()

const exp = useExperimentStore()
const viewport = useViewportStore()

const svgText = ref<string | null>(null)
const svgError = ref<string | null>(null)
const abOriginal = ref(false)

const cell = computed(() => exp.cells[props.candidateId] ?? null)

/** Done candidates in stable grid order, for ←/→ navigation. */
const doneIds = computed(() => {
  const n = exp.norm
  const ids: string[] = []
  for (const p of n.color_precision)
    for (const g of n.gradient_step)
      for (const f of n.filter_speckle) {
        const c = exp.cell(g, f, p)
        if (c?.status === 'done') ids.push(c.id)
      }
  return ids
})

watch(
  () => props.candidateId,
  async (id) => {
    svgText.value = null
    svgError.value = null
    if (exp.runId === null) return
    try {
      svgText.value = await api.getCandidateSvg(exp.runId, id)
    } catch (e) {
      svgError.value = String(e)
    }
  },
  { immediate: true },
)

function navigate(delta: number) {
  const ids = doneIds.value
  const i = ids.indexOf(props.candidateId)
  if (i < 0 || ids.length === 0) return
  const next = ids[(i + delta + ids.length) % ids.length]
  emit('navigate', next)
}

function onKeyDown(e: KeyboardEvent) {
  if (e.key === 'Escape') emit('close')
  else if (e.key === 'ArrowLeft') navigate(-1)
  else if (e.key === 'ArrowRight') navigate(1)
  else if (e.code === 'Space' && !e.repeat) {
    e.preventDefault()
    abOriginal.value = true
  }
}

function onKeyUp(e: KeyboardEvent) {
  if (e.code === 'Space') {
    e.preventDefault()
    abOriginal.value = false
  }
}

onMounted(() => {
  window.addEventListener('keydown', onKeyDown)
  window.addEventListener('keyup', onKeyUp)
})
onUnmounted(() => {
  window.removeEventListener('keydown', onKeyDown)
  window.removeEventListener('keyup', onKeyUp)
})

// --- pan/zoom -------------------------------------------------------------

const dragging = ref(false)
let lastX = 0
let lastY = 0

function onWheel(e: WheelEvent) {
  e.preventDefault()
  const box = (e.currentTarget as HTMLElement).getBoundingClientRect()
  viewport.zoomAt(e.clientX - box.left, e.clientY - box.top, e.deltaY < 0 ? 1.2 : 1 / 1.2)
}

function onPointerDown(e: PointerEvent) {
  dragging.value = true
  lastX = e.clientX
  lastY = e.clientY
  ;(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId)
}

function onPointerMove(e: PointerEvent) {
  if (!dragging.value) return
  viewport.panBy(e.clientX - lastX, e.clientY - lastY)
  lastX = e.clientX
  lastY = e.clientY
}

function onPointerUp() {
  dragging.value = false
}
</script>

<template>
  <div class="overlay" @click.self="emit('close')">
    <div class="inspector">
      <header class="bar">
        <span class="title">{{ candidateId }}</span>
        <span v-if="cell?.metrics" class="metrics">
          路径元素 {{ cell.metrics.paths }} · 颜色 {{ cell.metrics.colors }} ·
          {{ (cell.metrics.svg_bytes / 1024).toFixed(1) }} KB · {{ cell.metrics.elapsed_ms }} ms
        </span>
        <span class="keys">←/→ 切换候选 · 按住空格看原图 · 滚轮缩放 · 拖拽平移 · Esc 关闭</span>
        <button @click="viewport.reset()">重置视图</button>
        <button class="primary" @click="emit('adopt', candidateId)">采用此参数</button>
        <button @click="emit('close')">关闭</button>
      </header>

      <div class="panes">
        <div
          class="pane"
          @wheel="onWheel"
          @pointerdown="onPointerDown"
          @pointermove="onPointerMove"
          @pointerup="onPointerUp"
        >
          <div class="pane-label">原图（全分辨率 ROI）</div>
          <div class="layer" :style="{ transform: viewport.cssTransform }">
            <img :src="exp.roiReferenceUrl ?? ''" alt="原图 ROI" draggable="false" />
          </div>
        </div>

        <div
          class="pane"
          @wheel="onWheel"
          @pointerdown="onPointerDown"
          @pointermove="onPointerMove"
          @pointerup="onPointerUp"
        >
          <div class="pane-label">{{ abOriginal ? '原图（A/B）' : '矢量结果 SVG' }}</div>
          <div class="layer" :style="{ transform: viewport.cssTransform }">
            <img
              v-show="abOriginal"
              :src="exp.roiReferenceUrl ?? ''"
              alt="原图 ROI"
              draggable="false"
            />
            <!-- eslint-disable-next-line vue/no-v-html : SVG 由本地后端生成 -->
            <div v-show="!abOriginal" v-if="svgText" class="svg-host" v-html="svgText" />
            <div v-else-if="svgError" class="svg-error">{{ svgError }}</div>
            <div v-else-if="!abOriginal" class="svg-loading">加载 SVG…</div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 100;
}

.inspector {
  width: 94vw;
  height: 92vh;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 10px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.bar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 12px;
  background: var(--bg-panel);
  border-bottom: 1px solid var(--border);
}

.title {
  font-weight: 600;
  font-variant-numeric: tabular-nums;
}

.metrics {
  color: var(--text-dim);
  font-size: 12px;
  font-variant-numeric: tabular-nums;
}

.keys {
  flex: 1;
  text-align: right;
  color: var(--text-dim);
  font-size: 11px;
}

.panes {
  flex: 1;
  display: flex;
  min-height: 0;
}

.pane {
  flex: 1;
  position: relative;
  overflow: hidden;
  cursor: grab;
  touch-action: none;
}

.pane:active {
  cursor: grabbing;
}

.pane + .pane {
  border-left: 1px solid var(--border);
}

.pane-label {
  position: absolute;
  top: 8px;
  left: 8px;
  z-index: 2;
  font-size: 11px;
  color: var(--text-dim);
  background: rgba(0, 0, 0, 0.5);
  padding: 2px 8px;
  border-radius: 4px;
  pointer-events: none;
}

.layer {
  position: absolute;
  top: 0;
  left: 0;
  transform-origin: 0 0;
  line-height: 0;
}

.layer img {
  image-rendering: pixelated;
  display: block;
}

.svg-host :deep(svg) {
  display: block;
}

.svg-loading,
.svg-error {
  padding: 24px;
  color: var(--text-dim);
  line-height: 1.5;
}

.svg-error {
  color: var(--danger);
}
</style>
