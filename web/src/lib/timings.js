/**
 * Where a draw spends its time, printed to the console as one block of plain
 * text that can be copied whole. On in development, and anywhere with
 * `?timings` in the URL.
 */

export const timingsOn = import.meta.env.DEV
  || (typeof window !== 'undefined' && new URLSearchParams(window.location.search).has('timings'))

/** @param {string} title @param {Array<[string, number]>} rows @param {string[]} [notes] */
export function logTimings(title, rows, notes = []) {
  if (!timingsOn) return
  const width = Math.max(...rows.map(([label]) => label.length))
  const lines = rows.map(([label, ms]) => `  ${label.padEnd(width)}  ${ms.toFixed(1).padStart(7)} ms`)
  console.log([title, ...lines, ...notes.map((n) => `  ${n}`)].join('\n'))
}
