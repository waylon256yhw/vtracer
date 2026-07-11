<script setup lang="ts">
import ExperimentView from './views/ExperimentView.vue'
import { useImageStore } from './stores/image'

const image = useImageStore()
</script>

<template>
  <div class="app">
    <header class="toolbar">
      <span class="brand">VTracer Studio</span>
      <button class="primary" :disabled="image.loading" @click="image.openAndLoad()">
        {{ image.loading ? '加载中…' : '打开图片' }}
      </button>
      <span v-if="image.info" class="file-info">
        {{ image.path }} · {{ image.info.width }}×{{ image.info.height }} ·
        {{ (image.info.file_size / 1024 / 1024).toFixed(2) }} MB
      </span>
      <span v-if="image.error" class="error">{{ image.error }}</span>
    </header>
    <main class="content">
      <ExperimentView />
    </main>
  </div>
</template>

<style scoped>
.app {
  display: flex;
  flex-direction: column;
  height: 100%;
}

.toolbar {
  display: flex;
  align-items: center;
  gap: 12px;
  padding: 8px 14px;
  background: var(--bg-panel);
  border-bottom: 1px solid var(--border);
}

.brand {
  font-weight: 600;
  margin-right: 8px;
}

.file-info {
  color: var(--text-dim);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.error {
  color: var(--danger);
}

.content {
  flex: 1;
  min-height: 0;
}
</style>
