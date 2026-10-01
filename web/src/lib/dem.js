/** Terrarium DEM mosaic for a lon/lat bounding box → Float32 elevation grid. */

import { fetchTile, limiter } from './net'

const TERRARIUM = '/terrarium'
const TILE = 256
const R = 6378137
const CACHE_MAX = 512

/** @type {Map<string, Float32Array>} */
const tileCache = new Map()
/** @type {Map<string, Promise<Float32Array>>} */
const inflight = new Map()

export function lonLatToMerc(lon, lat) {
  const x = (R * lon * Math.PI) / 180
  const y = R * Math.log(Math.tan(Math.PI / 4 + (lat * Math.PI) / 360))
  return [x, y]
}

export function mercToLonLat(x, y) {
  const lon = (x / R) * (180 / Math.PI)
  const lat = (2 * Math.atan(Math.exp(y / R)) - Math.PI / 2) * (180 / Math.PI)
  return [lon, lat]
}

function lonLatToTile(lon, lat, z) {
  const n = 2 ** z
  const x = ((lon + 180) / 360) * n
  const s = Math.sin((lat * Math.PI) / 180)
  const y = (0.5 - Math.log((1 + s) / (1 - s)) / (4 * Math.PI)) * n
  return [x, y]
}

function chooseZ(lat, metresPerPx) {
  let z = Math.round(
    Math.log2((Math.cos((lat * Math.PI) / 180) * 2 * Math.PI * R) / (TILE * metresPerPx)),
  )
  return Math.max(0, Math.min(14, z))
}

function wrapTileX(x, z) {
  const n = 2 ** z
  return ((x % n) + n) % n
}

function touchCache(key, elev) {
  // LRU: re-insert so newest keys sit at the end.
  if (tileCache.has(key)) tileCache.delete(key)
  tileCache.set(key, elev)
  while (tileCache.size > CACHE_MAX) {
    tileCache.delete(tileCache.keys().next().value)
  }
}

const run = limiter(8)

async function fetchTileElev(z, x, y, progress) {
  const onRetry = (why, n, ms) => progress?.log(`elevation ${z}/${x}/${y}: ${why}, retry ${n} in ${ms / 1000} s`, 'warn')
  const r = await fetchTile(`${TERRARIUM}/${z}/${x}/${y}.png`, { run, onRetry })
  // Terrarium covers the world; a tile it lacks is open sea at sea level.
  if (!r) return new Float32Array(TILE * TILE)
  const bmp = await createImageBitmap(await r.blob())
  const c = new OffscreenCanvas(TILE, TILE)
  const g = c.getContext('2d', { willReadFrequently: true })
  g.drawImage(bmp, 0, 0)
  const { data } = g.getImageData(0, 0, TILE, TILE)
  const elev = new Float32Array(TILE * TILE)
  for (let i = 0, j = 0; i < data.length; i += 4, j++) {
    elev[j] = data[i] * 256 + data[i + 1] + data[i + 2] / 256 - 32768
  }
  return elev
}

/** Below this zoom no tile is patched from its parent. */
const PATCH_MIN_Z = 5

/**
 * Sea the tile has at exactly 0 m where its parent, a zoom level out, has a
 * seabed: from z11 up Terrarium has whole tiles of open sea missing their
 * bathymetry, and contours ring each one. Land, and sea the parent has at
 * 0 m too, stay as they are. True if any pixel changed.
 */
function patchSea(elev, parent, x, y) {
  const ox = (x & 1) * (TILE / 2)
  const oy = (y & 1) * (TILE / 2)
  const at = (px, py) => parent[Math.min(TILE - 1, Math.max(0, py)) * TILE + Math.min(TILE - 1, Math.max(0, px))]
  let changed = false
  for (let j = 0, i = 0; j < TILE; j++) {
    for (let k = 0; k < TILE; k++, i++) {
      if (elev[i] !== 0) continue
      const u = ox + (k + 0.5) / 2 - 0.5
      const v = oy + (j + 0.5) / 2 - 0.5
      const u0 = Math.floor(u)
      const v0 = Math.floor(v)
      const fu = u - u0
      const fv = v - v0
      const e =
        (at(u0, v0) * (1 - fu) + at(u0 + 1, v0) * fu) * (1 - fv) +
        (at(u0, v0 + 1) * (1 - fu) + at(u0 + 1, v0 + 1) * fu) * fv
      if (e < 0) {
        elev[i] = e
        changed = true
      }
    }
  }
  return changed
}

