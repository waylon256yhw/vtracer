import type { ImageInfo, StudioApi } from './types'

/**
 * Browser-only mock for headless UI development. M1 covers image loading;
 * M2 will extend this with a matrix event stream including out-of-order
 * arrival, stale-run events, cancellation and candidate errors.
 */

const MOCK_W = 2400
const MOCK_H = 1600

function mockPreviewDataUrl(): string {
  const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="800" viewBox="0 0 1200 800">
    <rect width="1200" height="800" fill="#264653"/>
    <circle cx="400" cy="380" r="220" fill="#e9c46a"/>
    <rect x="640" y="180" width="420" height="420" rx="24" fill="#e76f51"/>
    <path d="M 120 700 Q 400 520 700 680 T 1150 640" stroke="#2a9d8f" stroke-width="36" fill="none"/>
    <text x="60" y="90" font-family="sans-serif" font-size="48" fill="#ffffffaa">mock preview 2400×1600</text>
  </svg>`
  return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`
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

  assetUrl() {
    return mockPreviewDataUrl()
  },
}

function sleep(ms: number) {
  return new Promise((r) => setTimeout(r, ms))
}
