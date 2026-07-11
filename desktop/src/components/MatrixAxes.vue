<script setup lang="ts">
import { useExperimentStore } from '../stores/experiment'
import { useImageStore } from '../stores/image'

/**
 * Axis value pickers. Curated value chips per axis; the matrix runs the
 * Cartesian product of everything checked.
 */
const exp = useExperimentStore()
const image = useImageStore()

const GRADIENT_CHOICES = [0, 4, 8, 16, 32, 64, 128]
const SPECKLE_CHOICES = [0, 1, 2, 4, 8, 16, 32, 64, 128]
const PRECISION_CHOICES = [1, 2, 3, 4, 5, 6, 7, 8]

function toggle(list: number[], v: number) {
  const i = list.indexOf(v)
  if (i >= 0) list.splice(i, 1)
  else list.push(v)
}

const axisDefs = [
  { key: 'gradient_step', label: '渐变步长 gradient_step', choices: GRADIENT_CHOICES, hint: '0 = 对角聚类模式（行为不同，非更细步长）' },
  { key: 'filter_speckle', label: '斑点过滤 filter_speckle', choices: SPECKLE_CHOICES, hint: '>16 超出 CLI 旧上限，适合高分辨率图' },
  { key: 'color_precision', label: '颜色精度 color_precision', choices: PRECISION_CHOICES, hint: '每通道有效位数' },
] as const
</script>

<template>
  <div class="axes">
    <div v-for="def in axisDefs" :key="def.key" class="axis">
      <div class="axis-label">{{ def.label }}</div>
      <div class="chips">
        <button
          v-for="v in def.choices"
          :key="v"
          class="chip"
          :class="{ on: exp.axes[def.key].includes(v) }"
          :disabled="exp.running"
          @click="toggle(exp.axes[def.key], v)"
        >
          {{ v }}
        </button>
      </div>
      <div class="axis-hint">{{ def.hint }}</div>
    </div>

    <div class="run-row">
      <span class="combo-count">共 {{ exp.totalCombinations }} 种组合</span>
      <button
        class="primary"
        :disabled="exp.running || !image.roi || exp.totalCombinations === 0"
        @click="exp.start('sparse')"
      >
        先跑稀疏组（≤9）
      </button>
      <button
        :disabled="exp.running || !image.roi || !exp.sparseDone"
        @click="exp.start('full')"
      >
        补齐完整矩阵
      </button>
      <button v-if="exp.running" class="danger" @click="exp.cancel()">取消</button>
    </div>
    <div v-if="exp.running" class="progress">
      运行中 {{ exp.completed }}/{{ exp.total }}
    </div>
    <div v-else-if="exp.cancelled" class="progress">已取消（{{ exp.completed }}/{{ exp.total }} 完成）</div>
    <div v-else-if="exp.sparseDone" class="progress hint-strong">
      稀疏组已完成（角点+中心，共 {{ exp.total }} 个）。灰色「待补齐」格子尚未运行——先取消明显不合适的参数值，再点「补齐完整矩阵」。
    </div>
    <div v-if="exp.error" class="error">{{ exp.error }}</div>
  </div>
</template>

<style scoped>
.axis {
  margin-bottom: 10px;
}

.axis-label {
  font-size: 13px;
  margin-bottom: 4px;
}

.axis-hint {
  font-size: 11px;
  color: var(--text-dim);
  margin-top: 2px;
}

.chips {
  display: flex;
  flex-wrap: wrap;
  gap: 4px;
}

.chip {
  padding: 2px 9px;
  border-radius: 12px;
  font-size: 12px;
  font-variant-numeric: tabular-nums;
}

.chip.on {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}

.run-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 8px;
  margin-top: 12px;
}

.combo-count {
  color: var(--text-dim);
  font-size: 12px;
}

button.danger {
  border-color: var(--danger);
  color: var(--danger);
}

.progress {
  margin-top: 8px;
  color: var(--text-dim);
  font-size: 13px;
}

.hint-strong {
  color: var(--accent);
  line-height: 1.5;
}

.error {
  margin-top: 8px;
  color: var(--danger);
  font-size: 13px;
}
</style>
