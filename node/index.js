'use strict'

// The burin-map package: the same `burin` a document script calls, so a
// document runs here as it is written for the web page and the CLI.
//
//   const burin = require('burin-map')
//   burin.area({ at: '37.66,-122.45', size_km: [36, 25] })
//   burin.batch({ angle: 0, style: { space_light: 6 } })
//   const map = burin.draw()            // a Drawing: map.svg(), map.png()
//
// The library is the very source the script interpreter runs
// (`crates/burin-map-script/src/library.js`), mounted over the native addon.
// `burin.draw()` draws and starts a new, empty document.

const native = require('./native.js')

// Run in this realm, so its arrays and objects are this realm's, over a
// private `globalThis`: the library leaves nothing on the real one.
const library = new Function('globalThis', `${native.librarySource()}\n//# sourceURL=burin:library`)

/** The library over the addon, with an empty document. */
function mount() {
  let area = null
  const checked = (kind) => (json) => {
    native.check(kind, json)
    return JSON.parse(json)
  }
  const natives = {
    batch: checked('batch'),
    water: checked('water'),
    shore: checked('shore'),
    area(json) {
      area = checked('area')(json)
      return area
    },
    parameters: () => native.burinParameters(),
    atlas(json) {
      native.atlasFromJson(json)
      return JSON.parse(json)
    },
    draw(json) {
      const drawing = native.drawDocument(JSON.stringify({ atlas: JSON.parse(json), area }))
      current = mount()
      return drawing
    },
  }
  const scope = { __burin: natives }
  library(scope)
  return scope.__burinDoc
}

let current = mount()

const call = (name) => (...args) => current[name](...args)

/** Run a document script (its source text) and draw it. */
function drawScript(source) {
  return native.drawDocument(native.evaluate(source))
}

/** Run a document script and return what it built, `{ atlas, area }`. */
function evaluate(source) {
  return JSON.parse(native.evaluate(source))
}

// One assignment each, so `import { batch } from 'burin-map'` sees them too.

// The document, as a script writes it.
exports.area = call('area')
exports.batch = call('batch')
exports.fill = call('fill')
exports.water = call('water')
exports.shore = call('shore')
exports.draw = call('draw')
exports.json = call('json')
exports.parameters = call('parameters')
exports.elev = current.elev
exports.tone = current.tone
exports.dist = current.dist
exports.depth = current.depth
exports.lerp = call('lerp')
exports.rgb = call('rgb')
exports.clamp = call('clamp')
exports.smoothstep = call('smoothstep')
exports.step = call('step')
exports.abs = call('abs')
exports.floor = call('floor')
exports.ceil = call('ceil')
exports.round = call('round')
exports.min = call('min')
exports.max = call('max')
exports.mix = call('mix')

// Documents as files, and everything around them.
exports.drawScript = drawScript
exports.evaluate = evaluate
exports.drawElevation = native.drawElevation
exports.Drawing = native.Drawing
exports.locate = native.locate
exports.boundsAround = native.boundsAround
exports.projectedSize = native.projectedSize
exports.styleParams = native.styleParams
exports.styleGroups = native.styleGroups
exports.engine = native.engine
exports.forceCpu = native.forceCpu
exports.runCli = native.runCli
exports.version = native.version
