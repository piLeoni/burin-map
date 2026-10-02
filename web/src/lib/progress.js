/**
 * A draw's progress, for the loading panel: the engine's own panel
 * (burin-map-wasm `Panel`, the one the command line shows), told what the
 * page and the worker do, with the seconds since the draw began. What it
 * says to show goes to `onChange` once a frame, until the draw ends.
 */

/**
 * @typedef {{ state: 'wait' | 'run' | 'ok' | 'fail', mark: string, name: string, child: boolean,
 *   bar: [string, string, string] | null, detail: string }} Line
 * @typedef {{ at: number, text: string, warn: boolean }} Message
 * @typedef {{ head: string, title: string, rows: Line[], name_width: number, bar_width: number,
 *   messages: Message[], earlier: number }} View
 */

/**
 * @param {any} panel a `Panel` from the engine; freed when the draw ends
 * @param {(view: View) => void} onChange
 */
export function createProgress(panel, onChange) {
  const t0 = performance.now()
  const now = () => (performance.now() - t0) / 1000
  let live = true
  const frame = () => {
    if (!panel) return
    onChange(JSON.parse(panel.view(now())))
    if (live) requestAnimationFrame(frame)
    else {
      panel.free()
      panel = null
    }
  }
  requestAnimationFrame(frame)
  const tell = (f) => (...args) => {
    if (live) f(...args)
  }
  return {
    /** An elevation tile in; `done` of `total`. */
    tile: tell((done, total) => panel.tile(done, total, now())),
    /** The elevation grid, from `tiles` tiles at zoom `z`. */
    elevation: tell((elev, tiles, z, width, height, pxM) => panel.elevation(elev, tiles, z, width, height, pxM, now())),
    /** A water tile in; `done` of `total`. */
    waterTile: tell((done, total) => panel.waterTile(done, total, now())),
    /** Every water tile in. */
    water: tell(() => panel.water(now())),
    /** The area drawn, in degrees. */
    area: tell((b) => panel.area(b.west, b.south, b.east, b.north, now())),
    /** The drawing holds `count` dashes. */
    dashes: tell((count) => panel.dashes(count, now())),
    /** The engine's board slots to watch while the draw runs. */
    slots: () => (panel ? Array.from(panel.slots()) : []),
    /** What the board says in `slot`. */
    board: tell((slot, value) => panel.board(slot, value, now())),
    /** A line for the messages; `warn` and `error` stand out. */
    log: tell((text, level = 'info') => panel.message(text, level !== 'info', now())),
    /** The draw over, done or failed: one last frame shows how it ended. */
    finish: tell((ok) => {
      panel.finish(ok, now())
      live = false
    }),
    /** The draw dropped for a newer one: nothing more to show. */
    close() {
      live = false
    },
  }
}
