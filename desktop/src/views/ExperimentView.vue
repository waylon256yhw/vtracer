<script setup lang="ts">
import RoiSelector from '../components/RoiSelector.vue'
import { useImageStore } from '../stores/image'

const image = useImageStore()
</script>

<template>
  <div class="experiment">
    <div v-if="!image.hasImage" class="empty">
      <p>打开一张图片开始实验</p>
      <p class="hint">在原图上框选一块有代表性的区域（细节丰富处），参数矩阵将在该区域以原始分辨率试跑</p>
    </div>
    <div v-else class="workspace">
      <section class="canvas">
        <RoiSelector />
      </section>
      <aside class="sidebar">
        <h3>选区 (ROI)</h3>
        <template v-if="image.roi">
          <p class="roi-values">
            x={{ image.roi.x }} y={{ image.roi.y }}<br />
            {{ image.roi.w }}×{{ image.roi.h }} px（原图坐标）
          </p>
          <button @click="image.setRoi(null)">清除选区</button>
        </template>
        <p v-else class="hint">在左侧预览图上拖拽框选</p>
        <h3>参数矩阵</h3>
        <p class="hint">M2 里程碑功能：选择 gradient_step × filter_speckle × color_precision 的候选值，先跑稀疏 9 组</p>
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

.sidebar {
  width: 280px;
  border-left: 1px solid var(--border);
  background: var(--bg-panel);
  padding: 14px;
  overflow-y: auto;
}

.sidebar h3 {
  margin: 12px 0 6px;
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