/** A fetched tile with its missing seabed taken from its parent, itself patched in turn. */
async function patchedTile(z, x, y, progress) {
  const elev = await fetchTileElev(z, x, y, progress)
  if (z <= PATCH_MIN_Z || !elev.includes(0)) return { elev, patched: false }
  const { elev: parent } = await tileElev(z - 1, x >> 1, y >> 1, progress)
  return { elev, patched: patchSea(elev, parent, x, y) }
}

/**
 * Cached terrarium tile. Reuses memory + dedupes concurrent requests for the same z/x/y.
 * @returns {Promise<{ elev: Float32Array, cached: boolean, patched?: boolean }>}
 */
async function tileElev(z, x, y, progress) {
  x = wrapTileX(x, z)
  const key = `${z}/${x}/${y}`
  if (tileCache.has(key)) {
    const elev = tileCache.get(key)
    touchCache(key, elev)
    return { elev, cached: true }
  }
  if (inflight.has(key)) {
    const { elev } = await inflight.get(key)
    return { elev, cached: true }
  }
  const p = patchedTile(z, x, y, progress)
  inflight.set(key, p)
  try {
    const { elev, patched } = await p
    touchCache(key, elev)
    return { elev, cached: false, patched }
  } finally {
    inflight.delete(key)
  }
}

/** Column in the mosaic for a tile-pixel x, wrapped across ±180°; -1 when outside. */
function mosaicColumn(x0, nx, z, px) {
  let x = Math.floor(px) - x0 * TILE
  if (x < 0 || x >= nx * TILE) {
    const world = 2 ** z * TILE
    x = ((x % world) + world) % world
    if (x >= nx * TILE) return -1
  }
  return x
}

/** One mosaic pixel; rows and columns are clamped to the mosaic's edge. */
function mosaicAt(tiles, nx, ny, ix, iy) {
  ix = Math.max(0, Math.min(ix, nx * TILE - 1))
  iy = Math.max(0, Math.min(iy, ny * TILE - 1))
  const elev = tiles[Math.floor(iy / TILE) * nx + Math.floor(ix / TILE)]
  return elev ? elev[(iy % TILE) * TILE + (ix % TILE)] : 0
}

/** Catmull-Rom weights for the four samples around fraction t. */
function cubicWeights(t) {
  const t2 = t * t
  const t3 = t2 * t
  return [
    0.5 * (-t3 + 2 * t2 - t),
    0.5 * (3 * t3 - 5 * t2 + 2),
    0.5 * (-3 * t3 + 4 * t2 + t),
    0.5 * (t3 - t2),
  ]
}

/**
 * @param {{ west: number, south: number, east: number, north: number }} bounds degrees
 * @param {number} outW
 * @param {number} outH
 * @param {ReturnType<import('./progress.js').createProgress>} [progress]
 */
