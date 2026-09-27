/**
 * Drawing document: the JSON that describes a whole picture — where (`area`)
 * and how (`batches`). Same shape the CLI takes with --style-json, so a file
 * exported here can be fed straight to the command line.
 *
 * @typedef {{ place?: string, at?: string, bounds?: number[], size_km?: number[], px?: number, width_mm?: number, height_mm?: number, pen_mm?: number, scale?: number }} AreaDoc
 * @typedef {{ angle: number, style: Record<string, number> }} Batch
 * @typedef {{ area?: AreaDoc, batches: Batch[] }} Doc
 */

import { PARAM_SHORTS, SHORT_PARAMS } from './shorts.js'

/** Build a document from the live camera and the batch list. */
export function makeDoc(batches, mapView) {
  const area = {}
  if (mapView?.center && Number.isFinite(mapView.zoom)) {
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
    style: numericStyle(b.style),
  }))
  return JSON.stringify(out, null, 2)
}

/** Keep only finite numbers, so we never emit null/NaN into JSON. */
function numericStyle(style) {
  return Object.fromEntries(
    Object.entries(style).filter(([, v]) => typeof v === 'number' && Number.isFinite(v)),
  )
}

/** Parse a document, accepting a bare batch array too. */
export function parseDoc(text) {
  const raw = JSON.parse(text)
  if (Array.isArray(raw)) return { area: undefined, batches: raw }
  if (!raw || typeof raw !== 'object') throw new Error('Expected a JSON object or array')
  return {
    area: raw.area && typeof raw.area === 'object' ? raw.area : undefined,
    batches: raw.batches ?? [],
  }
}

/* ================================================================= */
/*  Compact notation — URL only                                       */
/* ================================================================= */
//
// Short CLI names, no spaces, comma-separated, area and batches split by '|':
//
//   at37.75,-122.44,ts8x8,fpx1600|a0,s4.8,dl0.15;a90,s5.5,bf0.45
//
// Where `a` is the batch angle. The comma is what makes it unambiguous:
// `sa0.3` alone could be `sa=0.3` (sun azimuth) or `s=` + garbage, but split
// on commas first and each token has exactly one reading. Unknown names and
// malformed values are hard errors, never silent misreads.

/** Area fields, in the order they are written. */
const AREA_SHORTS = {
  at: 'at',
  place: 'tp',
  bounds: 'tbo',
  size_km: 'ts',
  px: 'fpx',
  width_mm: 'fw',
  height_mm: 'fh',
  scale: 'fsc',
  pen_mm: 'fpen',
}

/** Longest-first name list, so `salt` wins over `sa` and `s`. */
const AREA_BY_LENGTH = Object.entries(AREA_SHORTS)
  .map(([key, short]) => ({ key, short }))
  .sort((a, b) => b.short.length - a.short.length)

const STYLE_BY_LENGTH = Object.entries(SHORT_PARAMS)
  .map(([short, key]) => ({ short, key }))
  .sort((a, b) => b.short.length - a.short.length)

/** Batch angle first (it is not one of the 33 style params), then the style. */
const ANGLE_AND_STYLE = [{ short: 'a', key: 'angle' }, ...STYLE_BY_LENGTH]

/** `4.8` → `4.8`, `1` → `1`; trims float noise without scientific notation. */
function num(v) {
  return String(Number(v.toFixed(6)))
}

/** Encode a document as compact notation. */
export function docToNote(doc) {
  const areaParts = []
  const area = doc.area || {}
  for (const [key, short] of Object.entries(AREA_SHORTS)) {
    const v = area[key]
    if (v == null) continue
    if (key === 'bounds') areaParts.push(`${short}${v.join(',')}`)
    else if (key === 'size_km') areaParts.push(`${short}${v.join('x')}`)
    else if (key === 'at') areaParts.push(`${short}${v}`)
    else areaParts.push(`${short}${num(v)}`)
  }

  const batchParts = doc.batches.map((b) => {
    const toks = [`a${num(b.angle ?? 0)}`]
    for (const [key, v] of Object.entries(numericStyle(b.style))) {
      toks.push(`${PARAM_SHORTS[key] ?? key}${num(v)}`)
    }
    return toks.join(',')
  })

  return `${areaParts.join(',')}|${batchParts.join(';')}`
}

/**
 * The area section is comma-separated, but `at37.75,-122.44` and
 * `tbo-122.5,37.7,-122.4,37.8` contain commas of their own. A token that is
 * just a number continues whatever came before it.
 */
