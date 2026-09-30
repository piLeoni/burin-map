/** When bathymetry needs a network fetch vs reusing the elevation mosaic. */

function exprUsesDepth(expr) {
  if (typeof expr !== 'string' || !expr.trim()) return false
  return expr.split(/[^A-Za-z0-9_]+/).includes('depth')
}

/** True if any water batch or shoreline colour needs a depth grid. */
export function waterSpecNeedsDepth(spec) {
  if (!spec || typeof spec !== 'object') return false
  const batches = Array.isArray(spec.batches) ? spec.batches : []
  for (const b of batches) {
    if (!b || typeof b !== 'object') continue
    const w = b.where
    if (w && (w.depth_min != null || w.depth_max != null)) return true
    if (exprUsesDepth(b.color_expr)) return true
  }
  const sh = spec.shoreline
  if (sh && typeof sh === 'object' && exprUsesDepth(sh.color_expr)) return true
  return false
}

/** Separate tile fetch (GEBCO, custom URL). Terrarium/auto reuse elev — no extra download. */
export function bathymetryNeedsNetworkFetch(bathymetry) {
  const raw = bathymetry?.source ?? bathymetry?.url ?? ''
  const s = String(raw).trim().toLowerCase()
  if (!s || s === 'terrarium' || s === 'auto' || s === 'dem') return false
  if (s === 'gebco' || s === 'url') return true
  return s.includes('{z}') || s.startsWith('http://') || s.startsWith('https://')
}

/**
 * Optional dedicated bathymetry grid (metres below surface). Returns null when
 * the engine should derive depth from elevation instead.
 */
export async function fetchBathymetryGrid(bounds, outW, outH, bathymetry, onProgress) {
  if (!bathymetryNeedsNetworkFetch(bathymetry)) return null
  const src = String(bathymetry?.source ?? 'gebco').trim().toLowerCase()
  onProgress?.(
    src === 'gebco'
      ? 'GEBCO bathymetry is not wired yet — use bathymetry.source: terrarium'
      : 'Custom bathymetry URL is not wired yet — use terrarium or wait for GEBCO',
  )
  return null
}
