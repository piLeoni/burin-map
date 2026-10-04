import assert from 'node:assert/strict'
import test from 'node:test'
import { readFileSync } from 'node:fs'
import { boundsAround, drawElevation, evaluate, styleParams, version } from './index.js'

test('version and style params', () => {
  assert.equal(version(), '0.1.0')
  const params = styleParams()
  assert.ok(params.some(([name]) => name === 'space'))
})

test('evaluate runs a document script and returns its atlas', () => {
  const script = `
    burin.area({ at: '46,7.7', size_km: [8, 6] })
    burin.batch({ angle: 0, style: { space: 5 } })
    burin.water('dots', { color: '#9db4c0' })
    burin.shore()
    burin.draw()
  `
  const { atlas, area } = JSON.parse(evaluate(script))
  assert.equal(atlas.length, 3)
  assert.equal(atlas[0].angle, 0)
  assert.equal(atlas[1].kind, 'water')
  assert.equal(atlas[2].kind, 'shore')
  assert.deepEqual(area, { at: '46,7.7', size_km: [8, 6] })
})

test('a mistake in a document comes back with its position', () => {
  assert.throws(() => evaluate('burin.batch({ angle: "x" })\nburin.draw()'), /line 1 column 12/)
  assert.throws(() => evaluate('burin.draw()'), /nothing to draw/)
})

test('every shipped preset runs', () => {
  for (const name of ['contours', 'contour-belts', 'downhill', 'layer-tints', 'scripted-hills', 'sea-depth']) {
    const script = readFileSync(new URL(`../presets/${name}.js`, import.meta.url), 'utf8')
    const { atlas } = JSON.parse(evaluate(script))
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
  const d = drawElevation(elev, w, h, 30, { space: 6, mod_wobble: 0, mod_jitter: 0 })
  assert.equal(d.width, w)
  assert.equal(d.height, h)
  const [pts, ends] = d.paths()
  assert.ok(pts.length > 0)
  assert.ok(ends.length > 0)
  assert.ok(d.svg().startsWith('<?xml') || d.svg().includes('<svg'))
  assert.ok(d.png().length > 100)
})

test('boundsAround', () => {
  const [west, south, east, north] = boundsAround(37.75, -122.45, 8, 8)
  assert.ok(west < east)
  assert.ok(south < north)
})
