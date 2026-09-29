/**
 * One drawing written three ways — YAML, JSON5 and a burin-map command — so
 * the editor can switch between them without losing anything, comments included.
 *
 * The model:
 *   area     the area block (`at`, `size_mm`, …), the same keys as the CLI's JSON
 *   batches  [{ name?, angle, style }], where `style` holds only the parameters
 *            that are set; the others are written commented out, at their default
 *            or, for the terrain ones, at what a batch above sets
 *   water    null, or { classes?, shoreline?, source?, batches: [{ name?, follow, draw?, angle, style }] },
 *            where `shoreline` is absent (drawn), false, or { lines, style },
 *            and `draw` is absent (lines) or a kind of wave mark;
 *            `land: { batches }` is read as `batches`
 *   notes    the user's own comments, by path: { above?: string[], after?: string }
 *            '' is the top of the document; then 'area', 'area.at', 'batches',
 *            'batches.0', 'batches.0.name', 'batches.0.angle', 'batches.0.style',
 *            'batches.0.style.space', …
 *
 * The `# default …` notes, group headers and commented-out defaults come from
 * the engine's parameter list every time a view is written, and are
 * recognised and dropped when it is read back, so they are never stored.
 *
 * `schema` is `{ params: [{ name, default, help, group }], groups: [{ id, blurb }] }`,
 * as returned by `style_params()` / `style_groups()`.
 *
 * @typedef {{ above?: string[], after?: string }} Note
 * @typedef {{ area: Record<string, any>, batches: Array<{ name?: string, angle: number, style: Record<string, number> }>, notes: Record<string, Note> }} Model
 */
import { LineCounter, parseDocument, isMap, isSeq, isScalar, stringify as yamlStringify } from 'yaml'
import { SHORT_PARAMS } from './shorts.js'

export const VIEWS = {
  yaml: { label: 'YAML', ext: 'yaml', language: 'yaml', mime: 'application/yaml' },
  json5: { label: 'JSON5', ext: 'json5', language: 'json', mime: 'application/json5' },
  cli: { label: 'CLI', ext: 'sh', language: 'burin-cli', mime: 'text/x-shellscript' },
}

const AREA_KEYS = ['place', 'at', 'bounds', 'size_km', 'size_mm', 'scale', 'pen_mm']

const AREA_HELP = {
  place: 'named area: bern, matterhorn, san-francisco',
  at: 'centre, "lat,lon"; dragging the map changes it',
  bounds: 'west, south, east, north, in degrees',
  size_km: 'ground width [and height], km',
  size_mm: 'paper width [and height], mm',
  scale: 'map scale, the N in 1:N',
  pen_mm: 'pen width on paper, mm',
}

const AREA_FLAG = {
  place: 'terrain-place',
  at: 'terrain-at',
  bounds: 'terrain-bounds',
  size_km: 'terrain-size',
  size_mm: 'frame-size',
  scale: 'export-scale',
  pen_mm: 'pen-width',
}

const ANGLE_HELP = 'line direction: 0 follows the contour lines, 90 runs straight downhill'

const SHEBANG = '#!/usr/bin/env bash'

/* ================================================================= */
/*  Shared helpers                                                    */
/* ================================================================= */

/** `4.8` → `4.8`; trims float noise without scientific notation. */
function num(v) {
  return String(Number(Number(v).toFixed(6)))
}

