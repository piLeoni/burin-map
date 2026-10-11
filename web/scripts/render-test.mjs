/**
 * DOM smoke test for the Sidebar — the JS-only document front end. Runs the
 * real component with jsdom so effects fire (WASM is stubbed) and checks the
 * document layer that sits between a script and the engine.
 *
 * A burin-map document *is* a JavaScript script (see docs/front-ends.md). The
 * page holds the script text and the atlas it built; there is no YAML/JSON5/
 * CLI view and nothing to convert between formats. What this test guards is
 * that shape: the editor, the presets, and `document.js` (atlas → engine,
 * atlas → script, atlas → share link).
 *
 * Run: npm run test:render
 */
import { JSDOM } from 'jsdom'

const dom = new JSDOM('<!doctype html><html><body><div id="root"></div></body></html>', {
  url: 'http://localhost/',
})
globalThis.window = dom.window
globalThis.document = dom.window.document
Object.defineProperty(globalThis, 'navigator', { value: dom.window.navigator, configurable: true })
globalThis.HTMLElement = dom.window.HTMLElement
globalThis.getComputedStyle = dom.window.getComputedStyle.bind(dom.window)
globalThis.requestAnimationFrame = (cb) => setTimeout(() => cb(Date.now()), 0)
globalThis.cancelAnimationFrame = clearTimeout
globalThis.matchMedia = () => ({ matches: false, addEventListener() {}, removeEventListener() {} })
globalThis.localStorage = dom.window.localStorage
globalThis.location = dom.window.location
globalThis.IS_REACT_ACT_ENVIRONMENT = true
// The editor is Monaco; the Sidebar only needs something that renders a box.
globalThis.ResizeObserver = class { observe() {} unobserve() {} disconnect() {} }

const { createElement } = await import('react')
const { createRoot } = await import('react-dom/client')
const { act } = await import('react')
const { Provider, createStore } = await import('jotai')
const { editorAtom, atlasAtom } = await import('../src/state.js')

const errors = []
const origError = console.error
console.error = (...args) => { errors.push(args.join(' ')); origError(...args) }

const { default: Sidebar } = await import('../src/components/Sidebar.jsx')
const { PRESETS, START_SCRIPT } = await import('../src/lib/presets.js')
const D = await import('../src/lib/document.js')

const missing = []

