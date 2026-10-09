/**
 * The document, as this page runs it.
 *
 * A burin-map document is a JavaScript script that calls `burin.*` and ends
 * with `burin.draw()`. The interpreter (`pkg-script`, over Boa) runs it and
 * hands back the *atlas*: the batches and water passes the engine draws, in
 * order. The page holds the script text and keeps the atlas it produced; it
 * never keeps a copy of the drawing in some other shape, so there is nothing
 * to convert and nothing to drift.
 *
 * Two directions only:
 *   script (text) → atlas   by running it (`runJs`)
 *   atlas          → engine  the batches and the water spec, as the worker takes them
 *
 * `burin.area({…})` is the one part the page reads directly: where to put the
 * map and the paper. The rest of the document is the script's own.
 *
 * The compact `#d=…` share link still carries a flat atlas (a script with a
 * loop has no flat form, so a link is not a script); `atlasToScript` writes
 * that atlas back out as a document, which is what the editor then shows.
 */

import { pickLand } from './fill.js'
import { pickInk } from './ink.js'

/* ---------------------------------------------------------------- */
/*  The interpreter                                                   */
/* ---------------------------------------------------------------- */

/** The script WASM module, loaded the first time a document has to run. */
let scriptModule = null

// Function so Vite does not rewrite the import of a file in /public.
const importPkg = new Function('url', 'return import(url)')

async function loadScript() {
  scriptModule ??= (async () => {
    const m = await importPkg('/pkg-script/burin_map_script_wasm.js')
    await m.default()
    return m
  })()
  return scriptModule
}

/** The library's source, so an editor can offer it beside the types. */
export async function librarySource() {
  const m = await loadScript()
  return m.librarySource()
}

/**
 * Run a document and return `{ atlas, area }`: the list of batches and water
 * passes the engine draws, and the area the script set (or `null`).
 * @param {string} source
 * @returns {Promise<{ atlas: Array<object>, area: object | null }>}
 */
export async function runJs(source) {
  const m = await loadScript()
  try {
    const parsed = JSON.parse(m.evaluate(source))
    // Older wasm builds returned the atlas array only; the Node binding returns
    // `{ atlas, area }`.
    if (Array.isArray(parsed)) {
      return { atlas: parsed, area: null }
    }
    return {
      atlas: parsed?.atlas ?? [],
      area: parsed?.area ?? null,
    }
  } catch (e) {
    // The interpreter's errors carry `line`/`column`; a readable prefix helps
    // the editor point at the mistake.
    const at = e && Number.isFinite(e.line) ? `JS, line ${e.line}${Number.isFinite(e.column) ? ` column ${e.column}` : ''}: ` : ''
    throw new Error(at + (e?.message ?? String(e)))
  }
}

/* ---------------------------------------------------------------- */
/*  atlas → engine                                                    */
/* ---------------------------------------------------------------- */

/**
 * The land batches of an atlas, as the drawing worker takes them.
 * @param {Array<object>} atlas
 */
export function atlasBatches(atlas) {
  return (atlas ?? []).filter((i) => i.kind !== 'water' && i.kind !== 'shore').map((b) => ({
    ...(typeof b?.name === 'string' && b.name.trim() ? { name: b.name } : {}),
    ...pickInk(b ?? {}),
    angle: Number.isFinite(Number(b?.angle)) ? Number(b.angle) : 0,
    ...pickLand(b),
    style: numericStyle(b?.style),
  }))
}

/**
 * The water of an atlas as the worker's water spec, or `null` when the atlas
 * has no water at all. The spec is what the engine reads: the passes, and the
 * shoreline (present, `false`, or a block with its lines and ink).
 * @param {Array<object>} atlas
 */
export function atlasWater(atlas) {
  const items = atlas ?? []
  const passes = items.filter((i) => i.kind === 'water')
  const shore = items.find((i) => i.kind === 'shore')
  if (!passes.length && !shore) return null
  return {
    ...(shore
      ? { shoreline: { lines: shore.lines ?? 1, ...inkOnly(shore), style: numericStyle(shore.style) } }
      : { shoreline: false }),
    batches: passes.map((b) => ({
      ...(typeof b?.name === 'string' && b.name.trim() ? { name: b.name } : {}),
      ...inkOnly(b),
      follow: b.follow ?? 'shore',
      ...(b.draw && b.draw !== 'lines' ? { draw: b.draw } : {}),
      angle: Number.isFinite(Number(b?.angle)) ? Number(b.angle) : 0,
      ...(b.where ? { where: waterWhere(b.where) } : {}),
      style: numericStyle(b.style),
    })),
  }
}

