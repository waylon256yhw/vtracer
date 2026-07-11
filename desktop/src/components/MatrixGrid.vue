<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import CandidateCard from './CandidateCard.vue'
import { useExperimentStore } from '../stores/experiment'

/**
 * rows = gradient_step, cols = filter_speckle, tabs = color_precision.
 * Cells are PNG thumbnails only — a live SVG is mounted solely in the
 * inspector (M3).
 */
const exp = useExperimentStore()

const activeP = ref<number | null>(null)
const norm = computed(() => exp.norm)

watch(
  () => norm.value.color_precision,
  (ps) => {
    if (activeP.value === null || !ps.includes(activeP.value)) {
      activeP.value = ps[Math.floor(ps.length / 2)] ?? null
    }
  },
  { immediate: true },
)

const emit = defineEmits<{ inspect: [id: string] }>()

function onCellClick(g: number, f: number) {
  if (activeP.value === null) return
  const cell = exp.cell(g, f, activeP.value)
  if (cell?.status === 'done') emit('inspect', cell.id)
}
</script>

<template>
  <div class="grid-wrap">
    <div class="tabs">
      <button
        v-for="p in norm.color_precision"
        :key="p"
        class="tab"
        :class="{ on: activeP === p }"
        @click="activeP = p"
      >
        颜色精度 {{ p }}
      </button>
    </div>

    <div v-if="activeP !== null" class="matrix">
      <table>
        <thead>
          <tr>
            <th class="corner">步长 \ 斑点</th>
            <th v-for="f in norm.filter_speckle" :key="f">f={{ f }}</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="g in norm.gradient_step" :key="g">
            <th>g={{ g }}</th>
            <td v-for="f in norm.filter_speckle" :key="f">
              <CandidateCard
                :cell="exp.cell(g, f, activeP)"
                @click="onCellClick(g, f)"
              />
            </td>
          </tr>
        </tbody>
      </table>
    </div>
  </div>
</template>

<style scoped>
.tabs {
  display: flex;
  gap: 4px;
  margin-bottom: 8px;
}

.tab {
  border-radius: 6px 6px 0 0;
}

.tab.on {
  background: var(--accent);
  border-color: var(--accent);
  color: #fff;
}

.matrix {
  overflow: auto;
}

table {
  border-collapse: separate;
  border-spacing: 6px;
}

th {
  font-size: 11px;
  font-weight: 500;
  color: var(--text-dim);
  white-space: nowrap;
  font-variant-numeric: tabular-nums;
}

td {
  min-width: 140px;
}

.corner {
  text-align: left;
}
</style>
