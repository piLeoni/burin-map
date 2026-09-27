/**
 * The frame: the rectangle that gets drawn and exported. It is set either as
 * ground (`land`: width × height in km) or as paper (`paper`: width × height
 * in mm at a scale 1:N). The map is only a window onto it: the rectangle sits
 * centred on the map, dragging moves it, zooming changes how much ground it
 * holds, and typing a value moves the camera to fit it.
 *
 * Same rules as the CLI (`--terrain-size`, `--frame-size`, `--frame-scale`,
 * `--frame-pen`), so a drawing made here and its exported command match.
 *
 * @typedef {{ mode: 'land' | 'paper', size_km: number[] | null, size_mm: number[], scale: number, pen_width: number }} Frame
 */

/** Web Mercator, as used by Mapbox and by the elevation builder. */
const EARTH_M = 40075016.686
const R = 6378137
const TILE_PX = 512

/** Grid width in px for land drawings; the CLI's internal default. */
export const LAND_PX = 2400
/** Longest edge the browser engine will draw. */
export const MAX_EDGE = 3072
/** Space kept around the rectangle on screen, px; clears the map's buttons. */
export const FIT_PAD = 56

export const PEN_DEFAULT = 0.3

/** @type {Frame} */
export const DEFAULT_FRAME = {
  mode: 'land',
  size_km: null,
  size_mm: [210, 297],
  scale: 25000,
  pen_width: PEN_DEFAULT,
}

/** Named areas the CLI knows: `[west, south, east, north]`. */
export const PLACES = {
  bern: [7.10, 46.70, 7.80, 47.20],
  matterhorn: [7.40, 45.80, 8.00, 46.20],
  'san-francisco': [-122.75, 37.45, -122.05, 38.05],
}

/** `[side]` or `[w, h]` → `[w, h]`; anything else → null. */
export function pair(v) {
  if (!Array.isArray(v)) return null
  const n = v.map(Number)
  if (n.length === 1 && n[0] > 0) return [n[0], n[0]]
  if (n.length === 2 && n[0] > 0 && n[1] > 0) return [n[0], n[1]]
  return null
}

/** Three significant figures: 24873 → 24900, 8.237 → 8.24. */
export function sig3(x) {
  if (!Number.isFinite(x) || x === 0) return x
  const d = Math.ceil(Math.log10(Math.abs(x)))
  const f = 10 ** (3 - d)
  return Math.round(x * f) / f
}

/** Ground covered by the frame, `[w, h]` in km, or null before it is known. */
export function groundKm(frame) {
  if (frame.mode === 'paper') {
    const mm = pair(frame.size_mm)
    if (!mm || !(frame.scale > 0)) return null
    return [(mm[0] * frame.scale) / 1e6, (mm[1] * frame.scale) / 1e6]
  }
  return pair(frame.size_km)
}

/** Height over width of the frame. */
export function aspectOf(frame) {
  const g = groundKm(frame)
  if (g) return g[1] / g[0]
  const mm = frame.mode === 'paper' ? pair(frame.size_mm) : null
  return mm ? mm[1] / mm[0] : 1
}

/** The largest box of `aspect` (h / w) centred in `cw × ch`, `pad` px from the edges. */
export function fitBox(cw, ch, aspect, pad = FIT_PAD) {
  const aw = Math.max(1, cw - 2 * pad)
  const ah = Math.max(1, ch - 2 * pad)
  const w = Math.min(aw, ah / aspect)
  const h = w * aspect
  return { x: (cw - w) / 2, y: (ch - h) / 2, w, h }
}

/** Zoom at which `groundWkm` spans `boxWpx` screen pixels at latitude `lat`. */
export function zoomFor(lat, groundWkm, boxWpx) {
  const cos = Math.cos((lat * Math.PI) / 180)
  return Math.log2((EARTH_M * cos * boxWpx) / (TILE_PX * groundWkm * 1000))
}

/** Ground width in km that `boxWpx` screen pixels span at `zoom` and latitude `lat`. */
export function groundWidthAt(lat, zoom, boxWpx) {
  const cos = Math.cos((lat * Math.PI) / 180)
  return (boxWpx * EARTH_M * cos) / (TILE_PX * 2 ** zoom) / 1000
}

/** The frame after a zoom by hand, holding `groundWkm` across its width. */
export function withGroundWidth(frame, groundWkm) {
  const aspect = aspectOf(frame)
  if (frame.mode === 'paper') {
    const mm = pair(frame.size_mm)
    return { ...frame, scale: sig3((groundWkm * 1e6) / mm[0]) }
  }
  const w = sig3(groundWkm)
  return { ...frame, size_km: [w, sig3(w * aspect)] }
}

function toMerc(lng, lat) {
  return [(R * lng * Math.PI) / 180, R * Math.log(Math.tan(Math.PI / 4 + (lat * Math.PI) / 360))]
}

function fromMerc(x, y) {
  return [(x / R) * (180 / Math.PI), (2 * Math.atan(Math.exp(y / R)) - Math.PI / 2) * (180 / Math.PI)]
}

/**
 * Degrees box of `[w, h]` km of ground centred on `[lng, lat]`, built in
 * Mercator so its proportions match the rectangle on the map exactly.
 */