export async function buildDem(bounds, outW, outH, progress) {
  let { west: lonW, south: latS, east: lonE, north: latN } = bounds

  // Clamp latitude; keep a single world copy of longitude span.
  latS = Math.max(-85.0511, latS)
  latN = Math.min(85.0511, latN)
  let span = lonE - lonW
  if (span <= 0) span += 360
  if (span > 360) {
    lonW = -180
    lonE = 180
    span = 360
  }

  const [xW, yS] = lonLatToMerc(lonW, latS)
  const [xE, yN] = lonLatToMerc(lonW + span, latN)
  const widthM = xE - xW
  const heightM = yN - yS
  const lat = (latS + latN) / 2
  const pxM = widthM / outW

  // Prefer a z that matches the pixel size, but step down if the mosaic
  // would need too many tiles (wide / zoomed-out views on big screens).
  const MAX_TILES = 64
  let z = chooseZ(lat, pxM)
  let x0, y0, x1, y1, nx, ny
  for (;;) {
    const [tx0f, ty0f] = lonLatToTile(lonW, latN, z)
    const [tx1f, ty1f] = lonLatToTile(lonW + span, latS, z)
    x0 = Math.floor(tx0f)
    y0 = Math.floor(ty0f)
    x1 = Math.floor(tx1f)
    y1 = Math.floor(ty1f)
    nx = Math.max(1, x1 - x0 + 1)
    ny = Math.max(1, y1 - y0 + 1)
    if (nx * ny <= MAX_TILES || z <= 0) break
    z -= 1
  }

  const coords = []
  for (let ty = y0; ty <= y1; ty++) {
    for (let tx = x0; tx <= x1; tx++) {
      const x = wrapTileX(tx, z)
      coords.push([z, x, ty])
    }
  }
  const task = progress?.task('elevation', 'elevation', coords.length, 'tiles')
  let results
  try {
    results = await Promise.all(coords.map(([zz, x, y]) => tileElev(zz, x, y, progress).then((r) => {
      task?.tick()
      return r
    })))
  } catch (e) {
    task?.fail(String(e?.message || e))
    progress?.log(`elevation: ${e?.message || e}`, 'error')
    throw e
  }
  const tiles = results.map((r) => r.elev)
  const cached = results.filter((r) => r.cached).length
  const fetched = results.length - cached
  // In Web Mercator a column has one tile x and a row one tile y.
  const colPx = new Float64Array(outW)
  for (let col = 0; col < outW; col++) {
    const [lon] = mercToLonLat(xW + (col + 0.5) * (widthM / outW), yN)
    colPx[col] = lonLatToTile(lon, lat, z)[0] * TILE
  }
  const rowPx = new Float64Array(outH)
  for (let row = 0; row < outH; row++) {
    const [lon, la] = mercToLonLat(xW, yN - (row + 0.5) * (heightM / outH))
    rowPx[row] = lonLatToTile(lon, la, z)[1] * TILE - y0 * TILE
  }

  const elev = new Float32Array(outW * outH)
  // Tile pixels larger than ours: sampled whole, each would be a flat
  // square with a step at its edge, and lines would trace the steps.
  const magnified = (2 * Math.PI * R) / (TILE * 2 ** z) > 1.2 * pxM
  if (!magnified) {
    for (let row = 0; row < outH; row++) {
      const iy = Math.floor(rowPx[row])
      if (iy < 0 || iy >= ny * TILE) continue
      for (let col = 0; col < outW; col++) {
        const ix = mosaicColumn(x0, nx, z, colPx[col])
        if (ix >= 0) elev[row * outW + col] = mosaicAt(tiles, nx, ny, ix, iy)
      }
    }
  } else {
    // Catmull-Rom between tile-pixel centres: a surface with smooth slopes.
    const cols = Array.from(colPx, (px) => {
      const u = px - 0.5
      const raw = Math.floor(u) - x0 * TILE
      // Half a pixel past the edge is still the edge pixel; further, wrap.
      let ix = raw
      if (raw < -1 || raw >= nx * TILE) {
        const wrapped = mosaicColumn(x0, nx, z, Math.floor(u))
        ix = wrapped < 0 ? null : wrapped
      }
      return { ix, w: cubicWeights(u - Math.floor(u)) }
    })
    for (let row = 0; row < outH; row++) {
      const v = rowPx[row] - 0.5
      const iy = Math.floor(v)
      if (iy < -1 || iy >= ny * TILE) continue
      const wy = cubicWeights(v - iy)
      for (let col = 0; col < outW; col++) {
        const { ix, w: wx } = cols[col]
        if (ix === null) continue
        let sum = 0
        for (let j = 0; j < 4; j++) {
          const y = iy - 1 + j
          const r =
            wx[0] * mosaicAt(tiles, nx, ny, ix - 1, y) +
            wx[1] * mosaicAt(tiles, nx, ny, ix, y) +
            wx[2] * mosaicAt(tiles, nx, ny, ix + 1, y) +
            wx[3] * mosaicAt(tiles, nx, ny, ix + 2, y)
          sum += wy[j] * r
        }
        elev[row * outW + col] = sum
      }
    }
  }
  const patched = results.filter((r) => r.patched).length
  task?.ok(`z${z} · ${fetched} new${patched ? ` · ${patched} seabed from z${z - 1}` : ''}`)
  return { elev, width: outW, height: outH, pxM, z, tiles: nx * ny, cached, fetched }
}
