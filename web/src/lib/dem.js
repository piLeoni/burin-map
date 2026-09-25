/** Terrarium DEM mosaic for a lon/lat bounding box → Float32 elevation grid. */

const TERRARIUM = '/terrarium'
const TILE = 256
const R = 6378137

const tileCache = new Map()

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

async function tileElev(z, x, y) {
  const key = `${z}/${x}/${y}`
  if (tileCache.has(key)) return tileCache.get(key)
  if (tileCache.size > 256) tileCache.delete(tileCache.keys().next().value)

  const r = await fetch(`${TERRARIUM}/${z}/${x}/${y}.png`)
  if (!r.ok) {
    const empty = new Float32Array(TILE * TILE)
    tileCache.set(key, empty)
    return empty
  }
  const bmp = await createImageBitmap(await r.blob())
  const c = new OffscreenCanvas(TILE, TILE)
  const g = c.getContext('2d', { willReadFrequently: true })
  g.drawImage(bmp, 0, 0)
  const { data } = g.getImageData(0, 0, TILE, TILE)
  const elev = new Float32Array(TILE * TILE)
  for (let i = 0, j = 0; i < data.length; i += 4, j++) {
    elev[j] = data[i] * 256 + data[i + 1] + data[i + 2] / 256 - 32768
  }
  tileCache.set(key, elev)
  return elev
}

function sampleMosaic(tiles, x0, y0, nx, ny, px, py) {
  const x = px - x0 * TILE
  const y = py - y0 * TILE
  if (x < 0 || y < 0 || x >= nx * TILE || y >= ny * TILE) return 0
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
  const { west: lonW, south: latS, east: lonE, north: latN } = bounds
  const [xW, yS] = lonLatToMerc(lonW, latS)
  const [xE, yN] = lonLatToMerc(lonE, latN)
  const widthM = xE - xW
  const heightM = yN - yS
  const lat = (latS + latN) / 2
  const pxM = widthM / outW
  const z = chooseZ(lat, pxM)

  const [tx0f, ty0f] = lonLatToTile(lonW, latN, z)
  const [tx1f, ty1f] = lonLatToTile(lonE, latS, z)
  const x0 = Math.floor(tx0f)
  const y0 = Math.floor(ty0f)
  const x1 = Math.floor(tx1f)
  const y1 = Math.floor(ty1f)
  const nx = x1 - x0 + 1
  const ny = y1 - y0 + 1
  if (nx * ny > 80) throw new Error(`Too many elevation tiles (${nx}×${ny}). Zoom in a bit.`)

  onProgress?.(`Fetching elevation (${nx * ny} tiles)…`)
  const jobs = []
  for (let ty = y0; ty <= y1; ty++) {
    for (let tx = x0; tx <= x1; tx++) jobs.push(tileElev(z, tx, ty))
  }
  const tiles = await Promise.all(jobs)

  const elev = new Float32Array(outW * outH)
  for (let row = 0; row < outH; row++) {
    const my = yN - (row + 0.5) * (heightM / outH)
    for (let col = 0; col < outW; col++) {
      const mx = xW + (col + 0.5) * (widthM / outW)
      const [lon, lat] = mercToLonLat(mx, my)
      const [fx, fy] = lonLatToTile(lon, lat, z)
      elev[row * outW + col] = sampleMosaic(tiles, x0, y0, nx, ny, fx * TILE, fy * TILE)
    }
  }
  return { elev, width: outW, height: outH, pxM, z, tiles: nx * ny }
}
