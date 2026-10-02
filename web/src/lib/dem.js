/**
 * Terrain for a lon/lat box, as a Float32 elevation grid. The engine
 * (terramask) says which Terrarium tiles the grid needs (`terrain_tiles`)
 * and turns them into metres (`elevation_grid`), as the command line does;
 * this file fetches and keeps them.
 */

import { loadBurin } from './burin'
import { fetchTile, limiter } from './net'

const TERRARIUM = '/terrarium'
const R = 6378137
/** Zoom out until the view needs no more tiles than this. */
const MAX_TILES = 64
const CACHE_MAX = 512

/** @type {Map<string, Uint8Array>} */
const tileCache = new Map()
/** @type {Map<string, Promise<Uint8Array>>} */
const inflight = new Map()

function remember(key, bytes) {
  // LRU: re-insert so newest keys sit at the end.
  if (tileCache.has(key)) tileCache.delete(key)
  tileCache.set(key, bytes)
  while (tileCache.size > CACHE_MAX) tileCache.delete(tileCache.keys().next().value)
}

/** Across the 180° meridian x runs past the last column; the URL wants it wrapped. */
function keyOf(z, x, y) {
  const n = 2 ** z
  return `${z}/${((x % n) + n) % n}/${y}`
}

const run = limiter(8)

/** One tile's PNG; empty where Terrarium has none. Failures are thrown, not kept. */
async function tile(z, x, y, progress) {
  const key = keyOf(z, x, y)
  if (tileCache.has(key)) {
    const bytes = tileCache.get(key)
    remember(key, bytes)
    return bytes
  }
  if (!inflight.has(key)) {
    const onRetry = (why, n, ms) => progress?.log(`elevation ${key}: ${why}, retry ${n} in ${ms / 1000} s`, 'warn')
    inflight.set(key, fetchTile(`${TERRARIUM}/${key}.png`, { run, onRetry })
      .then(async (r) => (r ? new Uint8Array(await r.arrayBuffer()) : new Uint8Array(0)))
      .finally(() => inflight.delete(key)))
  }
  const bytes = await inflight.get(key)
  remember(key, bytes)
  return bytes
}

/**
 * @param {{ west: number, south: number, east: number, north: number }} bounds degrees
 * @param {number} outW
 * @param {number} outH
 * @param {ReturnType<import('./progress.js').createProgress>} [progress]
 */
export async function buildDem(bounds, outW, outH, progress) {
  const { west, south, north } = bounds
  let { east } = bounds
  if (east <= west) east += 360
  const lat = (south + north) / 2
  // Ground metres per pixel at the centre, as the command line measures them.
  const pxM = ((R * (east - west) * Math.PI) / 180 / outW) * Math.cos((lat * Math.PI) / 180)
  try {
    const m = await loadBurin()
    const ids = m.terrain_tiles(west, south, east, north, outW, MAX_TILES)
    const list = []
    for (let i = 0; i < ids.length; i += 3) list.push([ids[i], ids[i + 1], ids[i + 2]])
    const z = list[0]?.[0] ?? 0
    let done = 0
    progress?.tile(0, list.length)
    const got = await Promise.all(list.map(([tz, x, y]) => tile(tz, x, y, progress).then((b) => {
      progress?.tile(++done, list.length)
      return b
    })))
    const elev = m.elevation_grid(list.map(([tz, x, y], i) => [tz, x, y, got[i]]), west, south, east, north, outW, outH)
    progress?.elevation(elev, list.length, z, outW, outH, pxM)
    return { elev, width: outW, height: outH, pxM, z, tiles: list.length }
  } catch (e) {
    throw new Error(`elevation: ${e?.message || e}`)
  }
}
