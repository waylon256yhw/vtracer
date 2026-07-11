<script setup lang="ts">
import { computed, ref } from 'vue'
import { api, presetConfig } from '../api'
import { useExperimentStore } from '../stores/experiment'
import { useRecipeStore } from '../stores/recipe'

/**
 * Non-axis parameters + presets, the adopted-candidate workflow (full-res
 * render → preview → export), and recipe/session file IO.
 */
const exp = useExperimentStore()
const recipe = useRecipeStore()

const showFullPreview = ref(false)
const fullPreviewUrl = computed(() =>
  recipe.fullResult ? api.assetUrl(recipe.fullResult.svg_path) : null,
)

const presets = [
  { key: 'bw', label: '黑白' },
  { key: 'poster', label: '海报' },
  { key: 'photo', label: '照片' },
] as const
</script>

<template>
  <div class="recipe-bar">
    <div class="row">
      <span class="lbl">预设</span>
      <button
        v-for="p in presets"
        :key="p.key"
        :disabled="exp.running"
        @click="exp.base = presetConfig(p.key)"
      >
        {{ p.label }}
      </button>
    </div>

    <div class="row">
      <span class="lbl">颜色</span>
      <select v-model="exp.base.color_mode" :disabled="exp.running">
        <option value="color">彩色</option>
        <option value="binary">黑白</option>
      </select>
      <select v-model="exp.base.hierarchical" :disabled="exp.running">
        <option value="stacked">堆叠</option>
        <option value="cutout">镂空</option>
      </select>
    </div>

    <div class="row">
      <span class="lbl">曲线</span>
      <select v-model="exp.base.mode" :disabled="exp.running">
        <option value="spline">样条</option>
        <option value="polygon">多边形</option>
        <option value="pixel">像素</option>
      </select>
    </div>

    <div class="row">
      <span class="lbl" title="拐角阈值（度）">拐角</span>
      <input v-model.number="exp.base.corner_threshold" type="number" min="0" max="180" :disabled="exp.running" />
      <span class="lbl" title="线段长度 [3.5,10]">线段</span>
      <input v-model.number="exp.base.segment_length" type="number" min="3.5" max="10" step="0.5" :disabled="exp.running" />
      <span class="lbl" title="拼接阈值（度）">拼接</span>
      <input v-model.number="exp.base.splice_threshold" type="number" min="0" max="180" :disabled="exp.running" />
    </div>

    <div class="row">
      <span class="lbl" title="曲线拟合最大迭代次数（至少 1）">迭代</span>
      <input v-model.number="exp.base.max_iterations" type="number" min="1" max="64" :disabled="exp.running" />
      <span class="lbl" title="路径坐标小数位 [0,8]">小数位</span>
      <input v-model.number="exp.base.path_precision" type="number" min="0" max="8" :disabled="exp.running" />
    </div>

    <div class="adopted">
      <template v-if="recipe.adopted">
        <div class="adopted-title">
          已采用：<b>{{ recipe.adoptedId ?? '（来自配方）' }}</b>
        </div>
        <div class="adopted-params">
          步长 {{ recipe.adopted.gradient_step }} · 斑点 {{ recipe.adopted.filter_speckle }} · 精度
          {{ recipe.adopted.color_precision }}
        </div>
        <div class="row">
          <button class="primary" :disabled="recipe.rendering" @click="recipe.renderFull()">
            {{ recipe.rendering ? `整图渲染中… ${recipe.renderSeconds}s` : '整图渲染' }}
          </button>
          <button :disabled="!recipe.fullResult" @click="showFullPreview = true">预览</button>
          <button :disabled="!recipe.fullResult" @click="recipe.exportSvg()">导出 SVG</button>
        </div>
        <div v-if="recipe.fullResult" class="full-metrics">
          整图：{{ recipe.fullResult.metrics.paths }} 路径元素 ·
          {{ recipe.fullResult.metrics.colors }} 色 ·
          {{ (recipe.fullResult.metrics.svg_bytes / 1024).toFixed(0) }} KB ·
          {{ (recipe.fullResult.metrics.elapsed_ms / 1000).toFixed(1) }}s
        </div>
      </template>
      <p v-else class="hint">在检查器中点「采用此参数」后，可整图渲染与导出</p>
    </div>

    <div class="row">
      <button :disabled="!recipe.adopted" @click="recipe.saveRecipe()">保存配方</button>
      <button @click="recipe.loadRecipe()">加载配方</button>
    </div>
    <div class="row">
      <button @click="recipe.saveSession()">保存会话</button>
      <button @click="recipe.loadSession()">加载会话</button>
    </div>

    <div v-if="recipe.notice" class="notice">{{ recipe.notice }}</div>
    <div v-if="recipe.error" class="error">{{ recipe.error }}</div>

    <!-- 整图结果预览：导出前先确认整图效果 -->
    <div v-if="showFullPreview && fullPreviewUrl" class="full-overlay" @click.self="showFullPreview = false">
      <div class="full-box">
        <header class="full-bar">
          <span>整图渲染结果预览</span>
          <button @click="showFullPreview = false">关闭</button>
        </header>
        <div class="full-body">
          <img :src="fullPreviewUrl" alt="整图渲染结果" />
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.recipe-bar {
  display: flex;
  flex-direction: column;
  gap: 8px;
}

.row {
  display: flex;
  align-items: center;
  gap: 6px;
  flex-wrap: wrap;
}

.lbl {
  font-size: 12px;
  color: var(--text-dim);
  min-width: 28px;
}

select,
input[type='number'] {
  font: inherit;
  color: var(--text);
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 5px;
  padding: 3px 6px;
}

input[type='number'] {
  width: 58px;
}

.adopted {
  border: 1px solid var(--border);
  border-radius: 6px;
  padding: 8px;
  display: flex;
  flex-direction: column;
  gap: 6px;
}

.adopted-title {
  font-size: 13px;
}

.adopted-params,
.full-metrics {
  font-size: 12px;
  color: var(--text-dim);
  font-variant-numeric: tabular-nums;
}

.hint {
  margin: 0;
  color: var(--text-dim);
  font-size: 12px;
  line-height: 1.5;
}

.notice {
  font-size: 12px;
  color: var(--accent);
  word-break: break-all;
}

.error {
  font-size: 12px;
  color: var(--danger);
  word-break: break-all;
}

.full-overlay {
  position: fixed;
  inset: 0;
  background: rgba(0, 0, 0, 0.6);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 120;
}

.full-box {
  width: 90vw;
  height: 88vh;
  background: var(--bg);
  border: 1px solid var(--border);
  border-radius: 10px;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

.full-bar {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
  background: var(--bg-panel);
  border-bottom: 1px solid var(--border);
}

.full-body {
  flex: 1;
  overflow: auto;
  display: flex;
  align-items: center;
  justify-content: center;
}

.full-body img {
  max-width: 100%;
  max-height: 100%;
  object-fit: contain;
}
</style>
