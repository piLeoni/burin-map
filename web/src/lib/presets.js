/**
 * The documents this app ships: the scripts in the repository's `presets/`,
 * each with the atlas it builds, pre-run by
 * `cargo run -p burin-map-script --example check_presets -- --write` into
 * `atlases.json`. With the atlas at hand, drawing a preset does not need the
 * script interpreter; editing the script does.
 *
 * The same files are what `burin-map --script presets/<name>.js` runs, so a
 * preset on the page and the command line is one document, not two.
 *
 * `atlases.json` is checked by `examples/check_presets.rs`: it fails when a
 * preset stops running or its atlas is stale.
 */
import atlases from '../presets/atlases.json'
import contours from '../../../presets/contours.js?raw'
import downhill from '../../../presets/downhill.js?raw'
import seaDepth from '../../../presets/sea-depth.js?raw'
import sculptedHills from '../../../presets/scripted-hills.js?raw'
import contourBelts from '../../../presets/contour-belts.js?raw'
import layerTints from '../../../presets/layer-tints.js?raw'

/** The first comment line of a script, as its title. */
function titleOf(source) {
  const m = source.match(/^\s*\/\*\*?\s*\n?\s*\*?\s*([^\n*]+)/)
  return m ? m[1].trim() : null
}

const SHIPPED = [
  { id: 'contours', screen: 'fast', plot: 'quick', blurb: 'contour lines, lit and shaded', doc: contours },
  { id: 'downhill', screen: 'medium', plot: 'medium', blurb: 'lines running down every slope, denser in shadow; a stippled sea', doc: downhill },
  { id: 'sea-depth', screen: 'fast', plot: 'quick', blurb: 'the seabed in contour lines, and nothing on land', doc: seaDepth },
  { id: 'scripted-hills', screen: 'fast', plot: 'very slow', blurb: 'a loop over angles: crosshatching that tightens with the slope', doc: sculptedHills },
  { id: 'contour-belts', screen: 'fast', plot: 'slow', blurb: 'a loop in JavaScript: a coloured belt for every 100 m of height and depth', doc: contourBelts },
  { id: 'layer-tints', screen: 'fast', plot: 'slow', blurb: 'bands of colour by height and depth under coloured contours', doc: layerTints },
]

export const PRESETS = SHIPPED.map((p) => ({
  ...p,
  label: titleOf(p.doc) ?? p.id,
  atlas: atlases[p.id] ?? null,
}))

/** The document the app opens on. */
export const START_SCRIPT = PRESETS[0].doc
