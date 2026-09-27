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
