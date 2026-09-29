/**
 * Drawing document: where (`area`) and how (`batches`) — the shape the CLI
 * takes with --style-json. This file holds the compact URL notation; the
 * editable views (YAML, JSON5, CLI) are in views.js.
 *
 * @typedef {{ place?: string, at?: string, bounds?: number[], size_km?: number[], size_mm?: number[], scale?: number, pen_mm?: number }} AreaDoc
 * @typedef {{ name?: string, angle: number, style: Record<string, number> }} Batch
 * @typedef {{ area?: AreaDoc, batches: Batch[] }} Doc
 */

import { PARAM_SHORTS, SHORT_PARAMS } from './shorts.js'

/** Keep only finite numbers, so we never emit null/NaN into JSON. */
function numericStyle(style) {
  return Object.fromEntries(
    Object.entries(style).filter(([, v]) => typeof v === 'number' && Number.isFinite(v)),
  )
}

/** Keys the area block may hold; the same set the CLI accepts. */
const AREA_KEYS = ['place', 'at', 'bounds', 'size_km', 'size_mm', 'scale', 'pen_mm']

/** Parse a document, accepting a bare batch array too. */
export function parseDoc(text) {
  const raw = JSON.parse(text)
  if (Array.isArray(raw)) return { area: undefined, batches: raw }
  if (!raw || typeof raw !== 'object') throw new Error('Not a BurinMap document: expected a JSON object or list')
  const area = raw.area && typeof raw.area === 'object' ? raw.area : undefined
  for (const key of Object.keys(area ?? {})) {
    if (!AREA_KEYS.includes(key)) {
      throw new Error(`Unknown area setting "${key}". Allowed: ${AREA_KEYS.join(', ')}`)
    }
  }
  return { area, batches: raw.batches ?? raw.land?.batches ?? [], water: raw.water ?? null }
}

/* ================================================================= */
/*  Compact notation — URL only                                       */
/* ================================================================= */
//
// Short CLI names, no spaces, comma-separated, area and batches split by '|':
//
//   at37.75,-122.44,fs210x297,fsc25000|a0,s4.8,dl0.15;a90,s5.5,bf0.45
//
// Where `a` is the batch angle and `n` its name (with `%,;|` percent-escaped). The comma is what makes it unambiguous:
// `sa0.3` alone could be `sa=0.3` (sun azimuth) or `s=` + garbage, but split
// on commas first and each token has exactly one reading. Unknown names and
// malformed values are hard errors, never silent misreads.

/** Area fields, in the order they are written. */
const AREA_SHORTS = {
  at: 'at',
  place: 'tp',
  bounds: 'tbo',
  size_km: 'ts',
  size_mm: 'fs',
  scale: 'esc',
  pen_mm: 'pen',
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
    else if (key === 'size_km' || key === 'size_mm') areaParts.push(`${short}${v.map(num).join('x')}`)
    else if (key === 'at') areaParts.push(`${short}${v}`)
    else areaParts.push(`${short}${num(v)}`)
  }

  const batchParts = doc.batches.map((b) => batchNote(b, [`a${num(b.angle ?? 0)}`]))
  const note = `${areaParts.join(',')}|${batchParts.join(';')}`
  if (!doc.water) return note

  // Water: its settings, then its batches, `;`-separated. A batch leads with
  // how it follows (`shore` or `map`), which is what tells the two apart. A
  // drawn shoreline with a style of its own is `l` and its number of lines,
  // then that style.
  const w = doc.water
  const head = []
  if (w.shore === false) head.push('noshore')
  if (w.classes) head.push(`c${w.classes.join('+')}`)
  if (w.source) head.push(`src${escapeNote(w.source)}`)
  if (w.shore && typeof w.shore === 'object') head.push(batchNote({ style: w.shore.style }, [`l${w.shore.lines ?? 1}`]))
  const wet = w.batches.map((b) => batchNote(b, [b.follow ?? 'shore', `a${num(b.angle ?? 0)}`]))
  return `${note}|${[head.join(','), ...wet].join(';')}`
}

/** Only the notation's own separators are escaped; docToHash URL-encodes the rest. */
function escapeNote(s) {
  return s.replace(/[%,;|]/g, (c) => `%${c.charCodeAt(0).toString(16).toUpperCase()}`)
}