/** Quote for a POSIX shell only when needed. */
function sh(v) {
  const s = String(v)
  return /^[\w.,:/@%+-]+$/.test(s) ? s : `'${s.replace(/'/g, `'\\''`)}'`
}

function groupHeader(id) {
  return `── ${id[0].toUpperCase()}${id.slice(1)}`
}

function paramByName(schema, name) {
  return schema.params.find((p) => p.name === name)
}

const ANGLE_NOTE = 'default 0'

/**
 * Where a style is: `land`, a `water` batch that draws lines, `ripples` or
 * `dots`, or the `shoreline`. Each has its own defaults, and leaves out what
 * means nothing there.
 * @typedef {'land' | 'water' | 'ripples' | 'dots' | 'shoreline'} Kind
 */

/** A parameter's default in a style of that kind. */
function defaultOf(p, kind) {
  if (kind === 'water') return p.water_default ?? p.default
  if (kind === 'ripples') return p.ripples_default ?? p.default
  if (kind === 'dots') return p.dots_default ?? p.default
  if (kind === 'shoreline') return p.shoreline_default ?? p.default
  return p.default
}

function defaultNote(p, kind = 'land') {
  return `default ${num(defaultOf(p, kind))}`
}

/** What the shoreline uses: no tracer and no terrain, just a line to dash and bend. */
const SHORELINE_PARAMS = new Set([
  'space', 'dash_light', 'dash_dark', 'dash_period', 'dash_cutoff', 'mod_jitter', 'mod_breaks',
  'mod_wobble', 'mod_wobble_size', 'mod_smooth', 'mod_wave', 'mod_wave_size', 'mod_wave_zone', 'batch_stroke', 'batch_seed',
  'water_reach', 'water_fade',
])

/** What ripples use: rows, their strokes and ink, the swell and the hand's wave; no tracer. */
const RIPPLES_PARAMS = new Set([
  'space', 'dash_light', 'dash_dark', 'mod_wobble', 'mod_wobble_size', 'mod_wave', 'mod_wave_size', 'mod_wave_zone', 'mod_smooth',
  'batch_stroke', 'batch_seed', 'water_start', 'water_reach', 'water_fade', 'water_size', 'water_patches',
])

/** What dots use: how far apart, how big, and where; no tracer, no dashes. */
const DOTS_PARAMS = new Set([
  'space', 'space_light', 'space_dark', 'mod_jitter',
  'batch_stroke', 'batch_seed', 'water_start', 'water_reach', 'water_fade', 'water_size', 'water_patches',
])

/** What water has no use for: its tone is the distance from the shore, not terrain. */
const WATER_SKIPS = new Set(['space_steep', 'space_steep_gamma', 'batch_floor', 'water_size', 'water_patches'])

/** Parameters that mean nothing there: terrain shading on water, water settings on land. */
function skipsParam(p, kind) {
  if (kind === 'shoreline') return !SHORELINE_PARAMS.has(p.name)
  if (kind === 'ripples') return !RIPPLES_PARAMS.has(p.name)
  if (kind === 'dots') return !DOTS_PARAMS.has(p.name)
  if (kind === 'water') {
    return (p.group === 'input' && p.name !== 'input_flat') || p.group === 'sun' || WATER_SKIPS.has(p.name)
  }
  return p.group === 'water'
}

const WATER_CLASSES = ['ocean', 'lake', 'river', 'dock', 'pond', 'swimming_pool']
const WATER_DRAWS = ['lines', 'ripples', 'dots']

/** A water batch's kind of style, from what it draws. */
const drawKind = (b) => (b.draw === 'ripples' || b.draw === 'dots' ? b.draw : 'water')
const WATER_HELP = {
  water: 'sea, lakes and rivers from OpenStreetMap; also decides where the land is',
  classes: `which water: ${WATER_CLASSES.join(', ')}`,
  shoreline: 'draw the shoreline; or give it lines and a style, like a batch',
  lines: 'how many: the shore, then more out to sea, `space` px apart',
  source: 'TileJSON URL or {z}/{x}/{y} template; default OpenFreeMap',
  follow: 'shore: along the coast; map: one direction over the whole map',
  draw: 'lines fill the water; ripples draw broken rows of hand-drawn waves; dots stipple it',
  angle: 'follow shore: 0 along the coast, 90 straight out; follow map: degrees from north',
}

/** Trailing comments that are generated for a path — today's and older, longer ones. */
function generatedAfter(schema, path) {
  const parts = path.split('.')
  if (parts[0] === 'area' && parts.length === 2) return [AREA_HELP[parts[1]]]
  if (parts[0] === 'batches' && parts[2] === 'angle') return [ANGLE_NOTE, ANGLE_HELP]
  if (parts[0] === 'batches' && parts[2] === 'style' && parts.length === 4) {
    const p = paramByName(schema, parts[3])
    if (p) return [defaultNote(p), p.help, `${p.help} (default ${num(p.default)})`]
  }
  if (parts[0] === 'water') {
    if (parts.length === 1) return [WATER_HELP.water]
    if (parts.length === 2) return [WATER_HELP[parts[1]]]
    if (parts[1] === 'batches' && parts[3] === 'angle') return [ANGLE_NOTE, ANGLE_HELP]
    if (parts[1] === 'batches' && parts.length === 4) return [WATER_HELP[parts[3]]]
    if (parts[1] === 'batches' && parts[3] === 'style' && parts.length === 5) {
      const p = paramByName(schema, parts[4])
      if (p) return [defaultNote(p, 'water'), defaultNote(p, 'ripples'), defaultNote(p, 'dots')]
    }
    if (parts[1] === 'shoreline' && parts[2] === 'lines') return [WATER_HELP.lines]
    if (parts[1] === 'shoreline' && parts[2] === 'style' && parts.length === 4) {
      const p = paramByName(schema, parts[3])
      if (p) return [defaultNote(p, 'shoreline')]
    }
  }
  return []
}

/** Collects the user's comments while a view is read, dropping generated ones. */
class NoteBook {
  constructor(schema, isGenerated) {
    this.schema = schema
    this.isGenerated = isGenerated
    /** @type {Record<string, Note>} */
    this.notes = {}
  }

  above(path, lines) {
    const keep = (Array.isArray(lines) ? lines : String(lines ?? '').split('\n'))
      .map((l) => l.trim())
      .filter((l) => l !== '' && !l.startsWith('── ') && !this.isGenerated(l))
    if (keep.length === 0) return
    const n = (this.notes[path] ??= {})
    n.above = [...(n.above ?? []), ...keep]
  }

  after(path, text) {
    const t = String(text ?? '').trim()
    if (!t || generatedAfter(this.schema, path).includes(t)) return
    const n = (this.notes[path] ??= {})
    n.after = n.after ? `${n.after} ${t}` : t
  }
}

/** Names a commented-out line may start with. */
function knownKey(schema, key) {
  return key === 'angle' || schema.params.some((p) => p.name === key)
}

/* ---------- rows: one line of output each ---------- */

const code = (indent, text, note) => ({ kind: 'code', indent, text, note })
const off = (indent, text, note) => ({ kind: 'off', indent, text, note })
const comment = (indent, text) => ({ kind: 'comment', indent, text })
const blank = () => ({ kind: 'blank' })

function aboveRows(rows, note, indent) {
  for (const line of note?.above ?? []) rows.push(comment(indent, line))
}

/** Lay the rows out with every trailing comment starting in the same column. */
function emit(rows, marker) {
  const head = (r) => ' '.repeat(r.indent) + (r.kind === 'off' ? `${marker} ${r.text}` : r.text)
  const widths = rows.filter((r) => (r.kind === 'code' || r.kind === 'off') && r.note).map((r) => head(r).length)
  const col = Math.min(Math.max(0, ...widths), 44) + 2
  return rows.map((r) => {
    if (r.kind === 'blank') return ''
    if (r.kind === 'comment') return `${' '.repeat(r.indent)}${marker} ${r.text}`
    const h = head(r)
    return r.note ? `${h.padEnd(col - 2)}  ${marker} ${r.note}` : h
  }).join('\n') + '\n'
}

/**
 * The rows for a batch's style: group headers, set values, commented-out
 * defaults. `base` is the path that holds the style; `kind` says whose it is.
 */
function styleRows(rows, style, base, schema, notes, indent, line, kind = 'land', inherited = {}) {
  let group = null
  for (const p of schema.params) {
    if (skipsParam(p, kind) && !(p.name in style)) continue
    if (p.group !== group) {
      group = p.group
      rows.push(comment(indent, groupHeader(group)))
    }
    const path = `${base}.style.${p.name}`
    if (p.name in style) {
      aboveRows(rows, notes[path], indent)
      rows.push(code(indent, line(p.name, style[p.name]), notes[path]?.after ?? defaultNote(p, kind)))
    } else {
      rows.push(off(indent, line(p.name, inherited[p.name] ?? defaultOf(p, kind))))
    }
  }
}

const TERRAIN_NAMES = new Set([
  'space_light', 'space_dark', 'space_steep', 'space_steep_gamma', 'mod_wobble', 'mod_wobble_size', 'mod_smooth',
])

/** Shapes the terrain a batch follows; a batch that leaves one out takes the one above's. */
function isTerrain(p) {
  return p.group === 'input' || p.group === 'sun' || TERRAIN_NAMES.has(p.name)
}

/** For each land batch, the terrain values it takes from the batches above. */
function inheritedTerrain(batches, schema) {
  let carry = {}
  return batches.map((b) => {
    const mine = carry
    carry = { ...carry }
    for (const p of schema.params) if (isTerrain(p) && p.name in b.style) carry[p.name] = b.style[p.name]
    return mine
  })
}

function areaKeys(area) {
  return AREA_KEYS.filter((k) => area?.[k] != null)
}

/* ================================================================= */
/*  YAML                                                              */
/* ================================================================= */

function yamlValue(v) {
  if (Array.isArray(v)) return `[${v.map(num).join(', ')}]`
  if (typeof v === 'number') return num(v)
  return yamlStringify(String(v)).trim()
}

function renderYaml(model, schema) {
  const n = model.notes ?? {}
  const rows = []
  aboveRows(rows, n[''], 0)
  if (n['']?.above?.length) rows.push(blank())

  const keys = areaKeys(model.area)
  if (keys.length) {
    aboveRows(rows, n.area, 0)
    rows.push(code(0, 'area:', n.area?.after))
    for (const k of keys) {
      const path = `area.${k}`
      aboveRows(rows, n[path], 2)
      rows.push(code(2, `${k}: ${yamlValue(model.area[k])}`, n[path]?.after))
    }
  }

  aboveRows(rows, n.batches, 0)
  rows.push(code(0, 'batches:', n.batches?.after))
  const inherited = inheritedTerrain(model.batches, schema)
  model.batches.forEach((b, i) => {
    const base = `batches.${i}`
    aboveRows(rows, n[base], 2)
    const head = []
    if (b.name) head.push(['name', yamlValue(b.name), undefined])
    head.push(['angle', num(b.angle ?? 0), ANGLE_NOTE])
    head.forEach(([k, v, help], j) => {
      const path = `${base}.${k}`
      aboveRows(rows, n[path], j === 0 ? 2 : 4)
      rows.push(code(j === 0 ? 2 : 4, `${j === 0 ? '- ' : ''}${k}: ${v}`, n[path]?.after ?? help))
    })
    aboveRows(rows, n[`${base}.style`], 4)
    rows.push(code(4, 'style:', n[`${base}.style`]?.after))
    styleRows(rows, b.style, base, schema, n, 6, (k, v) => `${k}: ${num(v)}`, 'land', inherited[i])
  })

  const w = model.water
  if (w) {
    aboveRows(rows, n.water, 0)
    rows.push(code(0, 'water:', n.water?.after ?? WATER_HELP.water))
    const setting = (k, v) => {
      aboveRows(rows, n[`water.${k}`], 2)
      rows.push(code(2, `${k}: ${v}`, n[`water.${k}`]?.after ?? WATER_HELP[k]))
    }
    if (w.classes) setting('classes', `[${w.classes.join(', ')}]`)
    if (typeof w.shoreline === 'object') {
      aboveRows(rows, n['water.shoreline'], 2)
      rows.push(code(2, 'shoreline:', n['water.shoreline']?.after ?? WATER_HELP.shoreline))
      aboveRows(rows, n['water.shoreline.lines'], 4)
      rows.push(code(4, `lines: ${num(w.shoreline.lines)}`, n['water.shoreline.lines']?.after ?? WATER_HELP.lines))
      aboveRows(rows, n['water.shoreline.style'], 4)
      rows.push(code(4, 'style:', n['water.shoreline.style']?.after))
      styleRows(rows, w.shoreline.style, 'water.shoreline', schema, n, 6, (k, v) => `${k}: ${num(v)}`, 'shoreline')
    } else {
      setting('shoreline', w.shoreline === false ? 'false' : 'true')
    }
    if (w.source) setting('source', yamlValue(w.source))
    aboveRows(rows, n['water.batches'], 2)
    rows.push(code(2, w.batches.length ? 'batches:' : 'batches: []', n['water.batches']?.after))
    w.batches.forEach((b, i) => {
      const base = `water.batches.${i}`
      aboveRows(rows, n[base], 4)
      const head = []
      if (b.name) head.push(['name', yamlValue(b.name), undefined])
      head.push(['follow', b.follow, WATER_HELP.follow])
      head.push(['draw', b.draw ?? 'lines', WATER_HELP.draw])
      head.push(['angle', num(b.angle ?? 0), ANGLE_NOTE])
      head.forEach(([k, v, help], j) => {
        const path = `${base}.${k}`
        aboveRows(rows, n[path], j === 0 ? 4 : 6)
        rows.push(code(j === 0 ? 4 : 6, `${j === 0 ? '- ' : ''}${k}: ${v}`, n[path]?.after ?? help))
      })
      aboveRows(rows, n[`${base}.style`], 6)
      rows.push(code(6, 'style:', n[`${base}.style`]?.after))
      styleRows(rows, b.style, base, schema, n, 8, (k, v) => `${k}: ${num(v)}`, drawKind(b))
    })
  }
  return emit(rows, '#')
}

/** Where a line's trailing `#` comment starts (outside quotes), or -1. */
function yamlCommentAt(line) {
  let quote = null
  for (let i = 0; i < line.length; i++) {
    const c = line[i]
    if (quote) {
      if (c === quote) quote = null
    } else if (c === '"' || c === "'") {
      quote = c
    } else if (c === '#' && (i === 0 || /\s/.test(line[i - 1]))) {
      return i
    }
  }
  return -1
}

function parseYaml(text, schema) {
  const lc = new LineCounter()
  const doc = parseDocument(text, { lineCounter: lc })
  if (doc.errors.length) {
    const e = doc.errors[0]
    const line = e.linePos?.[0]?.line
    throw new Error(`YAML${line ? `, line ${line}` : ''}: ${e.message.split('\n')[0].replace(/ at line \d+, column \d+:?\s*$/, '')}`)
  }

  // Every key and list item, with the line it starts on.
  const entries = []
  const walk = (node, path, depth) => {
    if (isMap(node)) {
      for (const pair of node.items) {
        const key = isScalar(pair.key) ? String(pair.key.value) : String(pair.key)
        const p = path ? `${path}.${key}` : key
        if (pair.key?.range) entries.push({ path: p, line: lc.linePos(pair.key.range[0]).line, depth })
        if (pair.value && !isScalar(pair.value) && !pair.value.flow) walk(pair.value, p, depth + 1)
      }
    } else if (isSeq(node)) {
      node.items.forEach((item, i) => {
        const p = `${path}.${i}`
        if (item?.range) entries.push({ path: p, line: lc.linePos(item.range[0]).line, depth })
        if (item && !isScalar(item)) walk(item, p, depth + 1)
      })
    }
  }
  const root = isSeq(doc.contents) ? 'batches' : ''
  walk(doc.contents, root, 0)
  const byLine = new Map()
  for (const e of entries) byLine.set(e.line, [...(byLine.get(e.line) ?? []), e])

  // An own-line comment belongs to the next key; a trailing one to the key on its line.
  const book = new NoteBook(schema, (l) => {
    const m = /^([a-z_]+):\s*\S+(\s+#.*)?$/.exec(l)
    return !!m && knownKey(schema, m[1])
  })
  let pending = []
  text.split('\n').forEach((raw, idx) => {
    const at = byLine.get(idx + 1)
    const hash = yamlCommentAt(raw)
    if (!at) {
      if (hash >= 0 && raw.slice(0, hash).trim() === '') pending.push(raw.slice(hash + 1))
      return
    }
    const shallow = at.reduce((a, b) => (b.depth < a.depth ? b : a))
    const deep = at.reduce((a, b) => (b.depth > a.depth ? b : a))
    book.above(idx + 1 === entries[0].line ? '' : shallow.path, pending)
    pending = []
    if (hash >= 0) book.after(deep.path, raw.slice(hash + 1))
  })
  book.above('', pending)
  return toModel(doc.toJS(), book.notes, schema)
}

/* ================================================================= */
/*  JSON5                                                             */
/* ================================================================= */

function renderJson5(model, schema) {
  const n = model.notes ?? {}
  const rows = []
  aboveRows(rows, n[''], 0)
  if (n['']?.above?.length) rows.push(blank())
  rows.push(code(0, '{'))

  const keys = areaKeys(model.area)
  if (keys.length) {
    aboveRows(rows, n.area, 2)
    rows.push(code(2, '"area": {', n.area?.after))
    for (const k of keys) {
      const path = `area.${k}`
      const v = model.area[k]
      const text = Array.isArray(v) ? `[${v.map(num).join(', ')}]` : typeof v === 'number' ? num(v) : JSON.stringify(String(v))
      aboveRows(rows, n[path], 4)
      rows.push(code(4, `"${k}": ${text},`, n[path]?.after))
    }
    rows.push(code(2, '},'))
  }

  aboveRows(rows, n.batches, 2)
  rows.push(code(2, '"batches": [', n.batches?.after))
  const inherited = inheritedTerrain(model.batches, schema)
  model.batches.forEach((b, i) => {
    const base = `batches.${i}`
    aboveRows(rows, n[base], 4)
    rows.push(code(4, '{', n[base]?.after))
    if (b.name) {
      aboveRows(rows, n[`${base}.name`], 6)
      rows.push(code(6, `"name": ${JSON.stringify(b.name)},`, n[`${base}.name`]?.after))
    }
    aboveRows(rows, n[`${base}.angle`], 6)
    rows.push(code(6, `"angle": ${num(b.angle ?? 0)},`, n[`${base}.angle`]?.after ?? ANGLE_NOTE))
    aboveRows(rows, n[`${base}.style`], 6)
    rows.push(code(6, '"style": {', n[`${base}.style`]?.after))
    styleRows(rows, b.style, base, schema, n, 8, (k, v) => `"${k}": ${num(v)},`, 'land', inherited[i])
    rows.push(code(6, '},'))
    rows.push(code(4, '},'))
  })
  rows.push(code(2, '],'))

  const w = model.water
  if (w) {
    aboveRows(rows, n.water, 2)
    rows.push(code(2, '"water": {', n.water?.after ?? WATER_HELP.water))
    const setting = (k, v) => {
      aboveRows(rows, n[`water.${k}`], 4)
      rows.push(code(4, `"${k}": ${v},`, n[`water.${k}`]?.after ?? WATER_HELP[k]))
    }
    if (w.classes) setting('classes', `[${w.classes.map((c) => JSON.stringify(c)).join(', ')}]`)
    if (typeof w.shoreline === 'object') {
      aboveRows(rows, n['water.shoreline'], 4)
      rows.push(code(4, '"shoreline": {', n['water.shoreline']?.after ?? WATER_HELP.shoreline))
      aboveRows(rows, n['water.shoreline.lines'], 6)
      rows.push(code(6, `"lines": ${num(w.shoreline.lines)},`, n['water.shoreline.lines']?.after ?? WATER_HELP.lines))
      aboveRows(rows, n['water.shoreline.style'], 6)
      rows.push(code(6, '"style": {', n['water.shoreline.style']?.after))
      styleRows(rows, w.shoreline.style, 'water.shoreline', schema, n, 8, (k, v) => `"${k}": ${num(v)},`, 'shoreline')
      rows.push(code(6, '},'))
      rows.push(code(4, '},'))
    } else {
      setting('shoreline', w.shoreline === false ? 'false' : 'true')
    }
    if (w.source) setting('source', JSON.stringify(w.source))
    aboveRows(rows, n['water.batches'], 4)
    rows.push(code(4, '"batches": [', n['water.batches']?.after))
    w.batches.forEach((b, i) => {
      const base = `water.batches.${i}`
      aboveRows(rows, n[base], 6)
      rows.push(code(6, '{', n[base]?.after))
      const field = (k, v, help) => {
        aboveRows(rows, n[`${base}.${k}`], 8)
        rows.push(code(8, `"${k}": ${v},`, n[`${base}.${k}`]?.after ?? help))
      }
      if (b.name) field('name', JSON.stringify(b.name))
      field('follow', JSON.stringify(b.follow), WATER_HELP.follow)
      field('draw', JSON.stringify(b.draw ?? 'lines'), WATER_HELP.draw)
      field('angle', num(b.angle ?? 0), ANGLE_NOTE)
      aboveRows(rows, n[`${base}.style`], 8)
      rows.push(code(8, '"style": {', n[`${base}.style`]?.after))
      styleRows(rows, b.style, base, schema, n, 10, (k, v) => `"${k}": ${num(v)},`, drawKind(b))
      rows.push(code(8, '},'))
      rows.push(code(6, '},'))
    })
    rows.push(code(4, '],'))
    rows.push(code(2, '},'))
  }
  rows.push(code(0, '}'))
  return emit(rows, '//')
}

/** Tokens of a JSON5 text, comments included, each with its line number. */
function json5Tokens(text) {
  const out = []
  let i = 0
  let line = 1
  const err = (msg) => new Error(`JSON5, line ${line}: ${msg}`)
  while (i < text.length) {
    const c = text[i]
    if (c === '\n') { line++; i++; continue }
    if (/\s/.test(c)) { i++; continue }
    if (c === '/' && text[i + 1] === '/') {
      const end = text.indexOf('\n', i)
      const stop = end < 0 ? text.length : end
      out.push({ t: 'comment', v: text.slice(i + 2, stop), line })
      i = stop
      continue
    }
    if (c === '/' && text[i + 1] === '*') {
      const end = text.indexOf('*/', i + 2)
      if (end < 0) throw err('a /* comment is never closed')
      const body = text.slice(i + 2, end)
      out.push({ t: 'comment', v: body, line })
      line += body.split('\n').length - 1
      i = end + 2
      continue
    }
    if ('{}[]:,'.includes(c)) { out.push({ t: c, line }); i++; continue }
    if (c === '"' || c === "'") {
      let j = i + 1
      let s = ''
      while (j < text.length && text[j] !== c) {
        if (text[j] === '\n') throw err('a string runs past the end of the line')
        if (text[j] === '\\') {
          const e = text[j + 1]
          const map = { n: '\n', t: '\t', r: '\r', b: '\b', f: '\f', v: '\v', 0: '\0' }
          if (e === 'u') { s += String.fromCharCode(parseInt(text.slice(j + 2, j + 6), 16)); j += 6; continue }
          if (e === '\n') { line++; j += 2; continue }
          s += map[e] ?? e
          j += 2
          continue
        }
        s += text[j++]
      }
      if (j >= text.length) throw err('a string is never closed')
      out.push({ t: 'str', v: s, line })
      i = j + 1
      continue
    }
    const numMatch = /^[+-]?(?:Infinity|NaN|0[xX][0-9a-fA-F]+|(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?)/.exec(text.slice(i))
    if (numMatch && /[+\-.\d]/.test(c)) {
      const raw = numMatch[0]
      const v = /0x/i.test(raw) ? parseInt(raw, 16) : Number(raw)
      out.push({ t: 'num', v, line })
      i += raw.length
      continue
    }
    const id = /^[A-Za-z_$][\w$]*/.exec(text.slice(i))
    if (id) {
      out.push({ t: 'id', v: id[0], line })
      i += id[0].length
      continue
    }
    throw err(`unexpected "${c}"`)
  }
  return out
}

function parseJson5(text, schema) {
  const toks = json5Tokens(text)
  const book = new NoteBook(schema, (l) => {
    const m = /^"?([a-z_]+)"?\s*:\s*[^,\s]+,?(\s+\/\/.*)?$/.exec(l)
    return !!m && knownKey(schema, m[1])
  })
  let i = 0
  let pending = []
  let lastPath = null
  let lastLine = -1

  const comments = () => {
    while (toks[i]?.t === 'comment') {
      const tok = toks[i++]
      if (lastPath != null && tok.line === lastLine) book.after(lastPath, tok.v)
      else pending.push(...tok.v.split('\n').map((l) => l.replace(/^\s*\*?/, '')))
    }
  }
  const peek = () => { comments(); return toks[i] }
  const take = (t) => {
    const tok = peek()
    if (!tok) throw new Error('JSON5: the text ends too early')
    if (t && tok.t !== t) throw new Error(`JSON5, line ${tok.line}: expected "${t}"`)
    i++
    return tok
  }
  const flush = (path) => { book.above(path, pending); pending = [] }

  const value = (path) => {
    const tok = take()
    if (tok.t === '{') {
      // A comment on the opening line belongs to the object itself.
      lastPath = path || null
      lastLine = tok.line
      const obj = {}
      for (;;) {
        const k = peek()
        if (k?.t === '}') break
        const keyTok = take()
        if (keyTok.t !== 'str' && keyTok.t !== 'id') throw new Error(`JSON5, line ${keyTok.line}: expected a key`)
        const p = path ? `${path}.${keyTok.v}` : keyTok.v
        flush(p)
        take(':')
        obj[keyTok.v] = value(p)
        lastPath = p
        lastLine = toks[i - 1].line
        if (peek()?.t === ',') { take(','); continue }
        break
      }
      flush(path)
      take('}')
      return obj
    }
    if (tok.t === '[') {
      lastPath = path || null
      lastLine = tok.line
      const arr = []
      for (;;) {
        if (peek()?.t === ']') break
        const p = `${path}.${arr.length}`
        flush(p)
        arr.push(value(p))
        lastPath = p
        lastLine = toks[i - 1].line
        if (peek()?.t === ',') { take(','); continue }
        break
      }
      flush(path)
      take(']')
      return arr
    }
    if (tok.t === 'str' || tok.t === 'num') return tok.v
    if (tok.t === 'id') {
      if (tok.v === 'true') return true
      if (tok.v === 'false') return false
      if (tok.v === 'null') return null
    }
    throw new Error(`JSON5, line ${tok.line}: unexpected ${tok.v ?? tok.t}`)
  }

  const first = peek()
  if (!first) throw new Error('The document is empty')
  const root = first.t === '[' ? 'batches' : ''
  flush('')
  const result = value(root)
  lastPath = null
  comments()
  flush('')
  if (i < toks.length) throw new Error(`JSON5, line ${toks[i].line}: unexpected text after the end`)
  return toModel(result, book.notes, schema)
}

/* ================================================================= */
/*  CLI                                                               */
/* ================================================================= */

function cliValue(key, v) {
  if (key === 'size_km') return v.map((k) => `${num(k)}km`).join(' ')
  if (key === 'scale') return `1:${num(v)}`
  if (Array.isArray(v)) return v.map(num).join(' ')
  return typeof v === 'number' ? num(v) : sh(v)
}

function renderCli(model, schema) {
  const n = model.notes ?? {}
  const rows = [code(0, SHEBANG)]
  aboveRows(rows, n[''], 0)
  rows.push(blank())
  rows.push(code(0, 'args=('))

  const keys = areaKeys(model.area)
  if (keys.length) aboveRows(rows, n.area, 2)
  for (const k of keys) {
    const path = `area.${k}`
    aboveRows(rows, n[path], 2)
    rows.push(code(2, `--${AREA_FLAG[k]} ${cliValue(k, model.area[k])}`, n[path]?.after))
  }

  aboveRows(rows, n.batches, 2)
  const inherited = inheritedTerrain(model.batches, schema)
  model.batches.forEach((b, i) => {
    const base = `batches.${i}`
    aboveRows(rows, n[base], 2)
    rows.push(code(2, '--batch', n[base]?.after))
    if (b.name) {
      aboveRows(rows, n[`${base}.name`], 4)
      rows.push(code(4, `--batch-name ${sh(b.name)}`, n[`${base}.name`]?.after))
    }
    aboveRows(rows, n[`${base}.angle`], 4)
    rows.push(code(4, `--batch-angle ${num(b.angle ?? 0)}`, n[`${base}.angle`]?.after ?? ANGLE_NOTE))
    aboveRows(rows, n[`${base}.style`], 4)
    styleRows(rows, b.style, base, schema, n, 4, (k, v) => `--${k.replace(/_/g, '-')} ${num(v)}`, 'land', inherited[i])
  })
  if (model.water) {
    aboveRows(rows, n.water, 2)
    rows.push(code(2, `--water ${sh(JSON.stringify(model.water))}`, n.water?.after ?? WATER_HELP.water))
  }
  rows.push(code(0, ')'))
  rows.push(code(0, 'burin-map "${args[@]}"'))
  return emit(rows, '#')
}

/** Split one line into shell words; a word starting with `#` begins a comment. */
function shellWords(line, lineNo) {
  const words = []
  let cur = null
  let i = 0
  const err = (msg) => new Error(`CLI, line ${lineNo}: ${msg}`)
  while (i < line.length) {
    const c = line[i]
    if (c === ' ' || c === '\t') {
      if (cur != null) { words.push(cur); cur = null }
      i++
    } else if (c === '#' && cur == null) {
      return { words, comment: line.slice(i + 1) }
    } else if (c === "'") {
      const j = line.indexOf("'", i + 1)
      if (j < 0) throw err('a quote is never closed (keep quoted text on one line)')
      cur = (cur ?? '') + line.slice(i + 1, j)
      i = j + 1
    } else if (c === '"') {
      let j = i + 1
      let s = ''
      while (j < line.length && line[j] !== '"') {
        if (line[j] === '\\' && j + 1 < line.length) { s += line[j + 1]; j += 2 } else s += line[j++]
      }
      if (j >= line.length) throw err('a quote is never closed (keep quoted text on one line)')
      cur = (cur ?? '') + s
      i = j + 1
    } else if (c === '\\') {
      if (i < line.length - 1) cur = (cur ?? '') + line[i + 1]
      i += 2
    } else {
      cur = (cur ?? '') + c
      i++
    }
  }
  if (cur != null) words.push(cur)
  return { words, comment: null }
}

const CLI_AREA = Object.fromEntries(
  Object.entries(AREA_FLAG).flatMap(([key, long]) => [[long, key]]),
)
Object.assign(CLI_AREA, { tp: 'place', tat: 'at', tbo: 'bounds', ts: 'size_km', fs: 'size_mm', esc: 'scale', pen: 'pen_mm' })

const CLI_IGNORED = new Set(['out-file', 'of', 'out-format', 'fmt', 'out-geojson'])

/** `20km`, `800m`, `5mi`, or a bare number of km. */
function lengthKm(s, flag) {
  const m = /^(\d*\.?\d+)(km|m|mi)?$/.exec(s)
  if (!m) throw new Error(`${flag}: "${s}" is not a length like 20km, 800m or 5mi`)
  const v = Number(m[1])
  return m[2] === 'm' ? v / 1000 : m[2] === 'mi' ? v * 1.609344 : v
}

function parseCli(text, schema) {
  const book = new NoteBook(schema, (l) => {
    const m = /^--([a-z-]+)\s+\S+(\s+#.*)?$/.exec(l)
    if (!m) return false
    const k = m[1].replace(/-/g, '_')
    return knownKey(schema, k) || k === 'batch_angle' || !!SHORT_PARAMS[m[1]]
  })

  // Words with the comments that belong to them. Comments before `args=(`
  // (or before the first flag) are the top of the document.
  const words = []
  const header = []
  let pending = []
  let inBody = false
  const lines = text.split('\n')
  lines.forEach((raw, idx) => {
    if (idx === 0 && raw.startsWith('#!')) return
    const { words: ws, comment: note } = shellWords(raw, idx + 1)
    if (ws.some((w) => w === 'burin-map' || w.startsWith('args=('))) inBody = true
    const kept = ws.filter((w) => w !== ')' && w !== 'burin-map' && w !== '${args[@]}' && w !== 'args=(')
      .map((w) => (w.startsWith('args=(') ? w.slice(6) : w))
      .filter((w) => w !== '')
    if (kept.length === 0) {
      if (note != null) (inBody ? pending : header).push(note)
      return
    }
    inBody = true
    const flags = kept.map((w, j) => (w.startsWith('-') && !/^-[\d.]/.test(w) ? j : -1)).filter((j) => j >= 0)
    const start = words.length
    kept.forEach((w) => words.push({ w, line: idx + 1 }))
    if (flags.length) {
      words[start + flags[0]].above = pending
      pending = []
      if (note != null) words[start + flags[flags.length - 1]].after = note
    }
  })

  book.above('', [...header, ...pending])

  const area = {}
  const shared = { style: {}, notes: {} }
  const batches = []
  let water = null
  let cur = null   // the batch being read; null before the first --batch

  const noteFor = (path, word) => {
    book.above(path, word.above)
    book.after(path, word.after)
  }
  const batchPath = (key) => (cur ? `batches.${batches.length - 1}.${key}` : null)

  for (let i = 0; i < words.length; i++) {
    const word = words[i]
    const w = word.w
    if (!w.startsWith('-')) throw new Error(`CLI, line ${word.line}: "${w}" is not a flag`)
    const name = w.replace(/^--?/, '')
    const values = (count, flag) => {
      const got = words.slice(i + 1, i + 1 + count).map((x) => x.w)
      if (got.length < count) throw new Error(`CLI, line ${word.line}: ${flag} needs ${count} value${count > 1 ? 's' : ''}`)
      i += count
      return got
    }
    const upTo2 = () => {
      const got = []
      while (got.length < 2 && words[i + 1] && !words[i + 1].w.startsWith('-')) got.push(words[++i].w)
      if (got.length === 0) throw new Error(`CLI, line ${word.line}: ${w} needs 1 or 2 values`)
      return got
    }
    const number = (s, flag) => {
      const v = Number(s)
      if (s === '' || !Number.isFinite(v)) throw new Error(`CLI, line ${word.line}: ${flag} needs a number, not "${s}"`)
      return v
    }

    if (name === 'batch' || w === '-b') {
      batches.push({ angle: 0, style: {} })
      cur = batches[batches.length - 1]
      noteFor(`batches.${batches.length - 1}`, word)
      continue
    }
    if (name in CLI_AREA) {
      const key = CLI_AREA[name]
      if (key === 'size_km') area.size_km = upTo2().map((s) => lengthKm(s, w))
      else if (key === 'size_mm') area.size_mm = upTo2().map((s) => number(s, w))
      else if (key === 'bounds') area.bounds = values(4, w).map((s) => number(s, w))
      else if (key === 'scale') {
        const [s] = values(1, w)
        const m = /^(?:1:)?(\d*\.?\d+)$/.exec(s)
        if (!m) throw new Error(`CLI, line ${word.line}: ${w} needs a scale like 1:25000, not "${s}"`)
        area.scale = Number(m[1])
      } else if (key === 'pen_mm') area.pen_mm = number(values(1, w)[0], w)
      else area[key] = values(1, w)[0]
      noteFor(`area.${key}`, word)
      continue
    }
    if (CLI_IGNORED.has(name)) { values(1, w); continue }
    if (name === 'water') {
      const [text] = values(1, w)
      try {
        water = JSON.parse(text)
      } catch {
        throw new Error(`CLI, line ${word.line}: --water needs the water block as JSON, like '{"batches": []}'`)
      }
      noteFor('water', word)
      continue
    }
    if (name === 'style-file' || name === 'sf' || name === 'style-json' || name === 'sj') {
      throw new Error(`CLI, line ${word.line}: ${w} can't be used here; paste the document in the YAML or JSON5 view instead`)
    }
    const target = cur ?? shared
    if (name === 'batch-name' || name === 'bn') {
      target.name = values(1, w)[0]
      if (cur) noteFor(batchPath('name'), word); else shared.notes.name = word
      continue
    }
    if (name === 'batch-angle' || name === 'ba') {
      target.angle = number(values(1, w)[0], w)
      if (cur) noteFor(batchPath('angle'), word); else shared.notes.angle = word
      continue
    }
    const key = SHORT_PARAMS[name] ?? name.replace(/-/g, '_')
    if (!paramByName(schema, key)) throw new Error(`CLI, line ${word.line}: unknown flag ${w}`)
    target.style[key] = number(values(1, w)[0], w)
    if (cur) noteFor(batchPath(`style.${key}`), word); else shared.notes[`style.${key}`] = word
  }

  if (batches.length === 0) {
    batches.push({ ...(shared.name ? { name: shared.name } : {}), angle: shared.angle ?? 0, style: {} })
  }
  for (const b of batches) b.style = { ...shared.style, ...b.style }
  for (const [key, word] of Object.entries(shared.notes)) noteFor(`batches.0.${key}`, word)
  return toModel({ ...(Object.keys(area).length ? { area } : {}), batches, ...(water ? { water } : {}) }, book.notes, schema)
}

/* ================================================================= */
/*  Model checks and the public entry points                          */
/* ================================================================= */

/** A style block: known parameters with numbers only. */
function readStyle(src, which, schema) {
  const style = {}
  if (src == null) return style
  if (typeof src !== 'object' || Array.isArray(src)) throw new Error(`The style of ${which} must be a block of parameters`)
  for (const [k, v] of Object.entries(src)) {
    if (!paramByName(schema, k)) throw new Error(`Unknown parameter "${k}" in ${which}`)
    if (typeof v !== 'number' || !Number.isFinite(v)) throw new Error(`"${k}" in ${which} must be a number`)
    style[k] = v
  }
  return style
}

function readWater(w, schema) {
  if (w == null) return null
  if (typeof w !== 'object' || Array.isArray(w)) throw new Error('"water" must be a block of settings')
  for (const k of Object.keys(w)) {
    if (!['classes', 'shoreline', 'source', 'batches'].includes(k)) {
      throw new Error(`Unknown setting "${k}" in water: use classes, shoreline, source and batches`)
    }
  }
  const out = {}
  if (w.classes != null) {
    if (!Array.isArray(w.classes)) throw new Error('water.classes must be a list, like [ocean, lake]')
    for (const c of w.classes) {
      if (!WATER_CLASSES.includes(c)) throw new Error(`Unknown water class "${c}". Allowed: ${WATER_CLASSES.join(', ')}`)
    }
    out.classes = w.classes.map(String)
  }
  if (w.shoreline === false) out.shoreline = false
  else if (w.shoreline != null && typeof w.shoreline === 'object' && !Array.isArray(w.shoreline)) {
    for (const k of Object.keys(w.shoreline)) {
      if (!['lines', 'style'].includes(k)) throw new Error(`Unknown setting "${k}" in water.shoreline: use lines and style`)
    }
    const lines = w.shoreline.lines == null ? 1 : Number(w.shoreline.lines)
    if (!Number.isInteger(lines) || lines < 1) throw new Error('water.shoreline.lines must be a whole number, 1 or more')
    out.shoreline = { lines, style: readStyle(w.shoreline.style, 'the shoreline', schema) }
  } else if (w.shoreline != null && w.shoreline !== true) {
    throw new Error('water.shoreline is true, false, or a block with lines and a style')
  }
  if (w.source != null) out.source = String(w.source)
  if (w.batches != null && !Array.isArray(w.batches)) throw new Error('water.batches must be a list')
  out.batches = (w.batches ?? []).map((b, i) => {
    const which = `water batch ${i + 1}`
    if (!b || typeof b !== 'object' || Array.isArray(b)) throw new Error(`${which} must be a block of settings`)
    for (const k of Object.keys(b)) {
      if (!['name', 'follow', 'draw', 'angle', 'style'].includes(k)) {
        throw new Error(`Unknown setting "${k}" in ${which}: use name, follow, draw, angle and style`)
      }
    }
    const follow = b.follow ?? 'shore'
    if (follow !== 'shore' && follow !== 'map') throw new Error(`follow in ${which} is "shore" or "map", not "${follow}"`)
    const draw = b.draw ?? 'lines'
    if (!WATER_DRAWS.includes(draw)) throw new Error(`draw in ${which} is ${WATER_DRAWS.map((d) => `"${d}"`).join(', ')}, not "${draw}"`)
    const angle = b.angle == null ? 0 : Number(b.angle)
    if (!Number.isFinite(angle)) throw new Error(`The angle of ${which} must be a number`)
    const style = readStyle(b.style, which, schema)
    const name = b.name != null && String(b.name).trim() ? String(b.name) : ''
    const batch = { follow, ...(draw !== 'lines' ? { draw } : {}), angle, style }
    return name ? { name, ...batch } : batch
  })
  return out
}

function toModel(value, notes, schema) {
  let doc = Array.isArray(value) ? { batches: value } : value
  if (!doc || typeof doc !== 'object') throw new Error('Expected an area block and a list of batches')
  for (const key of Object.keys(doc)) {
    if (!['area', 'batches', 'land', 'water'].includes(key)) {
      throw new Error(`Unknown setting "${key}": only "area", "batches" (or "land") and "water" go at the top`)
    }
  }
  if (doc.land != null) {
    if (doc.batches != null) throw new Error('Give the land\'s batches either as "batches" or as "land.batches", not both')
    if (typeof doc.land !== 'object' || Array.isArray(doc.land)) throw new Error('"land" must be a block with its batches')
    for (const k of Object.keys(doc.land)) {
      if (k !== 'batches') throw new Error(`Unknown setting "${k}" in land: only batches goes there`)
    }
    const { land, ...rest } = doc
    doc = { ...rest, batches: land.batches }
    notes = Object.fromEntries(Object.entries(notes).map(([p, n]) => [p.replace(/^land\.batches/, 'batches'), n]))
  }

  const area = {}
  if (doc.area != null) {
    if (typeof doc.area !== 'object' || Array.isArray(doc.area)) throw new Error('"area" must be a block of settings')
    for (const [k, v] of Object.entries(doc.area)) {
      if (!AREA_KEYS.includes(k)) throw new Error(`Unknown area setting "${k}". Allowed: ${AREA_KEYS.join(', ')}`)
      if (v == null) continue
      area[k] = k === 'at' || k === 'place' ? String(v) : v
    }
  }

  if (!Array.isArray(doc.batches) || doc.batches.length === 0) throw new Error('The document needs at least one batch')
  const batches = doc.batches.map((b, i) => {
    const which = `batch ${i + 1}`
    if (!b || typeof b !== 'object' || Array.isArray(b)) throw new Error(`${which} must be a block of settings`)
    for (const k of Object.keys(b)) {
      if (!['name', 'angle', 'style'].includes(k)) throw new Error(`Unknown setting "${k}" in ${which}: use name, angle and style`)
    }
    const angle = b.angle == null ? 0 : Number(b.angle)
    if (!Number.isFinite(angle)) throw new Error(`The angle of ${which} must be a number`)
    const style = readStyle(b.style, which, schema)
    const name = typeof b.name === 'string' && b.name.trim() ? b.name : b.name != null ? String(b.name) : ''
    return name ? { name, angle, style } : { angle, style }
  })
  return { area, batches, water: readWater(doc.water, schema), notes }
}

/** Write the model in one of the three views. */
export function renderView(kind, model, schema) {
  if (kind === 'json5') return renderJson5(model, schema)
  if (kind === 'cli') return renderCli(model, schema)
  return renderYaml(model, schema)
}

/** Read a view back into a model. Throws with a readable message. */
export function parseView(kind, text, schema) {
  if (kind === 'json5') return parseJson5(text, schema)
  if (kind === 'cli') return parseCli(text, schema)
  return parseYaml(text, schema)
}

/** Which view a file holds, from its name, then its content. */
export function detectView(fileName, text) {
  const ext = String(fileName || '').toLowerCase().split('.').pop()
  if (ext === 'yaml' || ext === 'yml') return 'yaml'
  if (ext === 'json' || ext === 'json5') return 'json5'
  if (ext === 'sh') return 'cli'
  const t = String(text || '').trimStart()
  if (/^(\{|\[|\/\/|\/\*)/.test(t)) return 'json5'
  if (/^(#!|burin-map\b|args=\()/.test(t) || (/(^|\s)--[a-z]/.test(t) && !/^\s*[a-z_]+:/m.test(t))) return 'cli'
  return 'yaml'
}