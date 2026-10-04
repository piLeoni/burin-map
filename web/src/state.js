import { atom } from 'jotai'
import { atomWithStorage } from 'jotai/utils'
import { DEFAULT_FRAME, pair } from './lib/frame.js'
import { pickLand } from './lib/fill.js'
import { pickInk } from './lib/ink.js'

/** Map camera — persisted in localStorage. The centre is the frame's centre. */
export const mapViewAtom = atomWithStorage('burin-map.map', {
  center: /** @type {[number, number]} */ ([-122.45, 37.75]),
  zoom: 12,
})

/**
 * A request to recentre the map (from a link or an imported document). The
 * map reads its camera once at startup, so later moves have to be asked for.
 * The zoom follows from the frame.
 */
export const cameraRequestAtom = atom(
  /** @type {{ center: [number, number] } | null} */ (null),
)

/**
 * Batch list, normalised to always be an array of `{ angle, style }`. `style`
 * holds only the parameters that are set; the engine uses its defaults for
 * the rest.
 *
 * Older builds stored a flat `Record<string, number>` under other keys, so we
 * accept anything and coerce it. Never let a non-array reach the drawing code.
 *
 * @param {unknown} value
 * @returns {Array<{ name?: string, angle: number, style: Record<string, number> }>}
 */
export function normalizeBatches(value) {
  if (Array.isArray(value)) {
    return value
      .filter((b) => b && typeof b === 'object')
      .map((b) => ({
        ...(typeof b.name === 'string' && b.name.trim() ? { name: b.name } : {}),
        ...pickInk(b),
        angle: Number.isFinite(Number(b.angle)) ? Number(b.angle) : 0,
        ...pickLand(b),
        style: b.style && typeof b.style === 'object' && !Array.isArray(b.style) ? b.style : {},
      }))
  }
  // Legacy: a flat style object becomes a single batch.
  if (value && typeof value === 'object') {
    const style = /** @type {Record<string, number>} */ (value)
    const numeric = Object.fromEntries(
      Object.entries(style).filter(([, v]) => typeof v === 'number' && Number.isFinite(v)),
    )
    if (Object.keys(numeric).length > 0) return [{ angle: 0, style: numeric }]
  }
  return []
}

/** Batch passes — persisted in localStorage as JSON. v1 stored every parameter, set or not. */
export const BATCHES_V1_KEY = 'burin-map.batches.v1'
const rawBatchesAtom = atomWithStorage('burin-map.batches.v2', [])

/** Public batches atom: read is always a normalised array; write accepts either. */
export const styleAtom = atom(
  (get) => normalizeBatches(get(rawBatchesAtom)),
  (get, set, next) => {
    const value = typeof next === 'function' ? next(normalizeBatches(get(rawBatchesAtom))) : next
    set(rawBatchesAtom, normalizeBatches(value))
  },
)

const WAVE_MARKS = ['ripples', 'dots']

/**
 * The document's water block, or null for none: `{ classes?, shoreline?, source?,
 * batches: [{ name?, follow, draw?, angle, style }] }`, `draw` only when it is
 * wave marks. Shaped by `normalizeWater`,
 * which also takes the older name `shore` still in saved documents.
 * @param {unknown} value
 */
export function normalizeWater(value) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  const v = /** @type {Record<string, any>} */ (value)
  const out = {}
  if (Array.isArray(v.classes)) out.classes = v.classes.map(String)
  const shoreline = v.shoreline ?? v.shore
  if (shoreline === false) out.shoreline = false
  else if (shoreline && typeof shoreline === 'object' && !Array.isArray(shoreline)) {
    const lines = Number(shoreline.lines)
    const style = shoreline.style
    out.shoreline = {
      lines: Number.isInteger(lines) && lines >= 1 ? lines : 1,
      ...pickInk(shoreline),
      style: style && typeof style === 'object' && !Array.isArray(style) ? style : {},
    }
  }
  if (typeof v.source === 'string' && v.source) out.source = v.source
  out.batches = (Array.isArray(v.batches) ? v.batches : [])
    .filter((b) => b && typeof b === 'object')
    .map((b) => ({
      ...(typeof b.name === 'string' && b.name.trim() ? { name: b.name } : {}),
      ...pickInk(b),
      follow: b.follow === 'map' ? 'map' : 'shore',
      ...(WAVE_MARKS.includes(b.draw) ? { draw: b.draw } : {}),
      angle: Number.isFinite(Number(b.angle)) ? Number(b.angle) : 0,
      ...(readWaterWhere(b.where) ? { where: readWaterWhere(b.where) } : {}),
      style: b.style && typeof b.style === 'object' && !Array.isArray(b.style) ? b.style : {},
    }))
  return out
}

