/**
 * Tile downloads: a few at a time, failures tried again, and never taken for
 * an empty tile — an empty elevation tile is flat ground at 0 m, an empty
 * water tile dry land, and either leaves a block in the drawing.
 */

/** Runs at most `n` jobs at once; the rest wait their turn. */
export function limiter(n) {
  let running = 0
  /** @type {Array<() => void>} */
  const queue = []
  return async function run(job) {
    if (running >= n) await new Promise((go) => queue.push(go))
    running++
    try {
      return await job()
    } finally {
      running--
      queue.shift()?.()
    }
  }
}

const RETRY_MS = [500, 1500, 4000]
const wait = (ms) => new Promise((r) => setTimeout(r, ms))

/**
 * The response for `url`, or null where the source says it has nothing
 * (204, 404). Other statuses and network errors are tried again after each
 * of `RETRY_MS`, then thrown. `onRetry(reason, attempt, ms)` hears of each.
 * @param {string} url
 * @param {{ run?: (job: () => Promise<Response>) => Promise<Response>, onRetry?: (reason: string, attempt: number, ms: number) => void }} [opts]
 */
export async function fetchTile(url, { run = (job) => job(), onRetry } = {}) {
  for (let attempt = 0; ; attempt++) {
    let reason
    try {
      const r = await run(() => fetch(url))
      if (r.ok) return r
      if (r.status === 204 || r.status === 404) return null
      reason = `HTTP ${r.status}`
    } catch (e) {
      reason = String(e?.message || e)
    }
    if (attempt >= RETRY_MS.length) throw new Error(`${url}: ${reason}`)
    onRetry?.(reason, attempt + 1, RETRY_MS[attempt])
    await wait(RETRY_MS[attempt])
  }
}
