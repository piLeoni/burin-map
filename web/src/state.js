import { atom } from 'jotai'
import { atomWithStorage } from 'jotai/utils'

/** Map camera — persisted in localStorage. */
export const mapViewAtom = atomWithStorage('burin-map.map', {
  center: /** @type {[number, number]} */ ([-122.45, 37.75]),
  zoom: 12,
})

/** Batch passes — persisted in localStorage as JSON. */
export const styleAtom = atomWithStorage('burin-map.style.v3', /** @type {Array<{ angle: number, style: Record<string, number> }>} */ ([]))

/**
 * Flat style from the first batch, for the WASM draw() API.
 */
export function firstStyle(batches) {
  if (!batches || batches.length === 0) return {}
  return batches[0].style || {}
}

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
