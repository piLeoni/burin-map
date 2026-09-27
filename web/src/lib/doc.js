/**
 * Drawing document: the JSON that describes a whole picture — where (`area`)
 * and how (`batches`). Same shape the CLI takes with --style-json, so a file
 * exported here can be fed straight to the command line.
 *
 * @typedef {{ place?: string, at?: string, bounds?: number[], size_km?: number[], px?: number, width_mm?: number, height_mm?: number, pen_mm?: number, scale?: number }} AreaDoc
 * @typedef {{ angle: number, style: Record<string, number> }} Batch
 * @typedef {{ area?: AreaDoc, batches: Batch[] }} Doc
 */

/** Build a document from the live camera and the batch list. */
export function makeDoc(batches, mapView) {
  const area = {}
  if (mapView?.center && Number.isFinite(mapView.zoom)) {
    // Encode the current view as explicit bounds is lossy; a centre + zoom is
    // what the map already persists, so keep it readable instead.
    area.at = `${mapView.center[1].toFixed(5)},${mapView.center[0].toFixed(5)}`
  }
  return { area, batches }
}

/** Pretty JSON for download / display. */
export function docToJson(doc) {
  const out = {}
  if (doc.area && Object.keys(doc.area).length > 0) out.area = doc.area
  out.batches = doc.batches.map((b) => ({
    angle: b.angle,
    style: Object.fromEntries(
      Object.entries(b.style).filter(([, v]) => typeof v === 'number' && Number.isFinite(v)),
    ),
  }))
  return JSON.stringify(out, null, 2)
}

/** Parse a document, accepting a bare batch array too. */
export function parseDoc(text) {
  const raw = JSON.parse(text)
  if (Array.isArray(raw)) return { area: undefined, batches: raw }
  if (!raw || typeof raw !== 'object') throw new Error('Expected a JSON object or array')
  return { area: raw.area && typeof raw.area === 'object' ? raw.area : undefined, batches: raw.batches ?? [] }
}

/** `#doc=<json>` fragment for sharing; empty string when there is nothing to share. */
export function docToHash(doc) {
  const compact = JSON.stringify({
    ...(doc.area && Object.keys(doc.area).length ? { area: doc.area } : {}),
    batches: doc.batches.map((b) => ({
      angle: b.angle,
      style: Object.fromEntries(
        Object.entries(b.style).filter(([, v]) => typeof v === 'number' && Number.isFinite(v)),
      ),
    })),
  })
  try {
    return `#doc=${encodeURIComponent(btoa(unicodeToBytes(compact)))}`
  } catch {
    return `#doc=${encodeURIComponent(compact)}`
  }
}

/** Read a document out of `location.hash`, if present. */
export function docFromHash(hash) {
  const m = /(?:^|[#&])doc=([^&]*)/.exec(hash || '')
  if (!m) return null
  const raw = decodeURIComponent(m[1])
  const text = (() => {
    try {
      return bytesToUnicode(atob(raw))
    } catch {
      return raw
    }
  })()
  return parseDoc(text)
}

/** UTF-8 safe binary string for btoa. */
function unicodeToBytes(s) {
  return String.fromCharCode(...new TextEncoder().encode(s))
}

/** Inverse of {@link unicodeToBytes}. */
function bytesToUnicode(s) {
  return new TextDecoder().decode(Uint8Array.from(s, (c) => c.charCodeAt(0)))
}
