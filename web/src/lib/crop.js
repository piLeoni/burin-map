/**
 * Cut a drawing down to the export frame: the SVG and the GeoJSON keep only
 * what lies inside the rectangle, lines crossing its edge are cut there.
 */

/** Liang–Barsky: the part of segment p→q inside `[x0, y0, x1, y1]`, as `[t0, t1]`, or null. */
function clipSegment(p, q, [x0, y0, x1, y1]) {
  const dx = q[0] - p[0]
  const dy = q[1] - p[1]
  let t0 = 0
  let t1 = 1
  for (const [a, b] of [[-dx, p[0] - x0], [dx, x1 - p[0]], [-dy, p[1] - y0], [dy, y1 - p[1]]]) {
    if (a === 0) {
      if (b < 0) return null
      continue
    }
    const t = b / a
    if (a < 0) {
      if (t > t1) return null
      if (t > t0) t0 = t
    } else {
      if (t < t0) return null
      if (t < t1) t1 = t
    }
  }
  return [t0, t1]
}

const lerp = (p, q, t) => [p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t]

/** The pieces of a polyline inside `rect` (`[x0, y0, x1, y1]`). */
export function clipPolyline(pts, rect) {
  const out = []
  let run = null
  for (let i = 1; i < pts.length; i++) {
    const t = clipSegment(pts[i - 1], pts[i], rect)
    if (!t) {
      if (run) out.push(run)
      run = null
      continue
    }
    if (!run || t[0] > 0) {
      if (run) out.push(run)
      run = [lerp(pts[i - 1], pts[i], t[0])]
    }
    run.push(lerp(pts[i - 1], pts[i], t[1]))
    if (t[1] < 1) {
      out.push(run)
      run = null
    }
  }
  if (run) out.push(run)
  return out
}

function svgPaths(svg) {
  const paths = []
  for (const m of svg.matchAll(/<path d="M([^"]+)"/g)) {
    paths.push(m[1].split(/\s*L\s*/).map((xy) => xy.trim().split(/\s+/).map(Number)))
  }
  return paths
}

const num = (x) => String(Math.round(x * 100) / 100)

/**
 * The part of `svg` inside `rect` (drawing px), sized `paperMm` on paper
 * with lines `penMm` thick.
 */
export function cropSvg(svg, rect, paperMm, penMm) {
  const [x0, y0, x1, y1] = rect
  const w = x1 - x0
  const h = y1 - y0
  const stroke = (penMm * w) / paperMm[0]
  let body = ''
  for (const p of svgPaths(svg)) {
    for (const piece of clipPolyline(p, rect)) {
      body += `<path d="M${piece.map(([x, y]) => `${num(x - x0)} ${num(y - y0)}`).join(' L')}"/>\n`
    }
  }
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${num(paperMm[0])}mm" height="${num(paperMm[1])}mm" viewBox="0 0 ${num(w)} ${num(h)}">`
    + `<g fill="none" stroke="#000" stroke-width="${stroke.toPrecision(4)}" stroke-linecap="round" stroke-linejoin="round">\n`
    + `${body}</g></svg>\n`
}

/** The part of a MultiLineString FeatureCollection inside `[west, south, east, north]`. */
export function cropGeojson(geojson, box) {
  const doc = JSON.parse(geojson)
  for (const f of doc.features ?? []) {
    if (f.geometry?.type !== 'MultiLineString') continue
    const r6 = (x) => Math.round(x * 1e6) / 1e6
    f.geometry.coordinates = f.geometry.coordinates
      .flatMap((line) => clipPolyline(line, box))
      .map((line) => line.map(([lon, lat]) => [r6(lon), r6(lat)]))
  }
  return JSON.stringify(doc)
}
