import assert from 'node:assert/strict'
import test from 'node:test'
import { boundsAround, drawElevation, styleParams, version } from './index.js'

test('version and style params', () => {
  assert.equal(version(), '0.1.0')
  const params = styleParams()
  assert.ok(params.some(([name]) => name === 'space'))
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
