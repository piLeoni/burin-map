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
import engravedSea from '../../../presets/engraved-sea.js?raw'
import seaDepth from '../../../presets/sea-depth.js?raw'
import sculptedHills from '../../../presets/scripted-hills.js?raw'
import contourBelts from '../../../presets/contour-belts.js?raw'
import layerTints from '../../../presets/layer-tints.js?raw'
import inkClassic from '../../../presets/ink-classic.js?raw'
import inkSolid from '../../../presets/ink-solid.js?raw'
import inkHalf from '../../../presets/ink-half.js?raw'
import inkJitter from '../../../presets/ink-jitter.js?raw'
import inkLevels from '../../../presets/ink-levels.js?raw'
import inkFlat from '../../../presets/ink-flat.js?raw'
import inkThresholds from '../../../presets/ink-thresholds.js?raw'

/** The first comment line of a script, as its title. */
function titleOf(source) {
  const m = source.match(/^\s*\/\*\*?\s*\n?\s*\*?\s*([^\n*]+)/)
  return m ? m[1].trim() : null
}

const SHIPPED = [
  { id: 'contours', screen: 'fast', plot: 'quick', blurb: 'contour lines, lit and shaded', doc: contours },
  { id: 'downhill', screen: 'medium', plot: 'medium', blurb: 'lines running down every slope, denser in shadow; a stippled sea', doc: downhill },
  { id: 'engraved-sea', screen: 'medium', plot: 'slow', blurb: 'crosshatched hills, coast lines, then a sea of evenly waving currents', doc: engravedSea },
  { id: 'sea-depth', screen: 'fast', plot: 'quick', blurb: 'the seabed in contour lines, and nothing on land', doc: seaDepth },
  { id: 'scripted-hills', screen: 'fast', plot: 'very slow', blurb: 'a loop over angles: crosshatching that tightens with the slope', doc: sculptedHills },
  { id: 'contour-belts', screen: 'fast', plot: 'slow', blurb: 'a loop in JavaScript: a coloured belt for every 100 m of height and depth', doc: contourBelts },
  { id: 'layer-tints', screen: 'fast', plot: 'slow', blurb: 'bands of colour by height and depth under coloured contours', doc: layerTints },
  { id: 'ink-classic', screen: 'fast', plot: 'quick', blurb: 'the old default look: jitter, flat lottery, breaks, cutoff', doc: inkClassic },
  { id: 'ink-solid', screen: 'fast', plot: 'quick', blurb: 'contour lines only: full ink in light and shadow', doc: inkSolid },
  { id: 'ink-half', screen: 'fast', plot: 'quick', blurb: 'mechanical 50 % dashes in light, solid in shadow', doc: inkHalf },
  { id: 'ink-jitter', screen: 'fast', plot: 'quick', blurb: 'ink-half by hand: same average ink', doc: inkJitter },
  { id: 'ink-levels', screen: 'fast', plot: 'quick', blurb: 'three flat zones of ink: solid, 50 %, 10 %', doc: inkLevels },
  { id: 'ink-flat', screen: 'fast', plot: 'quick', blurb: 'ink-half with only the flat-ground lottery', doc: inkFlat },
  { id: 'ink-thresholds', screen: 'fast', plot: 'quick', blurb: 'a constant 30 %, cut below and solid above two tones', doc: inkThresholds },
]

export const PRESETS = SHIPPED.map((p) => ({
  ...p,
  label: titleOf(p.doc) ?? p.id,
  atlas: atlases[p.id] ?? null,
}))

/** The document the app opens on. */
export const START_SCRIPT = PRESETS[0].doc
