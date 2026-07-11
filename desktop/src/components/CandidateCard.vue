<script setup lang="ts">
import type { CandidateCell } from '../stores/experiment'

defineProps<{ cell: CandidateCell | null }>()
</script>

<template>
  <div class="card" :class="cell?.status">
    <template v-if="!cell || cell.status === 'pending'">
      <div class="spinner" />
    </template>
    <template v-else-if="cell.status === 'error'">
      <div class="err" :title="cell.message ?? ''">✕</div>
    </template>
    <template v-else>
      <img :src="cell.thumbUrl ?? ''" :alt="cell.id" loading="lazy" draggable="false" />
      <div v-if="cell.metrics" class="mini-metrics">
        <span :title="'路径元素数（SVG <path> 数量）'">{{ cell.metrics.paths }}路径</span>
        <span>{{ cell.metrics.colors }}色</span>
        <span>{{ (cell.metrics.svg_bytes / 1024).toFixed(0) }}K</span>
        <span v-if="cell.cached" class="cached" title="缓存命中">⚡</span>
      </div>
    </template>
  </div>
</template>

<style scoped>
.card {
  position: relative;
  aspect-ratio: 4 / 3;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 6px;
  overflow: hidden;
  display: flex;
  align-items: center;
  justify-content: center;
  cursor: pointer;
}

.card:hover {
  border-color: var(--accent);
}

.card img {
  width: 100%;
  height: 100%;
  object-fit: contain;
}

.spinner {
  width: 18px;
  height: 18px;
  border: 2px solid var(--border);
  border-top-color: var(--accent);
  border-radius: 50%;
  animation: spin 0.9s linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

.err {
  color: var(--danger);
  font-size: 20px;
}

.mini-metrics {
  position: absolute;
  left: 0;
  right: 0;
  bottom: 0;
  display: flex;
  gap: 6px;
  padding: 2px 6px;
  font-size: 10px;
  color: var(--text-dim);
  background: rgba(0, 0, 0, 0.55);
  font-variant-numeric: tabular-nums;
}

.cached {
  color: var(--accent);
}
</style>
