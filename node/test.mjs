import assert from 'node:assert/strict'
import test from 'node:test'
import { readFileSync } from 'node:fs'
import vm from 'node:vm'
import burin from './index.js'
import native from './native.js'

test('version and style params', () => {
  assert.equal(burin.version(), '0.1.0')
  const params = burin.styleParams()
  assert.ok(params.some(([name]) => name === 'space_light'))
})

test('the package is the library a document calls', () => {
  const scope = { __burin: {} }
  vm.runInNewContext(native.librarySource(), scope)
  for (const [name, value] of Object.entries(scope.__burinDoc)) {
    assert.equal(typeof burin[name], typeof value, `burin.${name}`)
  }
  assert.ok(Array.isArray(burin.json()) && burin.json() instanceof Array)
  assert.equal(burin.tone, 'tone')
  assert.deepEqual(burin.lerp('#fff', '#000', burin.tone), ['lerp', '#fff', '#000', 'tone'])
  assert.ok(burin.parameters().some((p) => p.name === 'dash_cutoff_soft'))
})

test('a document builds here as it does in a script', () => {
  burin.area({ at: '46,7.7', size_km: [8, 6] })
  burin.batch({ angle: 0, style: { space_light: 5 }, dash_light: 0.4 })
  burin.water('dots', { color: '#9db4c0' })
  burin.shore({ lines: 2 })
  const atlas = burin.json()
  const { atlas: scripted } = burin.evaluate(`
    burin.area({ at: '46,7.7', size_km: [8, 6] })
    burin.batch({ angle: 0, style: { space_light: 5 }, dash_light: 0.4 })
    burin.water('dots', { color: '#9db4c0' })
    burin.shore({ lines: 2 })
    burin.draw()
  `)
  assert.deepEqual(atlas[0], { angle: 0, style: { space_light: 5, dash_light: 0.4 } })
  assert.equal(atlas[2].kind, 'shore')
  assert.deepEqual(JSON.parse(native.atlasFromJson(JSON.stringify(atlas))), scripted)
})

test('mistakes are caught where they are made', () => {
  assert.throws(() => burin.batch({ draw: 'hatch' }), /"lines", "dots" or "fill"/)
  assert.throws(() => burin.batch({ cap: 'square' }), /round, round-inside or butt/)
  assert.throws(() => burin.area({ centre: '46,7' }), /not one of/)
  assert.throws(() => burin.water('waves'), /lines, ripples or dots/)
})

test('evaluate runs a document script and returns its atlas', () => {
  const { atlas, area } = burin.evaluate(`
    burin.area({ at: '46,7.7', size_km: [8, 6] })
    burin.batch({ angle: 0, style: { space_light: 5 } })
    burin.water('dots', { color: '#9db4c0' })
    burin.shore()
    burin.draw()
  `)
  assert.equal(atlas.length, 3)
  assert.equal(atlas[1].kind, 'water')
  assert.equal(atlas[2].kind, 'shore')
  assert.deepEqual(area, { at: '46,7.7', size_km: [8, 6] })
})

test('a mistake in a document comes back with its position', () => {
  assert.throws(() => burin.evaluate('burin.batch({ angle: "x" })\nburin.draw()'), /line 1 column 12/)
  assert.throws(() => burin.evaluate('burin.draw()'), /nothing to draw/)
  assert.throws(() => burin.drawScript('burin.draw()'), /nothing to draw/)
})

test('every shipped preset runs', () => {
  for (const name of ['contours', 'contour-belts', 'downhill', 'layer-tints', 'scripted-hills', 'sea-depth']) {
    const script = readFileSync(new URL(`../presets/${name}.js`, import.meta.url), 'utf8')
    const { atlas } = burin.evaluate(script)
    assert.ok(Array.isArray(atlas) && atlas.length > 0, `${name} built no atlas`)
  }
})

test('drawElevation produces a drawing', () => {
  const w = 64
  const h = 64
  const elev = new Float32Array(w * h)
  for (let y = 0; y < h; y++) {
    for (let x = 0; x < w; x++) {
      const dx = x - w / 2
      const dy = y - h / 2
      elev[y * w + x] = 200 - Math.hypot(dx, dy) * 4
    }
  }
  const d = burin.drawElevation(elev, w, h, 30, { space_light: 6, mod_wobble: 0 })
  assert.equal(d.width, w)
  assert.equal(d.height, h)
  const [pts, ends] = d.paths()
  assert.ok(pts.length > 0)
  assert.ok(ends.length > 0)
  assert.ok(d.svg().includes('<svg'))
  assert.ok(d.png().length > 100)
})

test('boundsAround', () => {
  const [west, south, east, north] = burin.boundsAround(37.75, -122.45, 8, 8)
  assert.ok(west < east)
  assert.ok(south < north)
})
