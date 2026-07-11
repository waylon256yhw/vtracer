import { defineStore } from 'pinia'
import {
  api,
  candidateId,
  normalizedAxes,
  type MatrixAxes,
  type MatrixEvent,
  type Metrics,
  type RunMode,
  type StudioConfig,
} from '../api'
import { defaultConfig } from '../api'
import { useImageStore } from './image'

export interface CandidateCell {
  id: string
  status: 'pending' | 'done' | 'error'
  cached: boolean
  metrics: Metrics | null
  thumbUrl: string | null
  message: string | null
}

export const useExperimentStore = defineStore('experiment', {
  state: () => ({
    axes: {
      gradient_step: [8, 16, 32],
      filter_speckle: [2, 4, 8],
      color_precision: [4, 6, 8],
    } as MatrixAxes,
    base: defaultConfig() as StudioConfig,
    runId: null as number | null,
    /** Monotonic guard: events captured under an older generation are stale. */
    generation: 0,
    running: false,
    lastMode: null as RunMode | null,
    sparseDone: false,
    total: 0,
    completed: 0,
    cancelled: false,
    cells: {} as Record<string, CandidateCell>,
    roiReferenceUrl: null as string | null,
    error: null as string | null,
  }),

  getters: {
    /** Normalized axes — the exact values the backend will run. */
    norm: (s) => normalizedAxes(s.axes),
    totalCombinations(): number {
      const n = this.norm
      return n.gradient_step.length * n.filter_speckle.length * n.color_precision.length
    },
    doneCells(): CandidateCell[] {
      return Object.values(this.cells).filter((c) => c.status === 'done')
    },
  },

  actions: {
    cell(g: number, f: number, p: number): CandidateCell | null {
      return this.cells[candidateId(g, f, p)] ?? null
    },

    async start(mode: RunMode) {
      const image = useImageStore()
      if (!image.info || !image.roi) {
        this.error = '请先打开图片并框选 ROI'
        return
      }
      this.generation++
      const gen = this.generation
      this.error = null
      this.running = true
      this.cancelled = false
      this.lastMode = mode
      this.completed = 0
      // Full run keeps existing cells (cache hits will refresh them); a
      // sparse run over possibly-changed axes starts clean.
      if (mode === 'sparse') this.cells = {}

      try {
        const started = await api.runMatrix(
          { roi: image.roi, base: { ...this.base }, axes: this.norm, mode },
          (e) => {
            if (this.generation === gen) this.onEvent(e)
          },
        )
        if (this.generation !== gen) return // superseded while awaiting
        this.runId = started.run_id
        this.total = started.total
        this.roiReferenceUrl = api.assetUrl(started.roi_reference_path)
        for (const id of started.order) {
          if (!this.cells[id] || this.cells[id].status === 'error') {
            this.cells[id] = { id, status: 'pending', cached: false, metrics: null, thumbUrl: null, message: null }
          }
        }
      } catch (e) {
        if (this.generation === gen) {
          this.error = String(e)
          this.running = false
        }
      }
    },

    onEvent(e: MatrixEvent) {
      switch (e.type) {
        case 'CandidateDone':
          this.cells[e.id] = {
            id: e.id,
            status: 'done',
            cached: e.cached,
            metrics: e.metrics,
            thumbUrl: api.assetUrl(e.thumb_path),
            message: null,
          }
          this.completed++
          break
        case 'CandidateError':
          this.cells[e.id] = {
            id: e.id,
            status: 'error',
            cached: false,
            metrics: null,
            thumbUrl: null,
            message: e.message,
          }
          this.completed++
          break
        case 'Finished':
          this.running = false
          this.cancelled = e.cancelled
          if (this.lastMode === 'sparse' && !e.cancelled) this.sparseDone = true
          if (this.lastMode === 'full' && !e.cancelled) this.sparseDone = false
          break
      }
    },

    async cancel() {
      if (this.runId !== null && this.running) {
        await api.cancelRun(this.runId)
      }
    },

    reset() {
      this.generation++
      this.runId = null
      this.running = false
      this.sparseDone = false
      this.total = 0
      this.completed = 0
      this.cancelled = false
      this.cells = {}
      this.roiReferenceUrl = null
      this.error = null
    },
  },
})
