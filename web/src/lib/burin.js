/** Load the burin-map WASM module once and paint drawings to a canvas. */

let ready = null

export async function loadBurin() {
  if (!ready) {
    // Built by wasm-pack into public/pkg. Use Function so Vite does not try
    // to bundle / analyse the file (it rejects JS imports from /public).
    ready = Function('return import("/pkg/burin_map_wasm.js")')().then(async (m) => {
      await m.default()
      return m
    })
  }
  return ready
}

export function paintDrawing(canvas, drawing, style) {
  const w = drawing.width
  const h = drawing.height
  canvas.width = w
  canvas.height = h
  const ctx = canvas.getContext('2d')
  ctx.fillStyle = '#ffffff'
  ctx.fillRect(0, 0, w, h)

  const paths = drawing.paths
  const ends = drawing.ends
  const stroke = style.batch_stroke ?? 1.2
  ctx.strokeStyle = '#000000'
  ctx.lineWidth = stroke
  ctx.lineCap = 'round'
  ctx.lineJoin = 'round'

  let start = 0
  for (let i = 0; i < ends.length; i++) {
    const end = ends[i]
    if (end <= start) {
      start = end
      continue
    }
    ctx.beginPath()
    ctx.moveTo(paths[start * 2], paths[start * 2 + 1])
    for (let p = start + 1; p < end; p++) {
      ctx.lineTo(paths[p * 2], paths[p * 2 + 1])
    }
    if (end - start === 1) {
      ctx.lineTo(paths[start * 2] + 0.01, paths[start * 2 + 1])
    }
    ctx.stroke()
    start = end
  }
}
