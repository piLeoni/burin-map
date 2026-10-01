/**
 * What a land batch draws and on which ground: lines, dots, or a fill kept
 * to a band of heights or depths (`where`).
 */

export const LAND_DRAWS = ['lines', 'dots', 'fill']

export const FILL_WHERE_KEYS = ['elev_min', 'elev_max', 'depth_min', 'depth_max']

/** `ground`, `draw` and `where` as the drawing takes them, defaults left out. */
export function pickLand(b) {
  const where = b?.draw === 'fill' ? cleanWhere(b.where) : null
  return {
    ...(b?.ground === 'sea' ? { ground: 'sea' } : {}),
    ...(b?.draw === 'dots' || b?.draw === 'fill' ? { draw: b.draw } : {}),
    ...(where ? { where } : {}),
  }
}

function cleanWhere(w) {
  if (!w || typeof w !== 'object') return null
  const out = {}
  for (const k of FILL_WHERE_KEYS) {
    const v = Number(w[k])
    if (w[k] != null && Number.isFinite(v)) out[k] = v
  }
  return Object.keys(out).length ? out : null
}

/** A fill's `where` as written; throws on what it can't take. */
export function readFillWhere(w, which) {
  if (w == null) return null
  if (typeof w !== 'object' || Array.isArray(w)) {
    throw new Error(`where in ${which} must be a block with ${FILL_WHERE_KEYS.join(', ')}`)
  }
  for (const k of Object.keys(w)) {
    if (!FILL_WHERE_KEYS.includes(k)) {
      throw new Error(`Unknown setting "${k}" in ${which}.where: use ${FILL_WHERE_KEYS.join(', ')}`)
    }
    if (w[k] != null && !Number.isFinite(Number(w[k]))) {
      throw new Error(`${k} in ${which} must be a number (metres)`)
    }
  }
  return cleanWhere(w)
}
