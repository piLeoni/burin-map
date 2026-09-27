/** Terrarium DEM mosaic for a lon/lat bounding box → Float32 elevation grid. */

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

async function fetchTileElev(z, x, y) {
  const r = await fetch(`${TERRARIUM}/${z}/${x}/${y}.png`)
  if (!r.ok) return new Float32Array(TILE * TILE)
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

/**
 * Cached terrarium tile. Reuses memory + dedupes concurrent requests for the same z/x/y.
 * @returns {Promise<{ elev: Float32Array, cached: boolean }>}
 */
async function tileElev(z, x, y) {
  x = wrapTileX(x, z)
  const key = `${z}/${x}/${y}`
  if (tileCache.has(key)) {
    const elev = tileCache.get(key)
    touchCache(key, elev)
    return { elev, cached: true }
  }
  if (inflight.has(key)) {
    const elev = await inflight.get(key)
    return { elev, cached: true }
  }
  const p = fetchTileElev(z, x, y)
  inflight.set(key, p)
  try {
    const elev = await p
    touchCache(key, elev)
    return { elev, cached: false }
  } finally {
    inflight.delete(key)
  }
}

function sampleMosaic(tiles, x0, y0, nx, ny, z, px, py) {
  const n = 2 ** z
  // Horizontal wrap so samples near ±180° still hit the mosaic.
  let x = px - x0 * TILE
  const y = py - y0 * TILE
  if (y < 0 || y >= ny * TILE) return 0
  if (x < 0 || x >= nx * TILE) {
    // try wrapping by full world width in tile space
    const world = n * TILE
    x = ((x % world) + world) % world
    if (x >= nx * TILE) return 0
  }
  const ix = Math.min(Math.floor(x), nx * TILE - 1)
  const iy = Math.min(Math.floor(y), ny * TILE - 1)
  const elev = tiles[Math.floor(iy / TILE) * nx + Math.floor(ix / TILE)]
  if (!elev) return 0
  return elev[(iy % TILE) * TILE + (ix % TILE)]
}

/**
 * @param {{ west: number, south: number, east: number, north: number }} bounds degrees
 * @param {number} outW
 * @param {number} outH
 * @param {(msg: string) => void} [onProgress]
 */
export async function buildDem(bounds, outW, outH, onProgress) {
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
  const missing = coords.filter(([zz, x, y]) => {
    const key = `${zz}/${x}/${y}`
    return !tileCache.has(key) && !inflight.has(key)
  }).length
  if (missing > 0) onProgress?.(`Downloading elevation (${missing} tile${missing === 1 ? '' : 's'})…`)

  const results = await Promise.all(coords.map(([zz, x, y]) => tileElev(zz, x, y)))
  const tiles = results.map((r) => r.elev)
  const cached = results.filter((r) => r.cached).length
  const fetched = results.length - cached
  const elev = new Float32Array(outW * outH)
  for (let row = 0; row < outH; row++) {
    const my = yN - (row + 0.5) * (heightM / outH)
    for (let col = 0; col < outW; col++) {
      const mx = xW + (col + 0.5) * (widthM / outW)
      const [lon, la] = mercToLonLat(mx, my)
      const [fx, fy] = lonLatToTile(lon, la, z)
      elev[row * outW + col] = sampleMosaic(tiles, x0, y0, nx, ny, z, fx * TILE, fy * TILE)
    }
  }
  return { elev, width: outW, height: outH, pxM, z, tiles: nx * ny, cached, fetched }
}
