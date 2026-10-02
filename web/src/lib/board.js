/**
 * The engine's progress board (crates/burin-map-wasm/src/board.rs): how far
 * each step of a draw has got, in memory the worker shares with the page.
 * Read every frame and handed to the engine's panel, which says which slots
 * there are and what they mean.
 */

const DRAWS = 0

/**
 * Follow `board` (from the draw numbered `draws`) into `progress` until the
 * returned function is called.
 * @param {Uint32Array} board
 * @param {number} draws
 * @param {ReturnType<import('./progress.js').createProgress>} progress
 */
export function watchBoard(board, draws, progress) {
  const seen = new Map(progress.slots().map((s) => [s, 0]))
  let on = true
  const read = () => {
    if (!on) return
    if (Atomics.load(board, DRAWS) === draws) {
      for (const [slot, was] of seen) {
        const value = Atomics.load(board, slot)
        if (value === was) continue
        seen.set(slot, value)
        progress.board(slot, value)
      }
    }
    requestAnimationFrame(read)
  }
  read()
  return () => {
    on = false
  }
}
