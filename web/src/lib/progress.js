/**
 * What a draw is doing, for the loading panel: tasks with a count or a
 * time to go, and a short log of what went wrong on the way.
 */

const LOG_MAX = 40

/**
 * @typedef {{ id: string, label: string, done: number, total: number, unit: string,
 *   state: 'wait' | 'run' | 'ok' | 'fail', note: string, t0: number, ms: number, eta: number,
 *   child: boolean }} Task
 * @typedef {{ text: string, level: 'info' | 'warn' | 'error' }} LogLine
 */

/** A tracker that hands `onChange` a fresh `{ tasks, log }` at most once a frame. */
export function createProgress(onChange) {
  /** @type {Task[]} */
  const tasks = []
  /** @type {LogLine[]} */
  const log = []
  let queued = false
  const emit = () => {
    if (queued) return
    queued = true
    requestAnimationFrame(() => {
      queued = false
      onChange({ tasks: tasks.map((t) => ({ ...t })), log: [...log] })
    })
  }
  return {
    /**
     * A task of `total` steps (0: counted in time instead), shown as `label`;
     * `child`: part of the task above it; `wait`: not begun until its first step.
     */
    task(id, label, total = 0, unit = '', { child = false, wait = false } = {}) {
      const t = {
        id, label, done: 0, total, unit, state: wait ? 'wait' : 'run', note: '',
        t0: performance.now(), ms: 0, eta: 0, child,
      }
      tasks.push(t)
      emit()
      const begin = () => {
        if (t.state !== 'wait') return
        t.state = 'run'
        t.t0 = performance.now()
      }
      const end = (state, note) => {
        begin()
        t.state = state
        t.ms = performance.now() - t.t0
        if (state === 'ok' && t.total) t.done = t.total
        t.note = note ?? (state === 'ok' && t.child ? '' : t.note)
        emit()
      }
      return {
        setTotal(n) {
          t.total = n
          emit()
        },
        tick(n = 1) {
          t.done += n
          emit()
        },
        /** `done` steps so far, and what it is doing now. */
        step(done, note = '') {
          begin()
          t.done = done
          t.note = note
          emit()
        },
        /** How long it should take, ms, when it has no steps to count. */
        eta(ms) {
          t.eta = ms
          emit()
        },
        ok: (note) => end('ok', note),
        fail: (note) => end('fail', note),
      }
    },
    log(text, level = 'info') {
      log.push({ text, level })
      if (log.length > LOG_MAX) log.shift()
      emit()
    },
  }
}