/** @param {unknown} w */
function readWaterWhere(w) {
  if (!w || typeof w !== 'object' || Array.isArray(w)) return null
  const out = {}
  const min = Number(/** @type {Record<string, unknown>} */ (w).depth_min)
  const max = Number(/** @type {Record<string, unknown>} */ (w).depth_max)
  if (Number.isFinite(min)) out.depth_min = min
  if (Number.isFinite(max)) out.depth_max = max
  const fade = Number(w.depth_fade)
  if (Number.isFinite(fade) && fade > 0) out.depth_fade = fade
  return Object.keys(out).length ? out : null
}

/** @param {unknown} value */
export function normalizeBathymetry(value) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) return null
  const v = /** @type {Record<string, unknown>} */ (value)
  const out = {}
  if (typeof v.source === 'string' && v.source.trim()) out.source = v.source.trim()
  if (typeof v.url === 'string' && v.url.trim()) out.url = v.url.trim()
  return Object.keys(out).length ? out : null
}

const rawWaterAtom = atomWithStorage('burin-map.water.v1', null)

/** Water: read is always normalised (or null); write accepts either. */
export const waterAtom = atom(
  (get) => normalizeWater(get(rawWaterAtom)),
  (get, set, next) => {
    const value = typeof next === 'function' ? next(normalizeWater(get(rawWaterAtom))) : next
    set(rawWaterAtom, normalizeWater(value))
  },
)

const rawBathymetryAtom = atomWithStorage('burin-map.bathymetry.v1', null)

export const bathymetryAtom = atom(
  (get) => normalizeBathymetry(get(rawBathymetryAtom)),
  (get, set, next) => {
    const value = typeof next === 'function' ? next(normalizeBathymetry(get(rawBathymetryAtom))) : next
    set(rawBathymetryAtom, normalizeBathymetry(value))
  },
)

/** The user's comments in the editor, by path (see lib/views.js). */
export const notesAtom = atomWithStorage('burin-map.notes.v1', {})

/**
 * Where terrain fields are built. Not a user choice: the browser uses WebGPU
 * when it has it, the CPU when it does not, and the drawing comes out the same
 * shape either way. `?force-cpu` gives reproducible bytes for a bug report.
 */
export const forceCpu = typeof window !== 'undefined'
  && new URLSearchParams(window.location.search).has('force-cpu')

/** Which way the editor writes the document: 'yaml', 'json5', 'cli' or 'js'. */
export const viewKindAtom = atomWithStorage('burin-map.view', 'yaml')

/** How long a successful unlock stays valid (30 days). */
export const UNLOCK_TTL_MS = 30 * 24 * 60 * 60 * 1000

/** Password unlock — persisted until `expiresAt`. */
export const unlockAtom = atomWithStorage(
  'burin-map.unlock',
  /** @type {{ expiresAt: number } | null} */ (null),
)

export function isUnlockValid(unlock) {
  return !!(unlock && typeof unlock.expiresAt === 'number' && unlock.expiresAt > Date.now())
}

/** Ephemeral UI status (not persisted). */
export const statusAtom = atom({ phase: 'loading' })

/** Latest drawing + stats (not persisted). */
export const drawingAtom = atom(
  /** @type {{ svg: string, geojson?: string, lines: number, dashes: number, ms: number, capped?: { width: number, height: number } | null } | null} */ (null),
)

/**
 * Coerce whatever is stored into a valid frame.
 * @returns {import('./lib/frame.js').Frame}
 */
export function normalizeFrame(value) {
  const v = value && typeof value === 'object' ? value : {}
  return {
    mode: v.mode === 'paper' ? 'paper' : 'land',
    size_km: pair(v.size_km),
    size_mm: pair(v.size_mm) ?? DEFAULT_FRAME.size_mm,
    scale: Number(v.scale) > 0 ? Number(v.scale) : DEFAULT_FRAME.scale,
    pen_width: Number(v.pen_width) > 0 ? Number(v.pen_width) : DEFAULT_FRAME.pen_width,
  }
}

const rawFrameAtom = atomWithStorage('burin-map.frame.v2', DEFAULT_FRAME)

/** The frame: ground or paper. Persisted in localStorage. */
export const frameAtom = atom(
  (get) => normalizeFrame(get(rawFrameAtom)),
  (get, set, next) => {
    const value = typeof next === 'function' ? next(normalizeFrame(get(rawFrameAtom))) : next
    set(rawFrameAtom, normalizeFrame(value))
  },
)
