/**
 * Water tiles from OpenFreeMap (or another OpenMapTiles-schema source), for
 * the engine's water mask. The engine says which tiles a grid needs
 * (`water_tiles`); this file fetches and keeps them.
 */

import { loadBurin } from './burin'

const OPENFREEMAP = 'https://tiles.openfreemap.org/planet'
/** Zoom out until the view needs no more tiles than this. */
const MAX_TILES = 64
const CACHE_MAX = 256

/** @type {Map<string, Promise<{ template: string, maxzoom: number }>>} */
const sources = new Map()
/** @type {Map<string, Uint8Array>} */
const tileCache = new Map()
/** @type {Map<string, Promise<Uint8Array>>} */
const inflight = new Map()

/** A TileJSON URL, or a `{z}/{x}/{y}` template, resolved to a template and deepest zoom. */
function resolve(source) {
  if (source.includes('{z}')) return Promise.resolve({ template: source, maxzoom: 14 })
  if (!sources.has(source)) {
    const p = fetch(source)
      .then((r) => {
        if (!r.ok) throw new Error(`water source ${source}: ${r.status}`)
        return r.json()
      })
      .then((tj) => {
        if (!tj?.tiles?.[0]) throw new Error(`water source ${source}: no tile URL in the TileJSON`)
        return { template: tj.tiles[0], maxzoom: Math.min(14, tj.maxzoom ?? 14) }
      })
    // A failure should be retried next time, not remembered.
    p.catch(() => sources.delete(source))
    sources.set(source, p)
  }
  return sources.get(source)
}

function remember(key, bytes) {
  if (tileCache.has(key)) tileCache.delete(key)
  tileCache.set(key, bytes)
  while (tileCache.size > CACHE_MAX) tileCache.delete(tileCache.keys().next().value)
}

/** Across the 180° meridian x runs past the last column; the URL wants it wrapped. */
function urlOf(template, z, x, y) {
  const n = 2 ** z
  return template.replace('{z}', z).replace('{x}', ((x % n) + n) % n).replace('{y}', y)
}

/** One tile's bytes; empty where the source has none. */
async function tile(template, z, x, y) {
  const url = urlOf(template, z, x, y)
  if (tileCache.has(url)) {
    const bytes = tileCache.get(url)
    remember(url, bytes)
    return bytes
  }
  if (!inflight.has(url)) {
    inflight.set(url, fetch(url)
      .then(async (r) => (r.ok ? new Uint8Array(await r.arrayBuffer()) : new Uint8Array(0)))
      .finally(() => inflight.delete(url)))
  }
  const bytes = await inflight.get(url)
  remember(url, bytes)
  return bytes
}

/**
 * The tiles a north-up Web Mercator grid `width` px across `bounds` needs, as
 * `[z, x, y, bytes]` for the engine.
 * @param {{ west: number, south: number, east: number, north: number }} bounds
 * @param {number} width
 * @param {string} [source]
 * @param {(msg: string) => void} [onProgress]
 */
export async function waterTiles(bounds, width, source, onProgress) {
  const [m, src] = await Promise.all([loadBurin(), resolve(source || OPENFREEMAP)])
  const ids = m.water_tiles(bounds.west, bounds.south, bounds.east, bounds.north, width, src.maxzoom, MAX_TILES)
  const list = []
  for (let i = 0; i < ids.length; i += 3) list.push([ids[i], ids[i + 1], ids[i + 2]])
  const missing = list.filter(([z, x, y]) => !tileCache.has(urlOf(src.template, z, x, y))).length
  if (missing > 0) onProgress?.(`Downloading water (${missing} tile${missing === 1 ? '' : 's'})…`)
  const got = await Promise.all(list.map(([z, x, y]) => tile(src.template, z, x, y)))
  return list.map(([z, x, y], i) => [z, x, y, got[i]])
}
