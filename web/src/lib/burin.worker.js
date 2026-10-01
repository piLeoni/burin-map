/**
 * Draws off the main thread. On a cross-origin isolated page it loads the
 * threaded engine (public/pkg-mt) and starts a rayon pool of Web Workers;
 * anywhere else, or if that fails, the single-threaded one (public/pkg).
 * With `gpu` set, terrain fields are built on WebGPU when the browser has it.
 */

// Function so Vite does not rewrite imports of files in /public (as in burin.js).
const importPkg = new Function('url', 'return import(url)')

let engine = null
let gpu = null

function load() {
  engine ??= (async () => {
    if (self.crossOriginIsolated) {
      try {
        const m = await importPkg('/pkg-mt/burin_map_wasm.js')
        await m.default()
        const threads = Math.max(1, navigator.hardwareConcurrency || 4)
        await m.initThreadPool(threads)
        return { m, threads }
      } catch (e) {
        console.warn('burin-map: threaded engine unavailable, using one thread', e)
      }
    }
    const m = await importPkg('/pkg/burin_map_wasm.js')
    await m.default()
    return { m, threads: 1 }
  })()
  return engine
}

/** Adapter description, or null without WebGPU. */
function loadGpu(m) {
  gpu ??= (async () => {
    if (!navigator.gpu || !m.initGpu) return null
    try {
      const name = await m.initGpu()
      // The backend reports "(BrowserWebGpu)" with an empty adapter name on the
      // web, which would render as a stray double space in the counters.
      return name?.replace(/\s*\(([^)]+)\)/, ' $1').trim() || null
    } catch (e) {
      console.warn('burin-map: WebGPU unavailable, drawing on the CPU', e)
      return null
    }
  })()
  return gpu
}

self.onmessage = async ({ data }) => {
  const { id, elev, width, height, pxM, json, geo, mmPerPx, water } = data
  try {
    const timings = []
    let t = performance.now()
    const lap = (label) => {
      const now = performance.now()
      timings.push([label, now - t])
      t = now
    }
    const { m, threads } = await load()
    lap('worker: engine ready')
    const adapter = data.gpu ? await loadGpu(m) : null
    if (data.gpu) lap('worker: WebGPU ready')
    // Shared only by the threaded build: then the page can watch the draw.
    const board = m.progressBoard?.()
    if (typeof SharedArrayBuffer !== 'undefined' && board?.buffer instanceof SharedArrayBuffer) {
      self.postMessage({ id, board, draws: Atomics.load(board, 0) + 1 })
    }
    const d = adapter != null
      ? await m.draw_batches_gpu(elev, width, height, pxM, json, geo ?? null, mmPerPx ?? undefined, water ?? null)
      : m.draw_batches(elev, width, height, pxM, json, geo ?? null, mmPerPx ?? undefined, water ?? null)
    timings.push(...d.timings)
    t = performance.now()
    const svg = d.svg
    lap('worker: read svg')
    const geojson = d.geojson
    lap('worker: build geojson')
    const result = {
      svg, geojson, lines: d.lines, dashes: d.dashes, ms: d.ms, threads, timings,
      gpu: data.gpu ? (adapter ?? false) : null, sent: performance.timeOrigin + performance.now(),
    }
    d.free()
    self.postMessage({ id, ok: true, result })
  } catch (e) {
    self.postMessage({ id, ok: false, error: String(e?.message ?? e) })
  }
}
