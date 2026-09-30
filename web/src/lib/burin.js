/**
 * The burin-map WASM engine: loaded on the page for the style schema, and in
 * a Web Worker (burin.worker.js) for drawing.
 */

let ready = null

export async function loadBurin() {
  if (!ready) {
    // Built by wasm-pack into public/pkg. Use Function so Vite does not try
    // to bundle / analyse the file (it rejects JS imports from /public).
    ready = Function('return import("/pkg/burin_map_wasm.js")')().then(async (m) => {
      await m.default()
      return m
    })
  }
  return ready
}

export function paintDrawing(canvas, drawing, style) {
  const w = drawing.width
  const h = drawing.height
  canvas.width = w
  canvas.height = h
  const ctx = canvas.getContext('2d')
  ctx.fillStyle = '#ffffff'
  ctx.fillRect(0, 0, w, h)

  const paths = drawing.paths
  const ends = drawing.ends
  const stroke = style?.batch_stroke ?? 1.2
  ctx.strokeStyle = '#000000'
  ctx.lineWidth = stroke
  ctx.lineCap = 'round'
  ctx.lineJoin = 'round'

  let start = 0
  for (let i = 0; i < ends.length; i++) {
    const end = ends[i]
    if (end <= start) {
      start = end
      continue
    }
    ctx.beginPath()
    ctx.moveTo(paths[start * 2], paths[start * 2 + 1])
    for (let p = start + 1; p < end; p++) {
      ctx.lineTo(paths[p * 2], paths[p * 2 + 1])
    }
    if (end - start === 1) {
      ctx.lineTo(paths[start * 2] + 0.01, paths[start * 2 + 1])
    }
    ctx.stroke()
    start = end
  }
}

let worker = null
let nextId = 0
const pending = new Map()

function drawWorker() {
  if (!worker) {
    worker = new Worker(new URL('./burin.worker.js', import.meta.url), { type: 'module' })
    worker.onmessage = ({ data }) => {
      const p = pending.get(data.id)
      if (!p) return
      pending.delete(data.id)
      if (data.ok) p.resolve(data.result)
      else p.reject(new Error(data.error))
    }
    worker.onerror = (e) => {
      for (const p of pending.values()) p.reject(new Error(e.message || 'drawing worker failed'))
      pending.clear()
      worker = null
    }
  }
  return worker
}

/**
 * Draw with multiple batch passes, in a Web Worker (threaded when the page is
 * cross-origin isolated). `elev` is transferred to the worker: don't reuse it.
 * @param {Float32Array} elev
 * @param {number} width
 * @param {number} height
 * @param {number} pxM
 * @param {Array<{angle:number,style:Record<string,number>}>} batches
 * @param {{ nw_lon: number, nw_lat: number, deg_per_px_lon: number, deg_per_px_lat: number }} [geo]
 * @param {number | null} [mmPerPx] size the SVG in millimetres, for paper
 * @param {{ gpu?: boolean, water?: { spec: string, bounds: number[], tiles: Array<[number, number, number, Uint8Array]> } | null }} [opts]
 *   gpu: terrain fields on WebGPU, when available; water: the water part of the document (JSON), the grid's
 *   bounds [west, south, east, north] and the OpenMapTiles tiles covering them
 * @returns {Promise<{ svg: string, geojson: string, lines: number, dashes: number, ms: number, threads: number,
 *   gpu: string | false | null, timings: Array<[string, number]>, sent: number }>} gpu: adapter used, false if
 *   asked for but unavailable, null if not asked; timings: ms per phase in the worker; sent: epoch ms at postMessage
 */
export function drawBatches(elev, width, height, pxM, batches, geo, mmPerPx, opts) {
  const list = Array.isArray(batches) ? batches : []
  const json = JSON.stringify(list.map((b) => ({
    ...(typeof b?.name === 'string' && b.name.trim() ? { name: b.name } : {}),
    ...(typeof b?.layer === 'string' && b.layer.trim() ? { layer: b.layer } : {}),
    ...(typeof b?.color === 'string' && b.color.trim() ? { color: b.color } : {}),
    ...(typeof b?.color_expr === 'string' && b.color_expr.trim() ? { color_expr: b.color_expr } : {}),
    angle: Number.isFinite(Number(b?.angle)) ? Number(b.angle) : 0,
    style: Object.fromEntries(
      Object.entries(b?.style ?? {}).filter(
        ([, v]) => v != null && typeof v === 'number' && Number.isFinite(v),
      ),
    ),
  })))
  const id = ++nextId
  return new Promise((resolve, reject) => {
    pending.set(id, { resolve, reject })
    const gpu = !!opts?.gpu
    const water = opts?.water ?? null
    drawWorker().postMessage({ id, elev, width, height, pxM, json, geo, mmPerPx, gpu, water }, [elev.buffer])
  })
}
