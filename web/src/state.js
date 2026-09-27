import { atom } from 'jotai'
import { atomWithStorage } from 'jotai/utils'

/** Map camera — persisted in localStorage. */
export const mapViewAtom = atomWithStorage('burin-map.map', {
  center: /** @type {[number, number]} */ ([-122.45, 37.75]),
  zoom: 12,
})

/**
 * Batch list, normalised to always be an array of `{ angle, style }`.
 *
 * Older builds stored a flat `Record<string, number>` under other keys, so we
 * accept anything and coerce it. Never let a non-array reach the drawing code.
 *
 * @param {unknown} value
 * @returns {Array<{ angle: number, style: Record<string, number> }>}
 */
export function normalizeBatches(value) {
  if (Array.isArray(value)) {
    return value
      .filter((b) => b && typeof b === 'object')
      .map((b) => ({
        angle: Number.isFinite(Number(b.angle)) ? Number(b.angle) : 0,
        style: b.style && typeof b.style === 'object' && !Array.isArray(b.style) ? b.style : {},
      }))
      .filter((b) => Object.keys(b.style).length > 0)
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

/** Batch passes — persisted in localStorage as JSON. */
const rawBatchesAtom = atomWithStorage('burin-map.batches.v1', [])

/** Public batches atom: read is always a normalised array; write accepts either. */
export const styleAtom = atom(
  (get) => normalizeBatches(get(rawBatchesAtom)),
  (get, set, next) => {
    const value = typeof next === 'function' ? next(normalizeBatches(get(rawBatchesAtom))) : next
    set(rawBatchesAtom, normalizeBatches(value))
  },
)

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
export const statusAtom = atom({ phase: 'loading', detail: 'Starting…' })

/** Latest drawing + stats (not persisted). */
export const drawingAtom = atom(
  /** @type {{ svg: string, lines: number, dashes: number, ms: number } | null} */ (null),
)
