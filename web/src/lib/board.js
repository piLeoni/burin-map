/**
 * The engine's progress board (crates/burin-map-wasm/src/board.rs): one
 * slot per step of a draw, holding how far it has got, in memory the worker
 * shares with the page. Read every frame into rows of the loading panel.
 */

const DRAWS = 0
const WATER_MASK = 1
const TERRAIN = 2
const SHORE = 3
const SHORELINE = 4
const ORDER = 5
const SVG = 6
const MAX_BATCHES = 120
const LAND = 16
const WATER = LAND + MAX_BATCHES

/** Stages, as in burin_map_core::Stage. */
const DONE = 5
const LINE_STAGES = ['', 'fields', 'trace', 'order', 'dashes']

const kindOf = (b) => b?.draw === 'fill' ? 'fill' : b?.draw === 'dots' ? 'dots' : b?.draw === 'ripples' ? 'ripples'
  : b?.draw === 'waves' ? 'waves' : 'lines'

/** What stage `stage` is called for a batch drawing `kind`. */
function stageName(kind, stage) {
  if (stage === 2 && kind === 'fill') return 'rings'
  if (stage === 2 && kind !== 'lines') return kind
  return LINE_STAGES[stage] ?? ''
}

const name = (b, i, kind) => {
  const n = typeof b?.name === 'string' && b.name.trim() ? b.name.trim() : kind
  return `${i + 1} ${n}`.slice(0, 16)
}

/**
 * Follow `board` (from the draw numbered `draws`) into child rows of
 * `progress`, one per step, until the returned function is called.
 * @param {Uint32Array} board
 * @param {number} draws
 * @param {ReturnType<import('./progress.js').createProgress>} progress
 * @param {{ batches: object[], water: { batches?: object[], shoreline?: object } | null }} doc
 */
export function watchBoard(board, draws, progress, { batches, water }) {
  const rows = []
  const row = (slot, label, kind) => {
    const task = progress.task(`board-${slot}`, label, kind ? DONE : 0, '', { child: true, wait: true })
    rows.push({ slot, task, kind, stage: 0 })
  }
  if (water) row(WATER_MASK, 'water mask')
  row(TERRAIN, 'terrain')
  batches.slice(0, MAX_BATCHES).forEach((b, i) => row(LAND + i, name(b, i, kindOf(b)), kindOf(b)))
  if (water) {
    row(SHORE, 'shore')
    ;(water.batches ?? []).slice(0, MAX_BATCHES).forEach((b, i) => row(WATER + i, name(b, i, `sea ${kindOf(b)}`), kindOf(b)))
    if (water.shoreline) row(SHORELINE, 'shoreline')
  }
  row(ORDER, 'plot order')
  row(SVG, 'svg')

  let on = true
  const read = () => {
    if (!on) return
    if (Atomics.load(board, DRAWS) === draws) {
      for (const r of rows) {
        const stage = Atomics.load(board, r.slot)
        if (stage === r.stage) continue
        r.stage = stage
        if (stage >= DONE) r.task.ok()
        else if (stage > 0) r.task.step(r.kind ? stage : 0, r.kind ? stageName(r.kind, stage) : '')
      }
    }
    requestAnimationFrame(read)
  }
  read()
  return () => { on = false }
}
