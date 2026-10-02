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

/**
 * The part of a closed ring inside `rect`, or null (Sutherland–Hodgman).
 * Clipped one by one, rings still fill even-odd as before within the rect.
 */
export function clipRing(ring, [x0, y0, x1, y1]) {
  let pts = ring.length > 1 && ring[0][0] === ring.at(-1)[0] && ring[0][1] === ring.at(-1)[1] ? ring.slice(0, -1) : ring
  const sides = [
    [(p) => p[0] >= x0, (p, q) => lerp(p, q, (x0 - p[0]) / (q[0] - p[0]))],
    [(p) => p[0] <= x1, (p, q) => lerp(p, q, (x1 - p[0]) / (q[0] - p[0]))],
    [(p) => p[1] >= y0, (p, q) => lerp(p, q, (y0 - p[1]) / (q[1] - p[1]))],
    [(p) => p[1] <= y1, (p, q) => lerp(p, q, (y1 - p[1]) / (q[1] - p[1]))],
  ]
  for (const [inside, cut] of sides) {
    const out = []
    pts.forEach((q, i) => {
      const p = pts[(i + pts.length - 1) % pts.length]
      if (inside(q)) {
        if (!inside(p)) out.push(cut(p, q))
        out.push(q)
      } else if (inside(p)) {
        out.push(cut(p, q))
      }
    })
    pts = out
    if (pts.length < 3) return null
  }
  return pts
}

const pathPoints = (d) => d.split(/\s*L\s*/).map((xy) => xy.trim().split(/\s+/).map(Number))

/** A fill's rings from `M… L… Z M… Z`. */
const pathRings = (d) => d.split(/\s*Z\s*/).map((r) => r.replace(/^M\s*/, '')).filter(Boolean).map(pathPoints)

/** Each pen's group of the drawing: its `<g …>` attributes, and its paths or, for a fill, its rings. */
function svgPens(svg) {
  const pens = []
  for (const g of svg.matchAll(/<g( stroke="[^>]*)>([\s\S]*?)<\/g>/g)) {
    const fill = g[1].includes('fill-rule=')
    const ds = [...g[2].matchAll(/<path d="([^"]+)"/g)].map((m) => m[1])
    pens.push(fill
      ? { attrs: g[1], rings: ds.flatMap(pathRings) }
      : { attrs: g[1], paths: ds.map((d) => pathPoints(d.replace(/^M/, ''))) })
  }
  return pens
}

const num = (x) => String(Math.round(x * 100) / 100)

/** The stroke width of a drawing's SVG, in drawing px, or null; the first pen's when there are several. */
export function strokeOf(svg) {
  // Not only the head: a fill underneath can come first, and run long.
  const m = /stroke-width="([\d.]+)"/.exec(svg)
  return m ? Number(m[1]) : null
}

/**
 * The part of `svg` inside `rect` (drawing px), sized `paperMm` on paper.
 * Each pen keeps its colour, width, line ends and layer, so the export looks
 * like the screen.
 */
export function cropSvg(svg, rect, paperMm) {
  const [x0, y0, x1, y1] = rect
  const w = x1 - x0
  const h = y1 - y0
  let body = ''
  const at = ([x, y]) => `${num(x - x0)} ${num(y - y0)}`
  for (const pen of svgPens(svg)) {
    let inside = ''
    if (pen.rings) {
      const d = pen.rings.map((r) => clipRing(r, rect)).filter(Boolean).map((r) => `M${r.map(at).join(' L')} Z`)
      if (d.length) body += `<g${pen.attrs}>\n<path d="${d.join(' ')}"/>\n</g>\n`
      continue
    }
    for (const p of pen.paths) {
      for (const piece of clipPolyline(p, rect)) {
        inside += `<path d="M${piece.map(at).join(' L')}"/>\n`
      }
    }
    if (inside) body += `<g${pen.attrs}>\n${inside}</g>\n`
  }
  return '<svg xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape"'
    + ` width="${num(paperMm[0])}mm" height="${num(paperMm[1])}mm" viewBox="0 0 ${num(w)} ${num(h)}"`
    + ' fill="none" stroke-linecap="round" stroke-linejoin="round">\n'
    + `${body}</svg>\n`
}

/** The part of a drawing's FeatureCollection (lines, and fills as polygons) inside `[west, south, east, north]`. */
export function cropGeojson(geojson, box) {
  const doc = JSON.parse(geojson)
  const r6 = (x) => Math.round(x * 1e6) / 1e6
  const round = (line) => line.map(([lon, lat]) => [r6(lon), r6(lat)])
  for (const f of doc.features ?? []) {
    if (f.geometry?.type === 'MultiLineString') {
      f.geometry.coordinates = f.geometry.coordinates.flatMap((line) => clipPolyline(line, box)).map(round)
    } else if (f.geometry?.type === 'MultiPolygon') {
      f.geometry.coordinates = f.geometry.coordinates.flatMap((poly) => {
        const [outer, ...holes] = poly.map((r) => clipRing(r, box))
        if (!outer) return []
        return [[outer, ...holes.filter(Boolean)].map((r) => round([...r, r[0]]))]
      })
    }
  }
  doc.features = (doc.features ?? []).filter((f) => f.geometry?.type !== 'MultiPolygon' || f.geometry.coordinates.length)
  return JSON.stringify(doc)
}
