import { atom } from 'jotai'
import { atomWithStorage } from 'jotai/utils'
import { DEFAULT_FRAME, pair } from './lib/frame.js'

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

const EDITOR_STORAGE_KEY = 'burin-map.editor.v1'
const APPLIED_STORAGE_KEY = 'burin-map.applied.v1'
const LEGACY_SCRIPT_KEY = 'burin-map.script.v3'

/** @returns {string | null} */
function readStoredString(key) {
  if (typeof localStorage === 'undefined') return null
  try {
    const raw = localStorage.getItem(key)
    if (!raw) return null
    const v = JSON.parse(raw)
    return typeof v === 'string' ? v : null
  } catch {
    return null
  }
}

/**
 * What is in localStorage for the document editor and the last apply, read
 * synchronously so startup does not flash the default preset over a saved doc.
 */
export function loadPersistedDocument() {
  let editor = readStoredString(EDITOR_STORAGE_KEY)
  let applied = readStoredString(APPLIED_STORAGE_KEY)
  if (editor == null) {
    const legacy = readStoredString(LEGACY_SCRIPT_KEY)
    if (legacy != null) {
      editor = legacy
      applied = legacy
    }
  }
  if (applied == null) applied = editor
  return { editor: editor ?? '', applied: applied ?? '' }
}

const bootDocument = typeof window !== 'undefined'
  ? loadPersistedDocument()
  : { editor: '', applied: '' }

/**
 * The script in the Monaco editor — persisted across reloads (every keystroke).
 * Choosing a preset again replaces this with that preset's original file.
 */
export const editorAtom = atomWithStorage(EDITOR_STORAGE_KEY, bootDocument.editor)

/** The script the map last drew successfully; revert returns here. */
export const appliedScriptAtom = atomWithStorage(APPLIED_STORAGE_KEY, bootDocument.applied)

/** @deprecated use editorAtom */
export const scriptAtom = editorAtom

/** The atlas the current script last built (not persisted). */
export const atlasAtom = atom(/** @type {Array<object> | null} */ (null))

/** The area the script asked for, from `burin.area({…})` (not persisted). */
export const areaAtom = atom(/** @type {object | null} */ (null))

/** Where terrain fields are built. Not a user choice: the browser uses WebGPU
 * when it has it, the CPU when it does not, and the drawing comes out the same
 * shape either way. `?force-cpu` gives reproducible bytes for a bug report. */
export const forceCpu = typeof window !== 'undefined'
  && new URLSearchParams(window.location.search).has('force-cpu')

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
