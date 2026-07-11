<script setup lang="ts">
import { computed, ref } from 'vue'
import { useExperimentStore, type CandidateCell } from '../stores/experiment'

/** Sortable table of completed candidates. Row click opens the inspector. */
const exp = useExperimentStore()
const emit = defineEmits<{ inspect: [id: string] }>()

type SortKey = 'id' | 'paths' | 'colors' | 'svg_bytes' | 'elapsed_ms'
const sortKey = ref<SortKey>('svg_bytes')
const sortAsc = ref(true)

const columns: { key: SortKey; label: string; title?: string }[] = [
  { key: 'id', label: '候选' },
  { key: 'paths', label: '路径元素数', title: 'SVG <path> 元素数量（堆叠图层数），非贝塞尔段数' },
  { key: 'colors', label: '颜色数' },
  { key: 'svg_bytes', label: '大小' },
  { key: 'elapsed_ms', label: '耗时' },
]

function setSort(key: SortKey) {
  if (sortKey.value === key) sortAsc.value = !sortAsc.value
  else {
    sortKey.value = key
    sortAsc.value = true
  }
}

const rows = computed(() => {
  const val = (c: CandidateCell) =>
    sortKey.value === 'id' ? c.id : (c.metrics?.[sortKey.value] ?? 0)
  return [...exp.doneCells].sort((a, b) => {
    const av = val(a)
    const bv = val(b)
    const cmp = typeof av === 'string' ? av.localeCompare(String(bv)) : Number(av) - Number(bv)
    return sortAsc.value ? cmp : -cmp
  })
})
</script>

<template>
  <div v-if="rows.length" class="metrics-panel">
    <table>
      <thead>
        <tr>
          <th
            v-for="col in columns"
            :key="col.key"
            :title="col.title"
            @click="setSort(col.key)"
          >
            {{ col.label }}
            <span v-if="sortKey === col.key">{{ sortAsc ? '▲' : '▼' }}</span>
          </th>
        </tr>
      </thead>
      <tbody>
        <tr v-for="c in rows" :key="c.id" @click="emit('inspect', c.id)">
          <td>{{ c.id }}</td>
          <td>{{ c.metrics?.paths }}</td>
          <td>{{ c.metrics?.colors }}</td>
          <td>{{ ((c.metrics?.svg_bytes ?? 0) / 1024).toFixed(1) }} KB</td>
          <td>{{ c.metrics?.elapsed_ms }} ms</td>
        </tr>
      </tbody>
    </table>
  </div>
</template>

<style scoped>
.metrics-panel {
  max-height: 260px;
  overflow-y: auto;
  border: 1px solid var(--border);
  border-radius: 6px;
}

table {
  width: 100%;
  border-collapse: collapse;
  font-size: 12px;
  font-variant-numeric: tabular-nums;
}

th {
  position: sticky;
  top: 0;
  background: var(--bg-panel);
  text-align: left;
  padding: 5px 8px;
  cursor: pointer;
  white-space: nowrap;
  color: var(--text-dim);
  font-weight: 500;
  user-select: none;
}

td {
  padding: 4px 8px;
  border-top: 1px solid var(--border);
  white-space: nowrap;
}

tbody tr {
  cursor: pointer;
}

tbody tr:hover {
  background: var(--bg-panel);
}
</style>
