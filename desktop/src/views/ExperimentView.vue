<script setup lang="ts">
import { watch } from 'vue'
import RoiSelector from '../components/RoiSelector.vue'
import MatrixAxes from '../components/MatrixAxes.vue'
import MatrixGrid from '../components/MatrixGrid.vue'
import { useImageStore } from '../stores/image'
import { useExperimentStore } from '../stores/experiment'

const image = useImageStore()
const exp = useExperimentStore()

// Changing image or ROI invalidates every candidate on screen.
watch(
  () => [image.info?.hash, image.roi?.x, image.roi?.y, image.roi?.w, image.roi?.h],
  (next, prev) => {
    if (prev && next.some((v, i) => v !== prev[i])) exp.reset()
  },
)

function onInspect(id: string) {
  // M3: opens the candidate inspector (synced pan/zoom + A/B vs original).
  console.log('inspect', id)
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

        <template v-if="Object.keys(exp.cells).length">
          <h3>视图</h3>
          <button :disabled="exp.running" @click="exp.reset()">返回 ROI 选择</button>
        </template>
      </aside>
    </div>
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
