<script setup lang="ts">
import { ref, watch } from 'vue'
import RoiSelector from '../components/RoiSelector.vue'
import MatrixAxes from '../components/MatrixAxes.vue'
import MatrixGrid from '../components/MatrixGrid.vue'
import MetricsPanel from '../components/MetricsPanel.vue'
import CandidateInspector from '../components/CandidateInspector.vue'
import RecipeBar from '../components/RecipeBar.vue'
import { useImageStore } from '../stores/image'
import { useExperimentStore } from '../stores/experiment'
import { useRecipeStore } from '../stores/recipe'
import { useViewportStore } from '../stores/viewport'

const image = useImageStore()
const exp = useExperimentStore()
const recipe = useRecipeStore()
const viewport = useViewportStore()

/** Non-null while the inspector modal is open — only one live SVG at a time. */
const inspectId = ref<string | null>(null)

// Changing image or ROI invalidates every candidate on screen, the viewport,
// and any full-image render (it belongs to the old image/ROI).
watch(
  () => [image.info?.hash, image.roi?.x, image.roi?.y, image.roi?.w, image.roi?.h],
  (next, prev) => {
    if (recipe.restoring) return
    if (prev && next.some((v, i) => v !== prev[i])) {
      inspectId.value = null
      exp.reset()
      viewport.reset()
      recipe.invalidateRender()
    }
  },
)

// Changing any non-axis base parameter (including via presets or loadRecipe)
// makes existing results stale — they were computed with the old base and
// must not be inspected/adopted as if they matched the current one.
watch(
  () => exp.base,
  () => {
    if (recipe.restoring) return
    if (Object.keys(exp.cells).length) {
      inspectId.value = null
      exp.reset()
      viewport.reset()
    }
  },
  { deep: true },
)

function onInspect(id: string) {
  inspectId.value = id
}

function onAdopt(id: string) {
  recipe.adopt(id)
  inspectId.value = null
}
</script>

<template>
  <div class="experiment">
    <div v-if="!image.hasImage" class="empty">
      <p>打开一张图片开始实验</p>
      <p class="hint">在原图上框选一块有代表性的区域（细节丰富处），参数矩阵将在该区域以原始分辨率试跑</p>
    </div>
    <div v-else class="workspace">
      <section class="canvas">
        <div v-if="Object.keys(exp.cells).length" class="grid-area">
          <MatrixGrid @inspect="onInspect" />
        </div>
        <div v-else class="roi-area">
          <RoiSelector />
        </div>
      </section>
      <aside class="sidebar">
        <h3>选区 (ROI)</h3>
        <template v-if="image.roi">
          <p class="roi-values">
            x={{ image.roi.x }} y={{ image.roi.y }}<br />
            {{ image.roi.w }}×{{ image.roi.h }} px（原图坐标）
          </p>
          <button :disabled="exp.running" @click="image.setRoi(null)">清除选区</button>
        </template>
        <p v-else class="hint">在左侧预览图上拖拽框选</p>

        <h3>参数矩阵</h3>
        <MatrixAxes />

        <h3>配方与导出</h3>
        <RecipeBar />

        <template v-if="Object.keys(exp.cells).length">
          <h3>指标对比</h3>
          <MetricsPanel @inspect="onInspect" />
          <h3>视图</h3>
          <button :disabled="exp.running" @click="exp.reset()">返回 ROI 选择</button>
        </template>
      </aside>
    </div>

    <CandidateInspector
      v-if="inspectId"
      :candidate-id="inspectId"
      @close="inspectId = null"
      @adopt="onAdopt"
      @navigate="(id) => (inspectId = id)"
    />
  </div>
</template>

<style scoped>
.experiment {
  height: 100%;
}

.empty {
  height: 100%;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  color: var(--text-dim);
}

.workspace {
  display: flex;
  height: 100%;
}

.canvas {
  flex: 1;
  min-width: 0;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 16px;
  overflow: hidden;
}

.grid-area {
  width: 100%;
  height: 100%;
  overflow: auto;
}

.sidebar {
  width: 320px;
  border-left: 1px solid var(--border);
  background: var(--bg-panel);
  padding: 14px;
  overflow-y: auto;
}

.sidebar h3 {
  margin: 14px 0 6px;
  font-size: 13px;
  color: var(--text-dim);
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.sidebar h3:first-child {
  margin-top: 0;
}

.roi-values {
  font-variant-numeric: tabular-nums;
}

.hint {
  color: var(--text-dim);
  font-size: 13px;
  line-height: 1.5;
}
</style>