/** A batch's whole style, defaults and all; the worker keeps the finite numbers. */
function numericStyle(style) {
  return Object.fromEntries(
    Object.entries(style ?? {}).filter(([, v]) => typeof v === 'number' && Number.isFinite(v)),
  )
}

/** The ink keys of an atlas entry, without its style. */
function inkOnly(b) {
  return pickInk({ layer: b.layer, color: b.color, color_expr: b.color_expr ?? b.shade, width_expr: b.width_expr ?? b.width, width_steps: b.width_steps, cap: b.cap })
}

/** A water pass's `where`, as the engine reads it (snake_case, no fade here). */
function waterWhere(w) {
  const pick = (camel, snake) => w[camel] ?? w[snake]
  return {
    ...(pick('depthMin', 'depth_min') != null ? { depth_min: pick('depthMin', 'depth_min') } : {}),
    ...(pick('depthMax', 'depth_max') != null ? { depth_max: pick('depthMax', 'depth_max') } : {}),
    ...(pick('depthFade', 'depth_fade') != null ? { depth_fade: pick('depthFade', 'depth_fade') } : {}),
  }
}

/* ---------------------------------------------------------------- */
/*  atlas → script                                                    */
/* ---------------------------------------------------------------- */

/** A JS string literal. */
const js = (s) => JSON.stringify(String(s))

/** A number for a document: no float noise, no scientific notation. */
const num = (v) => String(Number(Number(v).toFixed(6)))

/** A style block as JS, or `null` when it is empty. */
function styleLiteral(style) {
  const entries = Object.entries(style ?? {}).filter(([, v]) => typeof v === 'number' && Number.isFinite(v))
  if (!entries.length) return null
  return `{ ${entries.map(([k, v]) => `${k}: ${num(v)}`).join(', ')} }`
}

/** The band a fill keeps to, as the library's camelCase. */
function whereLiteral(w) {
  const map = { elev_min: 'elevMin', elev_max: 'elevMax', depth_min: 'depthMin', depth_max: 'depthMax' }
  const parts = Object.entries(map).flatMap(([snake, camel]) => {
    const pick = w?.[camel] ?? w?.[snake]
    return pick == null ? [] : [`${camel}: ${num(pick)}`]
  })
  return parts.length ? `{ ${parts.join(', ')} }` : null
}

/** The land batch of an atlas item, as a `burin.batch({…})` argument. */
function batchArgs(b) {
  const parts = []
  if (b.name) parts.push(`name: ${js(b.name)}`)
  if (b.ground) parts.push(`ground: ${js(b.ground)}`)
  if (b.draw) parts.push(`draw: ${js(b.draw)}`)
  if (b.color) parts.push(`color: ${js(b.color)}`)
  if (b.color_expr ?? b.shade) parts.push(`shade: ${js(b.color_expr ?? b.shade)}`)
  if (b.width_expr ?? b.width) parts.push(`width: ${js(b.width_expr ?? b.width)}`)
  if (b.width_steps) parts.push(`widthSteps: ${b.width_steps}`)
  const where = whereLiteral(b.where)
  if (where) parts.push(`where: ${where}`)
  if (b.angle) parts.push(`angle: ${num(b.angle)}`)
  const style = styleLiteral(b.style)
  if (style) parts.push(`style: ${style}`)
  return parts.join(', ')
}

/** A water pass, as a `burin.water(kind, {…})` argument. */
function waterArgs(b) {
  const parts = []
  if (b.follow && b.follow !== 'shore') parts.push(`follow: ${js(b.follow)}`)
  if (b.color) parts.push(`color: ${js(b.color)}`)
  if (b.color_expr ?? b.shade) parts.push(`shade: ${js(b.color_expr ?? b.shade)}`)
  if (b.width_expr ?? b.width) parts.push(`width: ${js(b.width_expr ?? b.width)}`)
  const where = whereLiteral({ depthMin: b.where?.depthMin ?? b.where?.depth_min, depthMax: b.where?.depthMax ?? b.where?.depth_max })
  if (where) parts.push(`where: ${where}`)
  if (b.angle) parts.push(`angle: ${num(b.angle)}`)
  const style = styleLiteral(b.style)
  if (style) parts.push(`style: ${style}`)
  return parts.join(', ')
}