function groupAreaTokens(parts) {
  const out = []
  for (const part of parts) {
    if (part === '') continue
    if (/^-?[\d.]+$/.test(part) && out.length > 0) out[out.length - 1] += `,${part}`
    else out.push(part)
  }
  return out
}

/** Read one `nameValue` token against a candidate list, longest name first. */
function readToken(tok, candidates, what) {
  const m = /^([a-z]+)(.+)$/.exec(tok)
  if (!m) throw new Error(`cannot read ${what} token: "${tok}"`)
  const [, name, raw] = m
  // `candidates` are already longest-first, so `salt45` never reads as `sa`.
  const hit = candidates.find((c) => c.short === name)
  if (!hit) throw new Error(`unknown ${what} parameter: "${name}"`)
  return { key: hit.key, raw }
}

/** A plain numeric token. */
function readNumber(tok, candidates, what) {
  const { key, raw } = readToken(tok, candidates, what)
  const value = Number(raw)
  if (!Number.isFinite(value)) throw new Error(`bad number in "${tok}"`)
  return { key, value, raw }
}

/** Decode compact notation into a document. Throws on anything it can't read. */
export function noteToDoc(note) {
  const text = String(note || '').trim()
  if (!text) throw new Error('empty notation')

  const [areaText, batchText] = text.includes('|') ? text.split('|', 2) : ['', text]
  const area = {}

  if (areaText) {
    for (const tok of groupAreaTokens(areaText.split(','))) {
      // Coordinates and lists are read as text first, then split.
      const { key, raw } = readToken(tok, AREA_BY_LENGTH, 'area')
      if (key === 'bounds') {
        const parts = raw.split(',').map(Number)
        if (parts.length !== 4 || parts.some((n) => !Number.isFinite(n))) {
          throw new Error(`bounds needs 4 numbers: "${tok}"`)
        }
        area.bounds = parts
      } else if (key === 'size_km') {
        const parts = raw.split('x').map(Number)
        if (parts.length < 1 || parts.length > 2 || parts.some((n) => !Number.isFinite(n) || n <= 0)) {
          throw new Error(`size needs 1 or 2 numbers like 8x8: "${tok}"`)
        }
        area.size_km = parts
      } else if (key === 'at') {
        if (!/^-?[\d.]+,-?[\\d.]+$/.test(raw) && raw.length === 0) {
          throw new Error(`centre needs "lat,lon": "${tok}"`)
        }
        area.at = raw
      } else if (key === 'place') {
        area.place = raw
      } else {
        const value = Number(raw)
        if (!Number.isFinite(value)) throw new Error(`bad number in "${tok}"`)
        area[key] = value
      }
    }
  }

  const batches = (batchText ? batchText.split(';') : []).filter((s) => s.trim() !== '').map((seg) => {
    let angle = 0
    const style = {}
    for (const tok of seg.split(',')) {
      if (!tok) continue
      const { key, value } = readNumber(tok, ANGLE_AND_STYLE, 'style')
      if (key === 'angle') angle = value
      else style[key] = value
    }
    return { angle, style }
  })

  if (batches.length === 0) throw new Error('no batches in notation')
  return { area: Object.keys(area).length ? area : undefined, batches }
}

/* ================================================================= */
/*  URL fragment                                                      */
/* ================================================================= */

/** `#d=<notation>` for sharing. */
export function docToHash(doc) {
  return `#d=${encodeURIComponent(docToNote(doc)).replace(/%2C/g, ',').replace(/%7C/g, '|').replace(/%3B/g, ';')}`
}

/** Read a document out of `location.hash`. Understands `#d=` and the older `#doc=`. */
export function docFromHash(hash) {
  const h = hash || ''
  const compact = /(?:^|[#&])d=([^&]*)/.exec(h)
  if (compact) return noteToDoc(decodeURIComponent(compact[1]))

  // Legacy: base64 (or raw) JSON.
  const legacy = /(?:^|[#&])doc=([^&]*)/.exec(h)
  if (!legacy) return null
  const raw = decodeURIComponent(legacy[1])
  const text = (() => {
    try {
      return bytesToUnicode(atob(raw))
    } catch {
      return raw
    }
  })()
  return parseDoc(text)
}

/** Decode the UTF-8 bytes `btoa` was given, for legacy `#doc=` links. */
function bytesToUnicode(s) {
  return new TextDecoder().decode(Uint8Array.from(s, (c) => c.charCodeAt(0)))
}
