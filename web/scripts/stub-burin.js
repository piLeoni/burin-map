/** Stub of src/lib/burin.js for the SSR render smoke test. */
export async function loadBurin() {
  const params = [
    { name: 'space_light', default: 4.8, help: 'distance between lines in light, px', group: 'space' },
    { name: 'dash_light', default: 0.15, help: 'ink fraction in light', group: 'dash' },
    { name: 'batch_stroke', default: 1.2, help: 'stroke width', group: 'batch' },
  ]
  const groups = [
    { id: 'space', blurb: 'distance between lines' },
    { id: 'dash', blurb: 'ink along each line' },
    { id: 'batch', blurb: 'per-pass settings' },
  ]
  return {
    style_params: () => params,
    style_groups: () => groups,
    draw_batches: () => {
      throw new Error('not used in render test')
    },
  }
}

export function paintDrawing() {}
export async function drawBatches() {
  throw new Error('not used in render test')
}
