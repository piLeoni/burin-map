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
  { angle: 30, style: { space: 6, space_light: 1, dash_light: 0.2, batch_stroke: 1.5 } },
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
const checks = ['Batch 1', 'Batch 2', 'angle', 'space', 'dash-light', 'base distance between lines']
const missing = checks.filter((c) => !html.includes(c))

// Document helpers round-trip (the #doc= URL format).
const { docToHash, docFromHash, docToJson, parseDoc, docToNote, noteToDoc } = await import('../src/lib/doc.js')
const { PARAM_SHORTS } = await import('../src/lib/shorts.js')
const doc = {
  area: { at: '37.75111,-122.44722', size_km: [8, 8], px: 1600, pen_mm: 0.25 },
  batches: [{ angle: 90, style: { space: 6, dash_light: 0.2, batch_floor: 0.45 } }],
}
const hash = docToHash(doc)
if (hash.startsWith('#d=')) {
  const back = docFromHash(hash)
  if (back.batches[0].angle !== 90) missing.push('notation angle')
  if (back.batches[0].style.space !== 6) missing.push('notation space')
  if (back.batches[0].style.batch_floor !== 0.45) missing.push('notation batch_floor')
  if (back.area?.px !== 1600) missing.push('notation area px')
  if (back.area?.at !== '37.75111,-122.44722') missing.push('notation area at (comma in value)')
  if (back.area?.size_km?.[1] !== 8) missing.push('notation size_km')
} else {
  missing.push('hash should use the #d= notation')
}

// The whole document must survive the notation exactly.
const exact = docToNote(doc)
if (JSON.stringify(noteToDoc(exact)) !== JSON.stringify(doc)) missing.push('notation exact round-trip')

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
  mod_wobble: 'mw', mod_wobble_size: 'mws', mod_smooth: 'ms', input_flat: 'if',
  input_gain: 'ig', input_dark_gamma: 'idg', input_light_gain: 'ilg',
  input_light_floor: 'ilf', input_blur: 'ib', input_broad_blur: 'ibb',
  input_broad: 'ibr', sun_azimuth: 'sa', sun_altitude: 'salt', sun_exag: 'sx',
  engine_step: 'es', engine_min: 'em', engine_stop: 'esp', engine_fill: 'ef',
  batch_floor: 'bf', batch_stroke: 'bs', batch_seed: 'bseed',
}
for (const [full, short] of Object.entries(rustShorts)) {
  if (PARAM_SHORTS[full] !== short) missing.push(`drift: ${full} should be "${short}", got "${PARAM_SHORTS[full]}"`)
}
const extra = Object.keys(PARAM_SHORTS).filter((k) => !(k in rustShorts))
if (extra.length) missing.push(`drift: param(s) not in Rust: ${extra.join(', ')}`)
if (Object.keys(rustShorts).length !== 33) missing.push(`expected 33 params, Rust table has ${Object.keys(rustShorts).length}`)

// Legacy #doc= base64 links still open.
const legacy = `#doc=${Buffer.from(JSON.stringify(doc)).toString('base64')}`
const legacyBack = docFromHash(legacy)
if (legacyBack?.batches?.[0]?.angle !== 90) missing.push('legacy #doc= link')

// Object and bare-array JSON both parse.
if (parseDoc(docToJson(doc)).batches.length !== 1) missing.push('parseDoc object')
if (parseDoc('[{"angle":0,"style":{"space":5}}]').batches.length !== 1) missing.push('parseDoc array')

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
