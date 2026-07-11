import type {
  ImageInfo,
  MatrixEvent,
  MatrixRequest,
  RunStarted,
  StudioApi,
} from './types'
import { candidateId, normalizedAxes } from './types'

/**
 * Browser-only mock for headless UI development. Deliberately hostile: events
 * arrive out of order with random delays, one candidate fails, cancellation
 * skips pending candidates, and timers of a superseded run KEEP FIRING with
 * the old run_id — the store's stale-event guard must drop them.
 */

const MOCK_W = 2400
const MOCK_H = 1600

let mockRunId = 0
let cancelledRuns = new Set<number>()

function mockSceneSvg(detail: number, colors: number): string {
  // Pseudo-vectorization: fewer colors / more smoothing as params grow.
  const palette = ['#e9c46a', '#e76f51', '#2a9d8f', '#f4a261', '#a8dadc', '#457b9d', '#e63946', '#8ecae6']
  const shapes = Array.from({ length: 3 + detail }, (_, i) => {
    const c = palette[i % Math.max(2, Math.min(colors, palette.length))]
    const x = 40 + ((i * 137) % 500)
    const y = 40 + ((i * 89) % 300)
    const r = 20 + ((i * 53) % 90)
    return i % 2 === 0
      ? `<circle cx="${x}" cy="${y}" r="${r}" fill="${c}"/>`
      : `<rect x="${x}" y="${y}" width="${r * 2}" height="${r * 1.4}" rx="${8 - Math.min(7, detail)}" fill="${c}"/>`
  }).join('')
  return `<svg xmlns="http://www.w3.org/2000/svg" width="600" height="400" viewBox="0 0 600 400"><rect width="600" height="400" fill="#264653"/>${shapes}</svg>`
}

function dataUrl(svg: string): string {
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`
}

function mockPreviewDataUrl(): string {
  return dataUrl(mockSceneSvg(9, 8))
}

export const mockApi: StudioApi = {
  async pickImageFile() {
    return 'C:\\mock\\示例图片.png'
  },

  async loadImage(path: string): Promise<ImageInfo> {
    await sleep(300)
    return {
      width: MOCK_W,
      height: MOCK_H,
      file_size: 3_456_789,
      hash: 'mockhash0123456789abcdef',
      preview_path: path,
    }
  },

  assetUrl(path: string) {
    if (path.startsWith('data:')) return path
    if (path.includes('roi-ref')) return dataUrl(mockSceneSvg(11, 8))
    return mockPreviewDataUrl()
  },

  async runMatrix(req: MatrixRequest, onEvent: (e: MatrixEvent) => void): Promise<RunStarted> {
    await sleep(120)
    const runId = ++mockRunId
    const axes = normalizedAxes(req.axes)

    // Corners + center first (mirror of sparse_set), then the rest.
    const ends = (v: number[]) => (v.length === 1 ? [v[0]] : [v[0], v[v.length - 1]])
    const sparse: string[] = []
    for (const g of ends(axes.gradient_step))
      for (const f of ends(axes.filter_speckle))
        for (const p of ends(axes.color_precision)) {
          const id = candidateId(g, f, p)
          if (!sparse.includes(id)) sparse.push(id)
        }
    const centerId = candidateId(
      axes.gradient_step[Math.floor(axes.gradient_step.length / 2)],
      axes.filter_speckle[Math.floor(axes.filter_speckle.length / 2)],
      axes.color_precision[Math.floor(axes.color_precision.length / 2)],
    )
    if (!sparse.includes(centerId)) sparse.push(centerId)

    const all: { id: string; g: number; f: number; p: number }[] = []
    for (const g of axes.gradient_step)
      for (const f of axes.filter_speckle)
        for (const p of axes.color_precision) all.push({ id: candidateId(g, f, p), g, f, p })

    const inSparse = (id: string) => sparse.includes(id)
    const scheduled =
      req.mode === 'sparse'
        ? all.filter((c) => inSparse(c.id))
        : [...all.filter((c) => inSparse(c.id)), ...all.filter((c) => !inSparse(c.id))]

    let remaining = scheduled.length
    let completed = 0
    let finished = false
    const finishIfDrained = () => {
      if (remaining === 0 && !finished) {
        finished = true
        onEvent({ type: 'Finished', run_id: runId, completed, cancelled: cancelledRuns.has(runId) })
      }
    }

    scheduled.forEach((c, i) => {
      // Random delay per candidate → out-of-order arrival, sparse block first.
      const base = inSparse(c.id) ? 200 : 1800
      const delay = base + Math.random() * 1400 + i * 60
      setTimeout(() => {
        remaining--
        if (cancelledRuns.has(runId)) {
          finishIfDrained()
          return
        }
        completed++
        if (i === 3) {
          onEvent({
            type: 'CandidateError',
            run_id: runId,
            id: c.id,
            message: '模拟错误：该参数组合转换失败',
          })
        } else {
          onEvent({
            type: 'CandidateDone',
            run_id: runId,
            id: c.id,
            params: { id: c.id, gradient_step: c.g, filter_speckle: c.f, color_precision: c.p },
            metrics: {
              paths: 40 + c.p * 60 - c.f * 3 + Math.round(600 / (c.g + 1)),
              colors: Math.min(2 ** c.p, 64),
              svg_bytes: 12_000 + c.p * 9_000 - c.f * 500 + Math.round(90_000 / (c.g + 2)),
              elapsed_ms: Math.round(300 + Math.random() * 900),
            },
            thumb_path: dataUrl(mockSceneSvg(Math.max(2, 10 - c.f), 2 ** Math.min(c.p, 3))),
            cached: false,
          })
        }
        finishIfDrained()
      }, delay)
    })

    return {
      run_id: runId,
      total: scheduled.length,
      order: scheduled.map((c) => c.id),
      roi_reference_path: 'mock://roi-ref',
    }
  },

  async cancelRun(runId: number) {
    cancelledRuns.add(runId)
  },

  async getCandidateSvg(_runId: number, id: string) {
    await sleep(150)
    const m = id.match(/^g(\d+)_f(\d+)_p(\d+)$/)
    const f = m ? Number(m[2]) : 4
    const p = m ? Number(m[3]) : 6
    return mockSceneSvg(Math.max(2, 10 - f), 2 ** Math.min(p, 3))
  },
}

function sleep(ms: number) {
  return new Promise((r) => setTimeout(r, ms))
}
