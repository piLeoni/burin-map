import { Link } from 'react-router-dom'
import ZoomFigure from './ZoomFigure.jsx'
import manifest from './figures.json'
import engravedSea from '../../../presets/engraved-sea.js?raw'
import seaDepth from '../../../presets/sea-depth.js?raw'
import layerTints from '../../../presets/layer-tints.js?raw'
import contourBelts from '../../../presets/contour-belts.js?raw'
import downhill from '../../../presets/downhill.js?raw'
import scriptedHills from '../../../presets/scripted-hills.js?raw'
import contours from '../../../presets/contours.js?raw'

/** Strip the leading doc comment: the page says it in prose. */
function body(src) {
  return src.replace(/^\/\*\*[\s\S]*?\*\/\s*/, '').trim()
}

const EXAMPLES = [
  {
    id: 'contours',
    title: 'Contours (the default)',
    source: contours,
    lede: 'One pass along the contours with the engine defaults, every one written out so you can change it — the same drawing as an empty style.',
    how: [
      ['By hand', <><code>mod_jitter: 1</code> makes every dash and gap a little different; <code>dash_light: 0.3</code> to <code>dash_dark: 0.8</code> lets the ink, not the spacing, carry the shadow.</>],
      ['Lit ground breaks up', <><code>mod_flat: 0.5</code> leaves dashes out on lit flats, and <code>dash_cutoff: 0.15</code> thins the lightest slopes further, fewer dashes the lighter the ground.</>],
    ],
  },
  {
    id: 'engraved-sea',
    title: 'Engraved sea',
    source: engravedSea,
    lede: 'The old-chart look: hatched hills, a doubled coast, and an open sea of currents that wave in step, each line an offset copy of its neighbour — so the water reads as engraving rather than hatching.',
    how: [
      ['Hills, two passes', <>Contours at <code>angle: 0</code> with a wide light spacing and almost no ink in light (<code>dash_light: 0.1</code>), then a second pass at <code>angle: 45</code> that only draws where the ground is dark (<code>batch_floor: 0.6</code>): a crosshatch on the shadowed slopes only.</>],
      ['The coast', <><code>burin.shore({'{'} lines: 2 {'}'})</code> with <code>space_light: 1.5</code> draws two tight concentric lines.</>],
      ['Just off the coast', <>Water lines that <code>follow: "shore"</code>, from 3 px to 20 px out (<code>water_start</code>, <code>water_reach</code>), tightening towards the coast (<code>space_dark: 2.4</code>).</>],
      ['The open sea', <>A second water pass with <code>follow: "map"</code> at <code>angle: 90</code> over everything past 26 px. <code>mod_wave</code> makes the lines wave, <code>mod_wave_even: 1</code> makes them wave together so they never cross, <code>mod_wobble</code> lets the current turn, and <code>water_fray</code> breaks them into dashes as they approach the coast band.</>],
    ],
  },
  {
    id: 'sea-depth',
    title: 'Sea depth',
    source: seaDepth,
    lede: 'The seabed as continuous contour lines and nothing on land — the submarine canyons read like hills.',
    how: [
      ['Sea only', <><code>ground: "sea"</code> makes a land-style batch draw only below the waterline, using depth as the terrain.</>],
      ['Pure contours', <><code>dash_light: 1, dash_dark: 1</code>: every line solid, no dashes. Spacing alone (15 px in light, 4.5 px in shadow) carries the relief.</>],
      ['A smooth seabed', <>Heavy <code>input_blur</code> and <code>mod_smooth</code>, because depth data is coarse; <code>engine_stripes: 1</code> traces in one piece so solid lines show no seams.</>],
    ],
  },
  {
    id: 'layer-tints',
    title: 'Layer tints',
    source: layerTints,
    lede: 'Bands of colour by height and depth, as on atlas maps, with coloured contours drawn over them.',
    how: [
      ['Fills first', <><code>burin.fill</code> with a <code>where</code> range paints one flat band; later fills sit on top, so each deeper or higher band is written after the paler one.</>],
      ['Then lines', <>A brown land batch and a blue <code>ground: "sea"</code> batch, each with its own <code>color</code>, and a coloured <code>burin.shore</code>.</>],
    ],
  },
  {
    id: 'contour-belts',
    title: 'Contour belts',
    source: contourBelts,
    lede: 'A coloured belt for every 100 m of height and depth — a loop, which is why a document is a program and not a config file.',
    how: [
      ['A loop', <>Two <code>for</code> loops over colour arrays emit one <code>burin.fill</code> per 100 m band.</>],
      ['Ink coloured by tone', <><code>shade: burin.lerp(a, b, burin.tone)</code> colours each line by its own light/shadow value instead of one flat colour.</>],
    ],
  },
  {
    id: 'downhill',
    title: 'Downhill',
    source: downhill,
    lede: 'Lines running straight down every slope instead of along it, with a stippled sea.',
    how: [
      ['Across, not along', <><code>angle: 90</code> turns the streamlines 90° from the contours: they follow the fall line.</>],
      ['Stippled water', <><code>burin.water("dots", …)</code> — dense at the shore (<code>space_dark: 3</code>), very sparse far out (<code>space_light: 25</code>) — and a doubled shoreline.</>],
    ],
  },
  {
    id: 'scripted-hills',
    title: 'Scripted hills',
    source: scriptedHills,
    lede: 'Crosshatching written as a loop over four angles: each pass adds a direction, so steep summits go dark.',
    how: [
      ['A loop over angles', <><code>for (const angle of [0, 45, 90, 135])</code> — four batches with the same style.</>],
      ['Darkness from overlap', <>Each pass tightens on steep ground (<code>space_steep</code>), so where all four are dense the paper goes black.</>],
    ],
  },
]

function Example({ ex }) {
  const fig = manifest.examples[ex.id]
  return (
    <section className="docs-example" id={ex.id}>
      <ZoomFigure w={fig.w} h={fig.h} tiles={[{ src: fig.src, label: '' }]} columns={1} compact open={fig.src} />
      <h2>{ex.title}</h2>
      <p>{ex.lede}</p>
      <dl className="docs-how">
        {ex.how.map(([k, v]) => (
          <div key={k}>
            <dt>{k}</dt>
            <dd>{v}</dd>
          </div>
        ))}
      </dl>
      <details className="docs-source">
        <summary>
          <code>presets/{ex.id}.js</code>
        </summary>
        <pre className="docs-code">
          <code>{body(ex.source)}</code>
        </pre>
      </details>
    </section>
  )
}

export default function Examples() {
  return (
    <article className="docs-article docs-prose docs-examples">
      <h1>Examples</h1>
      <p className="docs-lede">
        Whole documents from <code>presets/</code>, each combining several passes. The style pages explain one
        parameter at a time; these show what they add up to. Every map is the engine's SVG: zoom into it, or open it
        on its own. Each one is also in the map editor's preset menu, and runs from the command line:
      </p>
      <pre className="docs-code">
        <code>burin-map --script presets/engraved-sea.js --out-file sea.svg</code>
      </pre>
      <div className="docs-example-grid">
        {EXAMPLES.map((ex) => (
          <Example key={ex.id} ex={ex} />
        ))}
      </div>
      <p>
        Parameters used here are explained on the style pages — start with{' '}
        <Link to="/docs/model">the shared model</Link>.
      </p>
    </article>
  )
}