/** The area block as the `burin.area({…})` argument. */
function areaArgs(area) {
  if (!area) return ''
  const parts = []
  for (const k of ['place', 'at', 'bounds', 'size_km', 'size_mm', 'scale', 'pen_mm']) {
    const v = area[k]
    if (v == null) continue
    parts.push(`${k}: ${Array.isArray(v) ? `[${v.map(num).join(', ')}]` : typeof v === 'number' ? num(v) : js(v)}`)
  }
  return parts.join(', ')
}

/**
 * Write an atlas back out as a document script. A flat atlas (from a share
 * link) becomes the flat script the other front ends can read; a script with
 * a loop is not round-tripped, since an atlas has no loops to recover.
 * @param {Array<object>} atlas
 * @param {object | null} [area]
 */
export function atlasToScript(atlas, area = null) {
  const lines = ['/**', ' * A burin-map document in JavaScript.', ' *', ' * Run it: the batches and water passes below are drawn in order.', ' */', '']
  const a = areaArgs(area)
  if (a) lines.push(`burin.area({ ${a} })`, '')

  for (const item of atlas ?? []) {
    if (item.kind === 'water') {
      const kind = item.draw ?? 'lines'
      lines.push(`burin.water(${js(kind)}, { ${waterArgs(item)} })`)
    } else if (item.kind === 'shore') {
      const parts = []
      if (item.lines != null && item.lines !== 1) parts.push(`lines: ${item.lines}`)
      if (item.color) parts.push(`color: ${js(item.color)}`)
      const style = styleLiteral(item.style)
      if (style) parts.push(`style: ${style}`)
      lines.push(`burin.shore({ ${parts.join(', ')} })`)
    } else {
      lines.push(`burin.batch({ ${batchArgs(item)} })`)
    }
  }

  lines.push('', 'burin.draw()', '')
  return lines.join('\n')
}

/* ---------------------------------------------------------------- */
/*  atlas ↔ the compact share-link document                           */
/* ---------------------------------------------------------------- */

/**
 * An atlas as the flat document the `#d=…` notation carries: `{ area,
 * batches, water }`. The notation has no loops, so this is the flat form of
 * the drawing, good enough to open a link (and then shown as a script).
 * @param {Array<object>} atlas
 * @param {object} [area]
 */
export function atlasToDoc(atlas, area = {}) {
  const items = atlas ?? []
  const batches = items.filter((i) => i.kind !== 'water' && i.kind !== 'shore')
  const passes = items.filter((i) => i.kind === 'water')
  const shore = items.find((i) => i.kind === 'shore')
  const water = passes.length || shore
    ? {
        ...(shore ? { shoreline: { lines: shore.lines ?? 1, ...inkOnly(shore), style: numericStyle(shore.style) } } : {}),
        batches: passes.map((b) => ({
          follow: b.follow ?? 'shore',
          ...(b.draw && b.draw !== 'lines' ? { draw: b.draw } : {}),
          ...(b.name ? { name: b.name } : {}),
          angle: Number(b.angle ?? 0),
          ...inkOnly(b),
          ...(b.where ? { where: waterWhere(b.where) } : {}),
          style: numericStyle(b.style),
        })),
      }
    : null
  return { area: area ?? {}, batches, water }
}

/**
 * The flat document a `#d=…` link carried, as an atlas again: the water
 * passes and the shore get their `kind` back, so the engine and the script
 * writer read them like anything else.
 * @param {{ area?: object, batches?: Array<object>, water?: object | null }} doc
 */
export function docToAtlas(doc) {
  const out = [...(doc.batches ?? [])]
  if (doc.water) {
    for (const b of doc.water.batches ?? []) out.push({ ...b, kind: 'water', draw: b.draw ?? 'lines' })
    if (doc.water.shoreline && doc.water.shoreline !== false) {
      out.push({ kind: 'shore', ...(typeof doc.water.shoreline === 'object' ? doc.water.shoreline : {}) })
    }
  }
  return out
}