export function boundsAround([lng, lat], [wKm, hKm]) {
  const [cx, cy] = toMerc(lng, lat)
  const s = 1 / Math.cos((lat * Math.PI) / 180)
  const hw = (wKm * 1000 * s) / 2
  const hh = (hKm * 1000 * s) / 2
  const [west, south] = fromMerc(cx - hw, cy - hh)
  const [east, north] = fromMerc(cx + hw, cy + hh)
  return { west, south, east, north }
}

/**
 * Pixel grid to draw on, as the CLI would: 2400 px wide for land, and for
 * paper the width that makes `batch_stroke` px equal the pen in mm.
 * `capped` is set when the browser has to draw it smaller than the CLI.
 */
export function drawGrid(frame, stroke = 1.2) {
  const aspect = aspectOf(frame)
  let width = LAND_PX
  let mmPerPx = null
  if (frame.mode === 'paper') {
    const mm = pair(frame.size_mm)
    mmPerPx = (frame.pen_width || PEN_DEFAULT) / stroke
    width = Math.round(mm[0] / mmPerPx)
  }
  let height = Math.round(width * aspect)
  const full = { width, height }
  const over = Math.max(width, height) / MAX_EDGE
  let capped = null
  if (over > 1) {
    capped = full
    width = Math.round(width / over)
    height = Math.round(height / over)
    if (mmPerPx) mmPerPx *= over
  }
  return { width: Math.max(16, width), height: Math.max(16, height), mmPerPx, capped }
}

/**
 * A document's `area` block → `{ center, frame }` for the app, following the
 * CLI: ground and paper and scale, any two decide the third.
 * Throws when the block can't be drawn.
 */
export function frameFromArea(area, current = DEFAULT_FRAME) {
  if (!area) return { center: null, frame: current }
  let center = null
  let ground = pair(area.size_km)

  if (typeof area.at === 'string') {
    const m = /^\s*(-?\d+(?:\.\d+)?)\s*,\s*(-?\d+(?:\.\d+)?)\s*$/.exec(area.at)
    if (!m) throw new Error(`The web app needs the centre as "lat,lon", not "${area.at}"`)
    center = [Number(m[2]), Number(m[1])]
  } else {
    const b = Array.isArray(area.bounds) && area.bounds.length === 4
      ? area.bounds
      : area.place ? PLACES[area.place] : null
    if (area.place && !b) throw new Error(`Unknown place "${area.place}"`)
    if (b) {
      const [w, s, e, n] = b
      const lat = (s + n) / 2
      center = [(w + e) / 2, lat]
      const [x0, y0] = toMerc(w, s)
      const [x1, y1] = toMerc(e, n)
      const k = Math.cos((lat * Math.PI) / 180) / 1000
      if (ground) throw new Error('Give the ground size with a centre, not with bounds or a place')
      ground = [(x1 - x0) * k, (y1 - y0) * k]
    }
  }

  const paper = area.size_mm != null ? pair(area.size_mm) : null
  if (area.size_mm != null && !paper) throw new Error('The paper size needs 1 or 2 positive numbers in mm')
  const scale = area.scale != null ? Number(area.scale) : null
  if (scale != null && !(scale > 0)) throw new Error('The scale needs a positive number, like 25000 for 1:25,000')
  const pen = area.pen_mm != null ? Number(area.pen_mm) : current.pen_width ?? PEN_DEFAULT

  if (paper && scale && ground) {
    throw new Error('Ground, paper size and scale can\'t all be given: drop one')
  }
  if (paper || scale) {
    let mm = paper
    let n = scale
    if (!mm && !ground) throw new Error('A scale needs a ground size or a paper size with it')
    if (!mm) mm = [(ground[0] * 1e6) / n, (ground[1] * 1e6) / n]
    if (!n) {
      // Paper alone: the CLI's default ground is 30 km across.
      const gw = ground ? ground[0] : 30
      n = (gw * 1e6) / mm[0]
      if (ground && Math.abs(ground[1] / ground[0] - mm[1] / mm[0]) > 1e-3) {
        // The drawing shrinks to fit the paper; keep the paper's shape.
        n = Math.max((ground[0] * 1e6) / mm[0], (ground[1] * 1e6) / mm[1])
      }
    }
    return {
      center,
      frame: { ...current, mode: 'paper', size_mm: mm.map(sig3), scale: sig3(n), pen_width: pen },
    }
  }
  return {
    center,
    frame: { ...current, mode: 'land', size_km: ground ? ground.map(sig3) : current.size_km, pen_width: pen },
  }
}

/** The frame as a document `area` block, centred on `center` (`[lng, lat]`). */
export function areaFromFrame(frame, center) {
  const area = {}
  if (center) area.at = `${center[1].toFixed(5)},${center[0].toFixed(5)}`
  if (frame.mode === 'paper') {
    const mm = pair(frame.size_mm)
    if (mm) area.size_mm = mm[0] === mm[1] ? [mm[0]] : mm
    if (frame.scale > 0) area.scale = frame.scale
    if (frame.pen_width != null && frame.pen_width !== PEN_DEFAULT) area.pen_mm = frame.pen_width
  } else {
    const km = pair(frame.size_km)
    if (km) area.size_km = km[0] === km[1] ? [km[0]] : km
  }
  return area
}
