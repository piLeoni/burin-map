/**
 * DOM smoke test for the Sidebar — the component that previously crashed with
 * "group is not defined". Runs the real component with jsdom so effects fire
 * and the WASM-backed parameter list loads (WASM is stubbed).
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

const { createElement } = await import('react')
const { createRoot } = await import('react-dom/client')
const { act } = await import('react')
const { Provider, createStore } = await import('jotai')
const { styleAtom } = await import('../src/state.js')
const { default: Sidebar } = await import('../src/components/Sidebar.jsx')

const errors = []
const origError = console.error
console.error = (...args) => { errors.push(args.join(' ')); origError(...args) }

const store = createStore()
store.set(styleAtom, [
  { name: 'Ridges', angle: 30, style: { space: 6, space_light: 1, dash_light: 0.2, batch_stroke: 1.5 } },
  { angle: 90, style: { space: 5, space_light: 1.2, dash_light: 1, batch_stroke: 1 } },
])

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
// let loadBurin()'s promise resolve and the resulting state flush
await act(async () => { await new Promise((r) => setTimeout(r, 50)) })

const html = document.getElementById('root').innerHTML
const checks = ['>YAML<', '>JSON5<', '>CLI<', 'Load ▾', 'Import', 'Download', 'Copy', 'Link', 'In step with the map']
const missing = checks.filter((c) => !html.includes(c))
if (html.includes('type="range"')) missing.push('the sliders should be gone')
if (html.includes('Parameters')) missing.push('the Parameters tab should be gone')

// Document helpers round-trip (the #doc= URL format).
const { docToHash, docFromHash, parseDoc, docToNote, noteToDoc } = await import('../src/lib/doc.js')
const { PARAM_SHORTS } = await import('../src/lib/shorts.js')
const doc = {
  area: { at: '37.75111,-122.44722', size_mm: [210, 297], scale: 25000, pen_mm: 0.25 },
  batches: [{ angle: 90, style: { space: 6, dash_light: 0.2, batch_floor: 0.45 } }],
}
const hash = docToHash(doc)
if (hash.startsWith('#d=')) {
  const back = docFromHash(hash)
  if (back.batches[0].angle !== 90) missing.push('notation angle')
  if (back.batches[0].style.space !== 6) missing.push('notation space')
  if (back.batches[0].style.batch_floor !== 0.45) missing.push('notation batch_floor')
  if (back.area?.size_mm?.[1] !== 297) missing.push('notation area size_mm')
  if (back.area?.scale !== 25000) missing.push('notation area scale')
  if (back.area?.at !== '37.75111,-122.44722') missing.push('notation area at (comma in value)')
  if (docFromHash(docToHash({ area: { size_km: [8, 6] }, batches: doc.batches })).area?.size_km?.[1] !== 6) {
    missing.push('notation size_km')
  }
} else {
  missing.push('hash should use the #d= notation')
}

// The whole document must survive the notation exactly.
const exact = docToNote(doc)
if (JSON.stringify(noteToDoc(exact)) !== JSON.stringify(doc)) missing.push('notation exact round-trip')

// Water rides in a third section, and survives it exactly.
const wet = {
  ...doc,
  water: {
    batches: [
      { name: 'coast; waves', follow: 'shore', angle: 0, style: { space: 5, water_reach: 80 } },
      { follow: 'map', angle: 70, style: {} },
      { follow: 'shore', draw: 'dots', angle: 10, style: { space: 50, water_size: 18 } },
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
    { name: 'north, ridges; 50% | "steep"', angle: 0, style: { space: 5 } },
    { name: 'Vallées ñ', angle: 90, style: { space: 4 } },
    { angle: 45, style: { space: 3 } },
  ],
}
const namedBack = docFromHash(docToHash(named))
if (JSON.stringify(namedBack.batches) !== JSON.stringify(named.batches)) missing.push('batch names through the link')

// Ambiguous short names resolve longest-first, never silently.
if (noteToDoc('|a0,salt45').batches[0].style.sun_altitude !== 45) missing.push('salt vs sa')
if (noteToDoc('|a0,sa315').batches[0].style.sun_azimuth !== 315) missing.push('sa vs s')
if (noteToDoc('|a0,ssg1').batches[0].style.space_steep_gamma !== 1) missing.push('ssg vs ss')
if (noteToDoc('|a0,ddrop0').batches[0].style.dash_drop !== 0) missing.push('ddrop vs dd')
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
  space: 's', space_light: 'sl', space_dark: 'sd', space_steep: 'ss',
  space_steep_gamma: 'ssg', dash_light: 'dl', dash_dark: 'dd', dash_period: 'dp',
  dash_cutoff: 'dc', dash_drop: 'ddrop', mod_jitter: 'mj', mod_breaks: 'mb',
  mod_wobble: 'mw', mod_wobble_size: 'mws', mod_smooth: 'ms', mod_wave: 'mwv',
  mod_wave_size: 'mwvs', mod_wave_zone: 'mwz',
  mod_wave_even: 'mwe', input_flat: 'if',
  input_gain: 'ig', input_dark_gamma: 'idg', input_light_gain: 'ilg',
  input_light_floor: 'ilf', input_blur: 'ib', input_broad_blur: 'ibb',
  input_broad: 'ibr', input_height: 'ih', sun_azimuth: 'sa', sun_altitude: 'salt', sun_exag: 'sx',
  engine_step: 'es', engine_min: 'em', engine_stop: 'esp', engine_fill: 'ef',
  batch_floor: 'bf', batch_stroke: 'bs', batch_seed: 'bseed',
  water_start: 'wst', water_reach: 'wr', water_fade: 'wf', water_size: 'ws', water_patches: 'wp',
  water_fray: 'wfr',
}
for (const [full, short] of Object.entries(rustShorts)) {
  if (PARAM_SHORTS[full] !== short) missing.push(`drift: ${full} should be "${short}", got "${PARAM_SHORTS[full]}"`)
}
const extra = Object.keys(PARAM_SHORTS).filter((k) => !(k in rustShorts))
if (extra.length) missing.push(`drift: param(s) not in Rust: ${extra.join(', ')}`)
if (Object.keys(rustShorts).length !== 44) missing.push(`expected 44 params, Rust table has ${Object.keys(rustShorts).length}`)

// Legacy #doc= base64 links still open.
const legacy = `#doc=${Buffer.from(JSON.stringify(doc)).toString('base64')}`
const legacyBack = docFromHash(legacy)
if (legacyBack?.batches?.[0]?.angle !== 90) missing.push('legacy #doc= link')

// Object and bare-array JSON both parse.
if (parseDoc(JSON.stringify(doc)).batches.length !== 1) missing.push('parseDoc object')
if (parseDoc('[{"angle":0,"style":{"space":5}}]').batches.length !== 1) missing.push('parseDoc array')

// The removed frame keys are errors that name the key, in JSON and in links.
const throwsWith = (fn, text) => { try { fn() } catch (e) { return String(e.message).includes(text) } return false }
for (const key of ['px', 'width_mm', 'height_mm']) {
  if (!throwsWith(() => parseDoc(`{"area":{"${key}":100},"batches":[]}`), key)) missing.push(`old key ${key} should be rejected`)
}
for (const note of ['fpx1600|a0,s5', 'fw200|a0,s5', 'fh300|a0,s5']) {
  if (!throwsWith(() => noteToDoc(note), 'Unknown area setting')) missing.push(`old notation ${note} should be rejected`)
}

// Frame: the same rules as the CLI.
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
const land = F.withGroundWidth({ ...F.DEFAULT_FRAME, size_km: [10, 5] }, 20.04)
if (land.size_km[0] !== 20 || land.size_km[1] !== 10) missing.push(`land zoom ${land.size_km}`)
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

// The editor's three views: YAML, JSON5 and the command, against the real
// parameter table from crates/burin-map-core/src/style.rs.
const V = await import('../src/lib/views.js')
const { readFileSync } = await import('node:fs')
const styleRs = readFileSync(new URL('../crates/burin-map-core/src/style.rs', `file://${process.cwd()}/`), 'utf8')
const schema = {
  params: [...styleRs.matchAll(/^\s+(\w+): (\w+) \("\w+"\): \w+ = ([\d.]+), "([^"]+)";/gm)]
    .map(([, group, name, def, help]) => ({ name, group, default: Number(def), help })),
  groups: [...styleRs.matchAll(/\("(\w+)", "([^"]+)"\)/g)].map(([, id, blurb]) => ({ id, blurb })),
}
if (schema.params.length !== 46) missing.push(`views: read ${schema.params.length} params from style.rs`)
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b)
const model = {
  area: { at: '46.02000,7.74000', size_mm: [210, 297], scale: 25000, pen_mm: 0.35 },
  batches: [
    { name: "Bob's ridges: #1", angle: 0, style: { space: 6, input_gain: 3.5 } },
    { angle: -30, style: { batch_floor: 0.45 } },
  ],
  water: {
    classes: ['ocean', 'lake'],
    shoreline: { lines: 2, style: { dash_dark: 0.5, batch_stroke: 2 } },
    batches: [
      { name: 'along the coast', follow: 'shore', angle: 0, style: { space: 5, water_reach: 80 } },
      { follow: 'map', angle: 70, style: {} },
      { follow: 'map', draw: 'dots', angle: 80, style: { water_size: 20 } },
      { follow: 'map', draw: 'ripples', angle: 90, style: { mod_wave: 2 } },
    ],
  },
  bathymetry: null,
  notes: {
    '': { above: ['Zermatt, A4 at 1:25000', 'second line'] },
    'area.at': { after: 'the Matterhorn is south-west' },
    'batches.0.style.space': { above: ['wider in the valleys'], after: 'was 4.8' },
    'batches.1': { above: ['steep ground only'] },
    'water.shoreline.style.dash_dark': { after: 'dotted' },
    'water.batches.0.style.space': { after: 'wider offshore' },
  },
}
const kinds = Object.keys(V.VIEWS)
// The command gives water as one JSON argument, which holds no comments.
const dry = { ...model, notes: Object.fromEntries(Object.entries(model.notes).filter(([k]) => !k.startsWith('water.'))) }
for (const kind of kinds) {
  const text = V.renderView(kind, model, schema)
  let back
  try { back = V.parseView(kind, text, schema) } catch (e) { missing.push(`views: ${kind} can't read itself: ${e.message}`); continue }
  if (!same(back, kind === 'cli' ? dry : model)) missing.push(`views: ${kind} round trip\n${JSON.stringify(back)}`)
  for (const other of kinds) {
    const again = V.parseView(other, V.renderView(other, back, schema), schema)
    if (!same(again, kind === 'cli' || other === 'cli' ? dry : model)) missing.push(`views: ${kind} -> ${other} loses something`)
  }
  if (!text.includes('input_broad') && !text.includes('input-broad')) missing.push(`views: ${kind} should list unset parameters`)
}

// A seabed batch keeps its ground in the editor views and in a share link.
{
  const sea = { ...model, batches: [{ angle: 0, ground: 'sea', style: { space: 5 } }], notes: {} }
  for (const kind of ['yaml', 'json5']) {
    const back = V.parseView(kind, V.renderView(kind, sea, schema), schema)
    if (!same(back.batches, sea.batches)) missing.push(`views: ${kind} loses ground ${JSON.stringify(back.batches)}`)
  }
  const D = await import('../src/lib/doc.js')
  const linked = D.noteToDoc(D.docToNote({ ...sea, water: { shoreline: { lines: 1 }, batches: [] } }))
  if (!same(linked.batches, sea.batches)) missing.push(`link loses ground ${JSON.stringify(linked.batches)}`)
}

// Colour, width and line ends survive both editor views, on land, at sea and on the shoreline.
{
  const ink = { color_expr: 'lerp(#000, #f00, tone)', width_expr: 'lerp(2, 0.5, clamp(elev / 500, 0, 1))', width_steps: 3, cap: 'round_inside' }
  const inked = {
    ...model,
    batches: [{ angle: 0, style: { space: 3 }, ...ink }],
    water: {
      shoreline: { lines: 2, color: '#123456', cap: 'butt', style: {} },
      batches: [{ follow: 'shore', draw: 'dots', angle: 0, style: {}, width_expr: 'lerp(3, 1, clamp(dist / 100, 0, 1))', cap: 'square' }],
    },
    notes: {},
  }
  for (const kind of ['yaml', 'json5']) {
    const back = V.parseView(kind, V.renderView(kind, inked, schema), schema)
    if (!same(back.batches, inked.batches)) missing.push(`views: ${kind} loses land ink ${JSON.stringify(back.batches)}`)
    if (!same(back.water, inked.water)) missing.push(`views: ${kind} loses water ink ${JSON.stringify(back.water)}`)
  }
  let threw = false
  try { V.parseView('yaml', 'batches:\n  - cap: pointy\n', schema) } catch { threw = true }
  if (!threw) missing.push('views: a wrong cap should be an error')

  const C = await import('../src/lib/crop.js')
  const svg = '<svg viewBox="0 0 100 100"><g fill="none" stroke-linecap="round" stroke-linejoin="round">\n'
    + '<g stroke="#ff0000" stroke-width="2" stroke-linecap="butt" id="hills-1">\n<path d="M10 10 L90 10"/>\n</g>\n'
    + '<g stroke="#000000" stroke-width="0.5">\n<path d="M10 20 L20 20"/>\n<path d="M200 200 L210 200"/>\n</g>\n</g></svg>\n'
  const cut = C.cropSvg(svg, [0, 0, 50, 50], [50, 50])
  if (!cut.includes('<g stroke="#ff0000" stroke-width="2" stroke-linecap="butt" id="hills-1">') || !cut.includes('stroke-width="0.5"')) {
    missing.push(`crop loses the pens ${cut}`)
  }
  if (cut.includes('M200')) missing.push('crop keeps a path outside the frame')
}

// A stippled land batch keeps `draw: dots` in the editor views and in a share link.
{
  const dots = { ...model, batches: [{ angle: 0, draw: 'dots', style: { space: 3, input_height: 0.5 } }], notes: {} }
  for (const kind of ['yaml', 'json5']) {
    const back = V.parseView(kind, V.renderView(kind, dots, schema), schema)
    if (!same(back.batches, dots.batches)) missing.push(`views: ${kind} loses land dots ${JSON.stringify(back.batches)}`)
  }
  const D = await import('../src/lib/doc.js')
  const linked = D.noteToDoc(D.docToNote({ ...dots, water: { shoreline: { lines: 1 }, batches: [] } }))
  if (!same(linked.batches, dots.batches)) missing.push(`link loses land dots ${JSON.stringify(linked.batches)}`)
}

// Fills: `draw: fill` and its `where` survive the editor views; `where` is theirs alone.
{
  const filled = {
    ...model,
    batches: [
      { angle: 0, ground: 'sea', draw: 'fill', where: { depth_min: 200 }, style: {}, color: '#aaccee' },
      { angle: 0, draw: 'fill', style: { input_blur: 3 } },
      { angle: 0, style: { space: 3 } },
    ],
    notes: {},
  }
  for (const kind of ['yaml', 'json5']) {
    const text = V.renderView(kind, filled, schema)
    const back = V.parseView(kind, text, schema)
    if (!same(back.batches, filled.batches)) missing.push(`views: ${kind} loses fills ${JSON.stringify(back.batches)}`)
    const first = text.slice(text.indexOf('batches'), text.indexOf('style'))
    if (first.includes('angle')) missing.push(`views: ${kind} writes an angle for a fill`)
  }
  for (const [text, what] of [
    ['batches:\n  - where: { elev_min: 10 }\n', 'where on a line batch'],
    ['batches:\n  - draw: fill\n    where: { height: 10 }\n', 'an unknown where key'],
    ['batches:\n  - angle: 0\nwater:\n  batches:\n    - draw: fill\n', 'a water fill'],
  ]) {
    let threw = false
    try { V.parseView('yaml', text, schema) } catch { threw = true }
    if (!threw) missing.push(`views: ${what} should be an error`)
  }
  const D = await import('../src/lib/doc.js')
  const plain = { ...model, batches: [{ angle: 0, draw: 'fill', style: {} }], notes: {} }
  const linked = D.noteToDoc(D.docToNote({ ...plain, water: { shoreline: { lines: 1 }, batches: [] } }))
  if (!same(linked.batches, plain.batches)) missing.push(`link loses a fill ${JSON.stringify(linked.batches)}`)

  const C = await import('../src/lib/crop.js')
  const svg = '<svg viewBox="0 0 100 100"><g fill="none" stroke-linecap="round" stroke-linejoin="round">\n'
    + '<g stroke="none" fill="#dddddd" fill-rule="evenodd" id="fill">\n<path d="M0 0 L100 0 L100 100 L0 100 Z M200 200 L210 200 L210 210 Z"/>\n</g>\n'
    + '<g stroke="#000000" stroke-width="0.5">\n<path d="M10 20 L20 20"/>\n</g>\n</g></svg>\n'
  const cut = C.cropSvg(svg, [10, 10, 50, 50], [40, 40])
  if (!cut.includes('fill-rule="evenodd" id="fill">') || !/M0 0 L40 0 L40 40 L0 40 Z|L0 0[^"]*Z/.test(cut) || cut.includes('M200') || cut.includes('190')) {
    missing.push(`crop mangles a fill ${cut}`)
  }
  if (C.strokeOf(svg) !== 0.5) missing.push('strokeOf should look past a fill')
}

// Water lists input_flat (its dashes use it) but no terrain-only parameters.
{
  const yaml = V.renderView('yaml', model, schema)
  const wet = yaml.slice(yaml.indexOf('\nwater:'))
  if (!wet.includes('# input_flat:')) missing.push('views: water should list input_flat')
  for (const p of ['space_steep', 'batch_floor', 'sun_azimuth', 'input_gain']) {
    if (wet.includes(`${p}:`)) missing.push(`views: water should not list ${p}`)
  }
}

// Dots and ripples list only what they use, at their own defaults.
{
  // The engine's list gives each parameter its default per draw; this one is read from style.rs.
  const withDraws = { ...schema, params: schema.params.map((p) => (p.name === 'space' ? { ...p, dots_default: 5, ripples_default: 9 } : p)) }
  const yaml = V.renderView('yaml', model, withDraws)
  const dots = yaml.slice(yaml.indexOf('draw: dots'), yaml.indexOf('draw: ripples'))
  if (!dots.includes('# space: 5')) missing.push('views: dots default to space 5')
  if (dots.includes('dash_light') || dots.includes('engine_step') || dots.includes('mod_wave')) missing.push('views: dots list dashes, the tracer or the wave')
  const ripples = yaml.slice(yaml.indexOf('draw: ripples'))
  if (!ripples.includes('# space: 9')) missing.push('views: ripples default to space 9')
  if (!ripples.includes('mod_wave_size') || ripples.includes('engine_step')) missing.push('views: ripples list the wave, not the tracer')
  const lines = yaml.slice(yaml.indexOf('\nwater:'), yaml.indexOf('draw: dots'))
  if (lines.includes('water_size')) missing.push('views: water lines should not list water_size')
  let threw = false
  try { V.parseView('yaml', yaml.replace('draw: dots', 'draw: foam'), schema) } catch { threw = true }
  if (!threw) missing.push('views: an unknown draw should be an error')
}

// A later land batch shows the terrain it takes from the one above, not the default.
{
  const two = { area: {}, batches: [{ angle: 0, style: { space: 6, sun_azimuth: 200 } }, { angle: 90, style: {} }], notes: {} }
  const yaml = V.renderView('yaml', two, schema)
  const second = yaml.slice(yaml.indexOf('angle: 90'))
  if (!second.includes('# sun_azimuth: 200')) missing.push('views: second batch should show the inherited sun_azimuth')
  if (!second.includes('# space: 4.8')) missing.push('views: space is not terrain, so it should not carry over')
  if (!same(V.parseView('yaml', yaml, schema).batches, two.batches)) missing.push('views: inherited values must not become set')
}

// Uncommenting a default sets it; commenting a value out unsets it.
const one = { area: {}, batches: [{ angle: 0, style: { space: 6 } }], notes: {} }
const edits = {
  yaml: [['# dash_light: 0.15', 'dash_light: 0.3'], ['space: 6', '# space: 6']],
  json5: [['// "dash_light": 0.15,', '"dash_light": 0.3,'], ['"space": 6,', '// "space": 6,']],
  cli: [['# --dash-light 0.15', '--dash-light 0.3'], ['--space 6', '# --space 6']],
}
for (const kind of kinds) {
  let text = V.renderView(kind, one, schema)
  for (const [from, to] of edits[kind]) {
    if (!text.includes(from)) missing.push(`views: ${kind} has no "${from}"`)
    text = text.replace(from, to)
  }
  const got = V.parseView(kind, text, schema)
  if (!same(got.batches[0].style, { dash_light: 0.3 })) missing.push(`views: ${kind} comment toggle ${JSON.stringify(got.batches[0].style)}`)
  if (Object.keys(got.notes).length) missing.push(`views: ${kind} generated comments kept as notes ${JSON.stringify(got.notes)}`)
}

// Files saved when every line carried its help text read back without notes.
const oldYaml = [
  'area:',
  '  at: 46.00000,7.70000   # centre, "lat,lon"; dragging the map changes it',
  'batches:',
  '  - angle: 0   # line direction: 0 follows the contour lines, 90 runs straight downhill',
  '    style:',
  '      # ── Space: Distance between neighbouring lines, from open on lit slopes to dense in shadow.',
  '      space: 6   # distance between neighbouring lines, in px (default 4.8)',
  '      # space_light: 1   # spacing multiplier on fully lit slopes; above 1 opens them up',
].join('\n')
const oldBack = V.parseView('yaml', oldYaml, schema)
if (Object.keys(oldBack.notes).length || oldBack.batches[0].style.space !== 6) missing.push(`views: old help comments ${JSON.stringify(oldBack.notes)}`)
if (V.renderView('yaml', one, schema).includes('distance between')) missing.push('views: help text should no longer be written')

// Unknown parameters are errors in every view, and say which.
const bad = {
  yaml: 'batches:\n  - style:\n      spacing: 5\n',
  json5: '{ batches: [{ style: { spacing: 5 } }] }',
  cli: 'burin-map --spacing 5',
}
for (const kind of kinds) {
  if (!throwsWith(() => V.parseView(kind, bad[kind], schema), 'spacing')) missing.push(`views: ${kind} should reject "spacing"`)
}

// Plain JSON (what older builds exported) and a one-line command both read.
const oldJson = '{"area":{"at":"46,7.7"},"batches":[{"angle":90,"style":{"space":5}}]}'
if (V.parseView('json5', oldJson, schema).batches[0].style.space !== 5) missing.push('views: plain JSON')
const line = V.parseView('cli', "burin-map --tat 46,7.7 --ts 8km 6km --esc 1:50000 --s 6 --dl 0.3 --batch-name 'A b'", schema)
if (!same(line.area, { at: '46,7.7', size_km: [8, 6], scale: 50000 }) || !same(line.batches, [{ name: 'A b', angle: 0, style: { space: 6, dash_light: 0.3 } }])) {
  missing.push(`views: one-line command ${JSON.stringify(line)}`)
}
const shared = V.parseView('cli', 'burin-map --s 6 --batch --ba 90 --batch --dl 0.2', schema)
if (!same(shared.batches.map((b) => b.style), [{ space: 6 }, { space: 6, dash_light: 0.2 }])) missing.push('views: flags before --batch are shared')

if (V.detectView('a.yml', '') !== 'yaml' || V.detectView('a.json', '') !== 'json5' || V.detectView('x', 'burin-map --s 5') !== 'cli' || V.detectView('x', '{"a":1}') !== 'json5' || V.detectView('x', 'batches:\n  - angle: 0') !== 'yaml') {
  missing.push('views: detectView')
}

// The loading panel: counted, timed and endless tasks, one begun after the
// panel's clock last ticked, and the log.
const { default: LoadPanel } = await import('../src/components/LoadPanel.jsx')
const { createProgress } = await import('../src/lib/progress.js')
let load = { tasks: [], log: [] }
const tracker = createProgress((s) => { load = s })
const tiles = tracker.task('elevation', 'elevation', 4, 'tiles')
tiles.tick(3)
tracker.task('water', 'water', 2, 'tiles').fail('z11 · 0 new')
tracker.task('draw', 'drawing').eta(2000)
tracker.task('terrain', 'terrain', 0, '', { child: true, wait: true })
tracker.task('b1', '1 lines', 5, '', { child: true, wait: true }).step(2, 'trace')
tracker.task('sea', 'seabed')
tracker.log('elevation 10/1/2: HTTP 503, retry 1 in 0.5 s', 'warn')
tracker.log('water 11/3/4: HTTP 500', 'error')
await act(async () => { await new Promise((r) => setTimeout(r, 10)) })
for (const t of load.tasks) t.t0 += 1000
globalThis.requestAnimationFrame = () => 0
const panelEl = document.createElement('div')
const panelRoot = createRoot(panelEl)
await act(async () => { panelRoot.render(createElement(LoadPanel, { title: '952×860 px · 3 batches', load })) })
const panel = panelEl.textContent
for (const want of ['3/4 tiles', '✗', 'z11 · 0 new', '/ ~2.0 s', '├ terrain', '└ 1 lines', 'trace · ', 'seabed', '! elevation 10/1/2', '✗ water 11/3/4']) {
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
