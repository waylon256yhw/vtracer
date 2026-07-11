import { defineStore } from 'pinia'
import {
  api,
  parseCandidateId,
  type ExperimentSession,
  type FullResult,
  type Recipe,
  type StudioConfig,
} from '../api'
import { useExperimentStore } from './experiment'
import { useImageStore } from './image'
import { useViewportStore } from './viewport'

/**
 * The adopted config (base + selected candidate's axis values), the full-res
 * render result, and recipe/session file IO. Export always copies the exact
 * bytes of the last render — never re-converts.
 */
export const useRecipeStore = defineStore('recipe', {
  state: () => ({
    adoptedId: null as string | null,
    adopted: null as StudioConfig | null,
    fullResult: null as FullResult | null,
    rendering: false,
    renderSeconds: 0,
    busy: null as string | null,
    error: null as string | null,
    notice: null as string | null,
  }),

  actions: {
    /** Take a candidate's params on top of the current base config. */
    adopt(candidateId: string) {
      const exp = useExperimentStore()
      const params = parseCandidateId(candidateId)
      if (!params) return
      this.adoptedId = candidateId
      this.adopted = {
        ...exp.base,
        gradient_step: params.gradient_step,
        filter_speckle: params.filter_speckle,
        color_precision: params.color_precision,
      }
      this.fullResult = null // 参数变了，旧渲染不再代表当前选择
      this.error = null
      this.notice = null
    },

    async renderFull() {
      if (!this.adopted || this.rendering) return
      this.rendering = true
      this.renderSeconds = 0
      this.error = null
      const timer = setInterval(() => this.renderSeconds++, 1000)
      try {
        this.fullResult = await api.renderFull({ ...this.adopted })
      } catch (e) {
        this.error = String(e)
      } finally {
        clearInterval(timer)
        this.rendering = false
      }
    },

    async exportSvg() {
      if (!this.fullResult) return
      const image = useImageStore()
      const base = (image.path ?? 'output').replace(/^.*[\\/]/, '').replace(/\.[^.]+$/, '')
      const path = await api.pickSavePath(`${base}.svg`, 'SVG 矢量图', ['svg'])
      if (!path) return
      try {
        await api.exportResult(this.fullResult.result_id, path)
        this.notice = `已导出：${path}`
      } catch (e) {
        this.error = String(e)
      }
    },

    async saveRecipe() {
      if (!this.adopted) return
      const image = useImageStore()
      const path = await api.pickSavePath('vtracer-recipe.json', '配方', ['json'])
      if (!path) return
      const recipe: Recipe = {
        version: 1,
        app: 'vtracer-studio',
        created_at: new Date().toISOString(),
        config: { ...this.adopted },
        scaling: {
          filter_speckle_mode: 'fixed',
          reference_long_edge: Math.max(image.info?.width ?? 0, image.info?.height ?? 0),
        },
      }
      try {
        await api.saveRecipe(recipe, path)
        this.notice = `配方已保存：${path}`
      } catch (e) {
        this.error = String(e)
      }
    },

    async loadRecipe() {
      const path = await api.pickOpenPath('配方', ['json'])
      if (!path) return
      try {
        const recipe = await api.loadRecipe(path)
        const exp = useExperimentStore()
        exp.base = { ...recipe.config }
        this.adopted = { ...recipe.config }
        this.adoptedId = null
        this.fullResult = null
        this.notice = `配方已加载：${path}`
      } catch (e) {
        this.error = String(e)
      }
    },

    async saveSession() {
      const image = useImageStore()
      const exp = useExperimentStore()
      const viewport = useViewportStore()
      if (!image.info || !image.path || !image.roi) {
        this.error = '当前没有可保存的实验（需要图片和 ROI）'
        return
      }
      const path = await api.pickSavePath('vtracer-session.json', '实验会话', ['json'])
      if (!path) return
      const session: ExperimentSession = {
        version: 1,
        source: {
          file: image.path,
          width: image.info.width,
          height: image.info.height,
          blake3: image.info.hash,
        },
        base_config: { ...exp.base },
        roi: { ...image.roi },
        axes: exp.norm,
        selected_candidate: this.adoptedId,
        viewport: { scale: viewport.scale, x: viewport.x, y: viewport.y },
      }
      try {
        await api.saveSession(session, path)
        this.notice = `会话已保存：${path}`
      } catch (e) {
        this.error = String(e)
      }
    },

    async loadSession() {
      const path = await api.pickOpenPath('实验会话', ['json'])
      if (!path) return
      try {
        const session = await api.loadSession(path)
        const image = useImageStore()
        const exp = useExperimentStore()
        const viewport = useViewportStore()

        await image.loadPath(session.source.file)
        if (image.error) return
        if (image.info && image.info.hash !== session.source.blake3) {
          this.notice = '注意：图片内容与保存会话时不一致（文件被修改过），实验结果可能不同'
        } else {
          this.notice = `会话已恢复：${path}`
        }
        exp.reset()
        exp.base = { ...session.base_config }
        exp.axes = {
          gradient_step: [...session.axes.gradient_step],
          filter_speckle: [...session.axes.filter_speckle],
          color_precision: [...session.axes.color_precision],
        }
        image.setRoi({ ...session.roi })
        if (session.selected_candidate) this.adopt(session.selected_candidate)
        if (session.viewport) {
          viewport.scale = session.viewport.scale
          viewport.x = session.viewport.x
          viewport.y = session.viewport.y
        }
      } catch (e) {
        this.error = String(e)
      }
    },
  },
})