// ---------- The presets are real scripts, each with a precomputed atlas ----------
if (PRESETS.length < 4) missing.push(`expected several presets, got ${PRESETS.length}`)
for (const p of PRESETS) {
  if (!p.doc || !/burin\./.test(p.doc)) missing.push(`preset ${p.id} is not a script`)
  if (!/burin\.draw\s*\(/.test(p.doc)) missing.push(`preset ${p.id} never calls burin.draw()`)
  if (!Array.isArray(p.atlas)) missing.push(`preset ${p.id} has no precomputed atlas`)
}
// The document the app opens on is the first preset.
if (START_SCRIPT !== PRESETS[0].doc) missing.push('START_SCRIPT should be the first preset')

// ---------- The Sidebar renders the JS-only UI ----------
const store = createStore()
store.set(editorAtom, START_SCRIPT)
store.set(atlasAtom, PRESETS[0].atlas)

const root = createRoot(document.getElementById('root'))
await act(async () => {
  root.render(
    createElement(
      Provider,
      { store },
      createElement(Sidebar, {
        open: true,
        onClose: () => {},
        status: { phase: 'ready' },
        drawing: { lines: 10, dashes: 12, ms: 5 },
      }),
    ),
  )
})
// let any pending promise resolve and the resulting state flush
await act(async () => { await new Promise((r) => setTimeout(r, 50)) })

const html = document.getElementById('root').innerHTML
const checks = ['presets ▾', 'import', 'download', 'share', 'Document', 'github', 'apply', 'revert', 'copy']
const gone = missing.concat(checks.filter((c) => !html.includes(c)))
missing.length = 0
missing.push(...gone)
// The four-format tabs are exactly what was removed.
for (const stale of ['>YAML<', '>JSON5<', '>CLI<', 'Parameters']) {
  if (html.includes(stale)) missing.push(`the removed view ${stale} is still rendered`)
}

// ---------- document.js: atlas → engine ----------
// Land batches come out as the worker takes them; water passes and the shore
// are split off into the water spec.
const atlas = [
  { angle: 90, style: { space_light: 6, dash_light: 0.2 }, name: 'ridges' },
  { kind: 'water', follow: 'shore', angle: 0, style: { water_reach: 80 }, name: 'coast' },
  { kind: 'water', follow: 'map', draw: 'dots', angle: 10, style: { water_size: 18 } },
  { kind: 'shore', lines: 2, color: '#123456', style: { dash_dark: 0.5 } },
  { angle: 0, draw: 'fill', ground: 'sea', where: { depth_min: 200 }, color: '#aaccee', style: {} },
]
const land = D.atlasBatches(atlas)
if (land.length !== 2) missing.push(`atlasBatches should keep 2 land batches, got ${land.length}`)
if (land[0].name !== 'ridges' || land[0].angle !== 90 || land[0].style.space_light !== 6) missing.push('atlasBatches: first batch')
if (land[1].draw !== 'fill' || land[1].ground !== 'sea' || land[1].color !== '#aaccee') missing.push('atlasBatches: sea fill')
if (!land[1].where || land[1].where.depth_min !== 200) missing.push('atlasBatches: fill where')

const water = D.atlasWater(atlas)
if (!water || water.batches.length !== 2) missing.push('atlasWater: two passes')
if (water.shoreline === false || water.shoreline.lines !== 2 || water.shoreline.color !== '#123456') missing.push('atlasWater: shoreline block')
if (water.batches[0].follow !== 'shore' || water.batches[0].name !== 'coast') missing.push('atlasWater: first pass')
if (water.batches[1].follow !== 'map' || water.batches[1].draw !== 'dots') missing.push('atlasWater: second pass')
// No water at all → null, so the engine draws a dry map.
if (D.atlasWater([{ angle: 0, style: {} }]) !== null) missing.push('atlasWater should be null without water')

// ---------- document.js: atlas → script (and back) ----------
const script = D.atlasToScript(atlas, { at: '46,7.7', size_km: [8, 6] })
if (!script.includes('burin.area(')) missing.push('atlasToScript: area')
if (!script.includes('burin.batch(')) missing.push('atlasToScript: batch')
if (!script.includes('burin.water(')) missing.push('atlasToScript: water')
if (!script.includes('burin.shore(')) missing.push('atlasToScript: shore')
if (!script.trimEnd().endsWith('burin.draw()')) missing.push('atlasToScript should end with burin.draw()')
// The interpreter the page runs is stubbed in this test; check the text only.

// ---------- document.js: atlas ↔ the compact share link document ----------
const flat = D.atlasToDoc(atlas, { at: '46,7.7', size_km: [8, 6] })
if (flat.batches.length !== 2) missing.push('atlasToDoc: land batches')
if (!flat.water || flat.water.batches.length !== 2) missing.push('atlasToDoc: water passes')
if (!flat.water.shoreline || flat.water.shoreline.lines !== 2) missing.push('atlasToDoc: shoreline')
const roundTrip = D.docToAtlas(flat)
const landAgain = D.atlasBatches(roundTrip)
if (JSON.stringify(landAgain[0]) !== JSON.stringify(land[0])) missing.push('docToAtlas: land batch round-trip')
if (D.atlasWater(roundTrip).batches.length !== 2) missing.push('docToAtlas: water round-trip')

// The whole atlas → doc → link → doc → atlas journey a share link makes. The
// compact notation is structure only (no ink colour, no `where`), so the two
// sides are compared after the same lossy step, which is what a link gives.
const { docToHash, docFromHash, parseDoc, noteToDoc } = await import('../src/lib/doc.js')
const back = docFromHash(docToHash(flat))
if (JSON.stringify(D.docToAtlas(back)) !== JSON.stringify(D.docToAtlas(docFromHash(docToHash(D.atlasToDoc(roundTrip)))))) {
  missing.push('a share link loses part of the document')
}

// ---------- The compact notation still reads and writes ----------
const { PARAM_SHORTS } = await import('../src/lib/shorts.js')
const doc = {
  area: { at: '37.75111,-122.44722', size_mm: [210, 297], scale: 25000, pen_mm: 0.25 },
  batches: [{ angle: 90, style: { space_light: 6, dash_light: 0.2, batch_floor: 0.45 } }],
}
const hash = docToHash(doc)
if (hash.startsWith('#d=')) {
  const b = docFromHash(hash)
  if (b.batches[0].angle !== 90) missing.push('notation angle')
  if (b.batches[0].style.space_light !== 6) missing.push('notation space')
  if (b.batches[0].style.batch_floor !== 0.45) missing.push('notation batch_floor')
  if (b.area?.size_mm?.[1] !== 297) missing.push('notation area size_mm')
  if (b.area?.scale !== 25000) missing.push('notation area scale')
  if (b.area?.at !== '37.75111,-122.44722') missing.push('notation area at (comma in value)')
  if (docFromHash(docToHash({ area: { size_km: [8, 6] }, batches: doc.batches })).area?.size_km?.[1] !== 6) {
    missing.push('notation size_km')
  }
} else {
  missing.push('hash should use the #d= notation')
}

// Water rides in a third section, and survives it exactly.
const wet = {
  ...doc,
  water: {
    batches: [
      { name: 'coast; waves', follow: 'shore', angle: 0, style: { space_light: 5, water_reach: 80 } },
      { follow: 'map', angle: 70, style: {} },
      { follow: 'shore', draw: 'dots', angle: 10, style: { space_light: 50, water_size: 18 } },
    ],
    shoreline: false,
    classes: ['ocean', 'lake'],
    source: 'https://example.org/tiles.json',
  },
}
const wetBack = docFromHash(docToHash(wet))
if (JSON.stringify(wetBack) !== JSON.stringify(wet)) missing.push(`notation water round-trip ${JSON.stringify(wetBack)}`)
const coast = {
  ...doc,
  water: {
    batches: [{ follow: 'shore', angle: 0, style: {} }],
    classes: ['ocean'],
    shoreline: { lines: 2, style: { dash_dark: 0.5, mod_wobble: 20, batch_stroke: 2 } },
  },
}
const coastBack = docFromHash(docToHash(coast))
if (JSON.stringify(coastBack) !== JSON.stringify(coast)) missing.push(`notation shoreline round-trip ${JSON.stringify(coastBack)}`)

// Batch names survive the link, whatever characters they hold.
const named = {
  area: { at: '37.75,-122.44' },
  batches: [
    { name: 'north, ridges; 50% | "steep"', angle: 0, style: { space_light: 5 } },
    { name: 'Vallées ñ', angle: 90, style: { space_light: 4 } },
    { angle: 45, style: { space_light: 3 } },
  ],
}
const namedBack = docFromHash(docToHash(named))
if (JSON.stringify(namedBack.batches) !== JSON.stringify(named.batches)) missing.push('batch names through the link')

// Sea-ground and fill batches keep their ground and draw through the link.
{
  const fill = { area: {}, batches: [{ angle: 0, ground: 'sea', draw: 'fill', style: { input_blur: 3 } }] }
  const back = docFromHash(docToHash(fill))
  if (JSON.stringify(back.batches) !== JSON.stringify(fill.batches)) missing.push(`link loses a sea fill ${JSON.stringify(back.batches)}`)
  const dots = { area: {}, batches: [{ angle: 0, draw: 'dots', style: { space_light: 3, input_height: 0.5 } }] }
  if (JSON.stringify(docFromHash(docToHash(dots)).batches) !== JSON.stringify(dots.batches)) missing.push('link loses land dots')
}

// Ambiguous short names resolve longest-first, never silently.
if (noteToDoc('|a0,salt45').batches[0].style.sun_altitude !== 45) missing.push('salt vs sa')
if (noteToDoc('|a0,sa315').batches[0].style.sun_azimuth !== 315) missing.push('sa vs s')
if (noteToDoc('|a0,ssg1').batches[0].style.space_steep_gamma !== 1) missing.push('ssg vs ss')
if (noteToDoc('|a0,dlv3').batches[0].style.dash_levels !== 3) missing.push('dlv vs dl')
if (noteToDoc('|a0,mws12').batches[0].style.mod_wobble_size !== 12) missing.push('mws vs mw')

// Unknown names and garbage are hard errors, not misreads.
const mustThrow = ['|a0,zz5', '|a0,sabc', '', '|a0,s5,sl']
for (const bad of mustThrow) {
  let threw = false
  try { noteToDoc(bad) } catch { threw = true }
  if (!threw) missing.push(`should error: ${JSON.stringify(bad)}`)
}

// The short-name table must match the Rust one (drift guard). The list below
// is the exact `style!` table from crates/burin-map-core/src/style.rs.
const rustShorts = {
  space_light: 'sl', space_dark: 'sd', space_steep: 'ss',
  space_steep_gamma: 'ssg', dash_light: 'dl', dash_dark: 'dd', dash_mid: 'dm', dash_period: 'dp',
  dash_levels: 'dlv', dash_cutoff: 'dc', dash_cutoff_soft: 'dcs', dash_solid: 'dsol', mod_jitter: 'mj', mod_flat: 'mf', mod_breaks: 'mb',
  mod_wobble: 'mw', mod_wobble_size: 'mws', mod_smooth: 'ms', mod_wave: 'mwv',
  mod_wave_size: 'mwvs', mod_wave_zone: 'mwz',
  mod_wave_even: 'mwe', input_flat: 'if',
  input_gain: 'ig', input_dark_gamma: 'idg', input_light_gain: 'ilg',
  input_light_floor: 'ilf', input_blur: 'ib', input_broad_blur: 'ibb',
  input_broad: 'ibr', input_height: 'ih', sun_azimuth: 'sa', sun_altitude: 'salt', sun_exag: 'sx',
  engine_step: 'es', engine_min: 'em', engine_stop: 'esp', engine_fill: 'ef', engine_stripes: 'estr',
  batch_floor: 'bf', batch_stroke: 'bs', batch_seed: 'bseed',
  water_start: 'wst', water_reach: 'wr', water_fade: 'wf', water_size: 'ws', water_patches: 'wp',
  water_fray: 'wfr',
  mask_start: 'mst', mask_reach: 'mr', mask_fade: 'mfd', mask_fray: 'mfr',
  mask_start_m: 'mstm', mask_reach_m: 'mrm', mask_fray_m: 'mfrm',
}
for (const [full, short] of Object.entries(rustShorts)) {
  if (PARAM_SHORTS[full] !== short) missing.push(`drift: ${full} should be "${short}", got "${PARAM_SHORTS[full]}"`)
}
const extra = Object.keys(PARAM_SHORTS).filter((k) => !(k in rustShorts))
if (extra.length) missing.push(`drift: param(s) not in Rust: ${extra.join(', ')}`)
if (Object.keys(rustShorts).length !== 55) missing.push(`expected 55 params, Rust table has ${Object.keys(rustShorts).length}`)

// Legacy #doc= base64 links still open.
const legacy = `#doc=${Buffer.from(JSON.stringify(doc)).toString('base64')}`
const legacyBack = docFromHash(legacy)
if (legacyBack?.batches?.[0]?.angle !== 90) missing.push('legacy #doc= link')

// Object and bare-array JSON both parse.
if (parseDoc(JSON.stringify(doc)).batches.length !== 1) missing.push('parseDoc object')
if (parseDoc('[{"angle":0,"style":{"space_light":5}}]').batches.length !== 1) missing.push('parseDoc array')

// The removed frame keys are errors that name the key, in JSON and in links.
const throwsWith = (fn, text) => { try { fn() } catch (e) { return String(e.message).includes(text) } return false }
for (const key of ['px', 'width_mm', 'height_mm']) {
  if (!throwsWith(() => parseDoc(`{"area":{"${key}":100},"batches":[]}`), key)) missing.push(`old key ${key} should be rejected`)
}
for (const note of ['fpx1600|a0,s5', 'fw200|a0,s5', 'fh300|a0,s5']) {
  if (!throwsWith(() => noteToDoc(note), 'Unknown area setting')) missing.push(`old notation ${note} should be rejected`)
}

// ---------- Frame: the same rules as the CLI ----------
const F = await import('../src/lib/frame.js')
const near = (a, b, tol = 1e-6) => Math.abs(a - b) <= tol * Math.max(1, Math.abs(b))
const paper = { ...F.DEFAULT_FRAME, mode: 'paper', size_mm: [297, 420], scale: 50000 }
const g = F.groundKm(paper)
if (!near(g[0], 14.85) || !near(g[1], 21)) missing.push(`paper ground ${g}`)
if (F.groundKm({ ...F.DEFAULT_FRAME, size_km: null }) !== null) missing.push('land without size has no ground yet')
if (!near(F.aspectOf(paper), 420 / 297)) missing.push('paper aspect')

// Zoom and ground are two views of one value.
for (const lat of [0, 37.75, 60]) {
  const z = F.zoomFor(lat, 8, 600)
  if (!near(F.groundWidthAt(lat, z, 600), 8)) missing.push(`zoom round trip at ${lat}`)
}
// A zoom by hand changes km in land mode, the scale in paper mode; the shape stays.
const landZoom = F.withGroundWidth({ ...F.DEFAULT_FRAME, size_km: [10, 5] }, 20.04)
if (landZoom.size_km[0] !== 20 || landZoom.size_km[1] !== 10) missing.push(`land zoom ${landZoom.size_km}`)
const zoomed = F.withGroundWidth(paper, 29.7)
if (zoomed.scale !== 100000 || zoomed.size_mm[0] !== 297) missing.push(`paper zoom ${zoomed.scale}`)

// The box keeps the frame's shape and fits the screen with room around it.
const box = F.fitBox(1000, 600, 2)
if (!near(box.h, 600 - 2 * F.FIT_PAD) || !near(box.w * 2, box.h) || !near(box.x + box.w / 2, 500)) missing.push('fitBox')

// Bounds are square in Mercator, so pixels are square and match the map.
const bb = F.boundsAround([7.7, 46], [10, 5])
const merc = (lng, lat) => [lng * Math.PI / 180, Math.log(Math.tan(Math.PI / 4 + lat * Math.PI / 360))]
const [x0, y0] = merc(bb.west, bb.south)
const [x1, y1] = merc(bb.east, bb.north)
if (!near((y1 - y0) / (x1 - x0), 0.5, 1e-9)) missing.push('boundsAround proportions')

// Grid: 2400 px for land (the CLI default); for paper the pen sets it, as in the CLI.
const lg = F.drawGrid({ ...F.DEFAULT_FRAME, size_km: [10, 5] })
if (lg.width !== 2400 || lg.height !== 1200 || lg.mmPerPx !== null) missing.push(`land grid ${lg.width}x${lg.height}`)
const pg = F.drawGrid({ ...paper, size_mm: [210, 297], pen_mm: 0.3 }, 1.2)
if (pg.width !== 840 || pg.height !== 1188 || !near(pg.mmPerPx, 0.25)) missing.push(`paper grid ${pg.width}x${pg.height}`)
const big = F.drawGrid({ ...paper, size_mm: [841, 1189], pen_mm: 0.3 }, 1.2)
if (Math.max(big.width, big.height) > F.MAX_EDGE || big.capped?.width !== 3364) missing.push('large paper is capped and says so')

// Documents: any two of ground, paper and scale decide the third.
const a1 = F.frameFromArea({ at: '46,7.7', size_mm: [297, 420], scale: 50000 })
if (a1.frame.mode !== 'paper' || a1.center[0] !== 7.7 || a1.center[1] !== 46) missing.push('area paper + scale')
const a2 = F.frameFromArea({ at: '46,7.7', size_km: [20, 10], scale: 100000 })
if (a2.frame.mode !== 'paper' || a2.frame.size_mm[0] !== 200 || a2.frame.size_mm[1] !== 100) missing.push(`area ground + scale ${a2.frame.size_mm}`)
const a3 = F.frameFromArea({ at: '46,7.7', size_km: [8] })
if (a3.frame.mode !== 'land' || a3.frame.size_km[1] !== 8) missing.push('area one km value is a square')
const a4 = F.frameFromArea({ place: 'bern', size_mm: [200] })
if (a4.frame.mode !== 'paper' || !(a4.frame.scale > 0) || a4.frame.size_mm[1] !== 200) missing.push('area place + paper')
if (!throwsWith(() => F.frameFromArea({ at: '46,7', size_km: [5], size_mm: [200], scale: 1000 }), 'drop one')) missing.push('overdetermined area')
if (!throwsWith(() => F.frameFromArea({ at: '46,7', scale: 1000 }), 'scale needs')) missing.push('scale alone')
if (!throwsWith(() => F.frameFromArea({ at: 'Zermatt' }), 'lat,lon')) missing.push('address centre explains itself')

// And back: the area block the app exports.
const out = F.areaFromFrame(paper, [7.7, 46])
if (JSON.stringify(out) !== JSON.stringify({ at: '46.00000,7.70000', size_mm: [297, 420], scale: 50000 })) missing.push(`areaFromFrame ${JSON.stringify(out)}`)
if (JSON.stringify(F.areaFromFrame({ ...F.DEFAULT_FRAME, size_km: [8, 8] }, null)) !== '{"size_km":[8]}') missing.push('square km written once')

// ---------- Crop: the SVG the app hands to the downloader ----------
{
  const C = await import('../src/lib/crop.js')
  const layer = ' id="hills-1" inkscape:groupmode="layer" inkscape:label="hills-1"'
  const svg = '<svg xmlns="http://www.w3.org/2000/svg" xmlns:inkscape="http://www.inkscape.org/namespaces/inkscape"'
    + ' viewBox="0 0 100 100" fill="none" stroke-linecap="round" stroke-linejoin="round">\n'
    + `<g stroke="#ff0000" stroke-width="2" stroke-linecap="butt"${layer}>\n<path d="M10 10 L90 10"/>\n</g>\n`
    + '<g stroke="#000000" stroke-width="0.5">\n<path d="M10 20 L20 20"/>\n<path d="M200 200 L210 200"/>\n</g>\n</svg>\n'
  const cut = C.cropSvg(svg, [0, 0, 50, 50], [50, 50])
  if (!cut.includes(`<g stroke="#ff0000" stroke-width="2" stroke-linecap="butt"${layer}>`) || !cut.includes('stroke-width="0.5"')) {
    missing.push(`crop loses the pens ${cut}`)
  }
  if (!cut.includes('xmlns:inkscape=') || /<svg[^>]*>\s*<g fill/.test(cut)) {
    missing.push(`crop should keep layers at the root, with their namespace ${cut}`)
  }
  if (cut.includes('M200')) missing.push('crop keeps a path outside the frame')

  const fillSvg = '<svg viewBox="0 0 100 100"><g fill="none" stroke-linecap="round" stroke-linejoin="round">\n'
    + '<g stroke="none" fill="#dddddd" fill-rule="evenodd" id="fill">\n<path d="M0 0 L100 0 L100 100 L0 100 Z M200 200 L210 200 L210 210 Z"/>\n</g>\n'
    + '<g stroke="#000000" stroke-width="0.5">\n<path d="M10 20 L20 20"/>\n</g>\n</g></svg>\n'
  const fillCut = C.cropSvg(fillSvg, [10, 10, 50, 50], [40, 40])
  if (!fillCut.includes('fill-rule="evenodd" id="fill">') || !/M0 0 L40 0 L40 40 L0 40 Z|L0 0[^"]*Z/.test(fillCut) || fillCut.includes('M200') || fillCut.includes('190')) {
    missing.push(`crop mangles a fill ${fillCut}`)
  }
  if (C.strokeOf(fillSvg) !== 0.5) missing.push('strokeOf should look past a fill')
}

// ---------- The loading panel ----------
const { default: LoadPanel } = await import('../src/components/LoadPanel.jsx')
const wasm = await import('../pkg/burin_map_wasm.js')
const { readFileSync } = await import('node:fs')
await wasm.default({ module_or_path: readFileSync(new URL('../pkg/burin_map_wasm_bg.wasm', import.meta.url)) })
const engine = new wasm.Panel('952×860 px · 1 batch', '[{"angle":0,"style":{}}]', '{"shoreline":{"lines":1},"batches":[]}')
engine.area(-122.5, 37.7, -122.4, 37.8, 0)
engine.tile(3, 4, 0.2)
engine.waterTile(2, 2, 0.3)
engine.water(0.4)
engine.message('elevation 10/1/2: HTTP 503, retry 1 in 0.5 s', true, 0.5)
engine.elevation(new Float32Array([-40, 0, 120, 300]), 4, 12, 2, 2, 8.4, 0.6)
const slots = Array.from(engine.slots())
engine.board(slots[1], 2, 0.7)
engine.board(slots[2], 2, 0.8)
const view = JSON.parse(engine.view(1.0))
engine.free()
globalThis.requestAnimationFrame = () => 0
const panelEl = document.createElement('div')
const panelRoot = createRoot(panelEl)
await act(async () => { panelRoot.render(createElement(LoadPanel, { view })) })
const panel = panelEl.textContent
for (const want of ['please wait', '✓ download', '4 tiles', '├ elevation', '4/4 tiles', 'zoom 12', '└ water', 'drawing', '├ terrain', '└ 1 lines', 'trace · ', 'messages', '0.0 s  8.8 × 11.1 km around 37.75°N 122.45°W', '0.5 s  elevation 10/1/2', 'highest point 300 m, sea floor down to 40 m']) {
  if (!panel.includes(want)) missing.push(`load panel: "${want}" in ${JSON.stringify(panel)}`)
}
act(() => panelRoot.unmount())

console.log('rendered', html.length, 'chars')
if (errors.length) {
  console.error('React errors:\n' + errors.join('\n'))
}
if (missing.length || errors.length) {
  if (missing.length) console.error('MISSING:', missing)
  process.exit(1)
}
console.log('sidebar render OK; all checks present')
dom.window.close()
process.exit(0)
