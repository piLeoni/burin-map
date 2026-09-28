/**
 * Draws off the main thread. On a cross-origin isolated page it loads the
 * threaded engine (public/pkg-mt) and starts a rayon pool of Web Workers;
 * anywhere else, or if that fails, the single-threaded one (public/pkg).
 */

// Function so Vite does not rewrite imports of files in /public (as in burin.js).
const importPkg = new Function('url', 'return import(url)')

let engine = null

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

self.onmessage = async ({ data }) => {
  const { id, elev, width, height, pxM, json, geo, mmPerPx } = data
  try {
    const { m, threads } = await load()
    const d = m.draw_batches(elev, width, height, pxM, json, geo ?? null, mmPerPx ?? undefined)
    const result = { svg: d.svg, geojson: d.geojson, lines: d.lines, dashes: d.dashes, ms: d.ms, threads }
    d.free()
    self.postMessage({ id, ok: true, result })
  } catch (e) {
    self.postMessage({ id, ok: false, error: String(e?.message ?? e) })
  }
}
