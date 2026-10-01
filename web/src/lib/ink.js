/**
 * The settings a land batch, water batch or shoreline carries beside its
 * style to pick each dash's pen: layer, colour, width and line ends.
 */

export const CAPS = ['round', 'round_inside', 'square', 'butt']

export const INK_KEYS = ['layer', 'color', 'color_expr', 'width_expr', 'width_steps', 'cap']

export const INK_HELP = {
  layer: 'SVG layer id for export; separate from name, which is only a label in the app',
  color: 'stroke colour as #rgb or #rrggbb',
  color_expr: 'colour from a small expression; see SYNTAX.md',
  width_expr: 'line width as a multiple of batch_stroke, e.g. lerp(2, 0.5, clamp(dist / 200, 0, 1)); see SYNTAX.md',
  width_steps: 'round the widths to this many pens, each its own layer; 0 keeps every width',
  cap: `line ends: ${CAPS.join(', ')}`,
}

const text = (v) => (typeof v === 'string' && v.trim() ? v.trim() : null)

/**
 * The ink settings of `b` that change the drawing, cleaned; defaults left out.
 * @param {Record<string, any>} b
 */
export function pickInk(b) {
  const steps = Number(b?.width_steps)
  return {
    ...(text(b?.layer) ? { layer: text(b.layer) } : {}),
    ...(text(b?.color) ? { color: text(b.color) } : {}),
    ...(text(b?.color_expr) ? { color_expr: text(b.color_expr) } : {}),
    ...(text(b?.width_expr) ? { width_expr: text(b.width_expr) } : {}),
    ...(Number.isInteger(steps) && steps >= 2 ? { width_steps: steps } : {}),
    ...(CAPS.includes(b?.cap) && b.cap !== 'round' ? { cap: b.cap } : {}),
  }
}

/**
 * `pickInk`, but throwing on values that are there and wrong, for the editors.
 * @param {Record<string, any>} b
 * @param {string} which
 */
export function readInk(b, which) {
  for (const k of ['layer', 'color', 'color_expr', 'width_expr']) {
    if (b[k] != null && typeof b[k] !== 'string') throw new Error(`${k} in ${which} must be text`)
  }
  if (b.width_steps != null) {
    const n = Number(b.width_steps)
    if (!Number.isInteger(n) || n < 0 || n === 1) throw new Error(`width_steps in ${which} is 0, or a whole number from 2`)
  }
  if (b.cap != null && !CAPS.includes(b.cap)) {
    throw new Error(`cap in ${which} is ${CAPS.join(', ')}, not "${b.cap}"`)
  }
  return pickInk(b)
}