function batchNote(b, toks) {
  if (b.name) toks.push(`n${escapeNote(b.name)}`)
  for (const [key, v] of Object.entries(numericStyle(b.style))) {
    toks.push(`${PARAM_SHORTS[key] ?? key}${num(v)}`)
  }
  return toks.join(',')
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
  if (!m) throw new Error(`Can't read "${tok}"`)
  const [, name, raw] = m
  // `candidates` are already longest-first, so `salt45` never reads as `sa`.
  const hit = candidates.find((c) => c.short === name)
  if (!hit) throw new Error(`Unknown ${what} setting "${name}"`)
  return { key: hit.key, raw }
}

/** A plain numeric token. */
function readNumber(tok, candidates, what) {
  const { key, raw } = readToken(tok, candidates, what)
  const value = Number(raw)
  if (!Number.isFinite(value)) throw new Error(`"${tok}" needs a number after its name`)
  return { key, value, raw }
}

/** Decode compact notation into a document. Throws on anything it can't read. */
export function noteToDoc(note) {
  const text = String(note || '').trim()
  if (!text) throw new Error('The URL holds no drawing')

  const [areaText, batchText, waterText] = text.includes('|') ? text.split('|', 3) : ['', text]
  const area = {}

  if (areaText) {
    for (const tok of groupAreaTokens(areaText.split(','))) {
      // Coordinates and lists are read as text first, then split.
      const { key, raw } = readToken(tok, AREA_BY_LENGTH, 'area')
      if (key === 'bounds') {
        const parts = raw.split(',').map(Number)
        if (parts.length !== 4 || parts.some((n) => !Number.isFinite(n))) {
          throw new Error(`Bounds need 4 numbers (west, south, east, north): "${tok}"`)
        }
        area.bounds = parts
      } else if (key === 'size_km' || key === 'size_mm') {
        const parts = raw.split('x').map(Number)
        const unit = key === 'size_km' ? 'km, like 8x8' : 'mm, like 210x297'
        if (parts.length < 1 || parts.length > 2 || parts.some((n) => !Number.isFinite(n) || n <= 0)) {
          throw new Error(`Size needs 1 or 2 numbers in ${unit}: "${tok}"`)
        }
        area[key] = parts
      } else if (key === 'at') {
        if (!/^-?[\d.]+,-?[\\d.]+$/.test(raw) && raw.length === 0) {
          throw new Error(`Centre needs "lat,lon": "${tok}"`)
        }
        area.at = raw
      } else if (key === 'place') {
        area.place = raw
      } else {
        const value = Number(raw)
        if (!Number.isFinite(value)) throw new Error(`"${tok}" needs a number after its name`)
        area[key] = value
      }
    }
  }

  const batches = (batchText ? batchText.split(';') : []).filter((s) => s.trim() !== '').map((seg) => readBatch(seg))

  if (batches.length === 0) throw new Error('The URL holds no batches')
  const doc = { area: Object.keys(area).length ? area : undefined, batches }
  if (waterText != null) doc.water = readWater(waterText)
  return doc
}

/** One batch: `a` its angle, `n` its name, the style by short names; `shore` / `map` for water. */
function readBatch(seg) {
  let angle = 0
  let name
  let follow
  const style = {}
  for (const tok of seg.split(',')) {
    if (!tok) continue
    // No style short starts with `n`, so an `n…` token is always the name.
    if (tok[0] === 'n') {
      name = decodeURIComponent(tok.slice(1))
      continue
    }
    if (tok === 'shore' || tok === 'map') {
      follow = tok
      continue
    }
    const { key, value } = readNumber(tok, ANGLE_AND_STYLE, 'style')
    if (key === 'angle') angle = value
    else style[key] = value
  }
  const b = follow ? { follow, angle, style } : { angle, style }
  return name ? { name, ...b } : b
}

function readWater(text) {
  const [headText, ...segs] = text.split(';')
  const water = { batches: segs.filter((s) => s.trim() !== '').map((seg) => readBatch(seg)) }
  let shore = null
  for (const tok of headText.split(',')) {
    if (!tok) continue
    if (tok === 'noshore') water.shore = false
    else if (tok.startsWith('src')) water.source = decodeURIComponent(tok.slice(3))
    else if (tok[0] === 'c') water.classes = tok.slice(1).split('+')
    else if (/^l\d+$/.test(tok)) (shore ??= { style: {} }).lines = Number(tok.slice(1))
    else {
      const { key, value } = readNumber(tok, STYLE_BY_LENGTH, 'shoreline')
      ;(shore ??= { style: {} }).style[key] = value
    }
  }
  if (shore) water.shore = { lines: shore.lines ?? 1, style: shore.style }
  return water
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
