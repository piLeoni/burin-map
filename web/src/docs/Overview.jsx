import { Link } from 'react-router-dom'
import ZoomFigure from './ZoomFigure.jsx'
import manifest from './figures.json'

const hero = manifest.examples.contours

const REPO = 'https://github.com/piLeoni/burin-map/blob/main'

const STYLES = [
  { slug: 'land-lines', title: 'Land — lines', blurb: 'Streamlines that follow (or cut across) the terrain, dashed by light/shadow. The default style for drawing land.' },
  { slug: 'land-dots', title: 'Land — dots', blurb: 'Stippling instead of lines: denser in shadow and on steep ground.' },
  { slug: 'water-lines', title: 'Water — lines', blurb: 'Lines on water, spaced by distance from the shore — tight near the coast, loose further out.' },
  { slug: 'water-ripples', title: 'Water — ripples', blurb: 'Broken, swelling rows evoking old nautical-chart seas.' },
  { slug: 'water-dots', title: 'Water — dots', blurb: 'Stippled water, denser near the coast.' },
  { slug: 'shoreline', title: 'Shoreline', blurb: 'The coast itself: one solid line, or several, optionally dashed or wobbled.' },
  { slug: 'fills', title: 'Fills', blurb: 'Flat colour bands by height (land) or depth (sea) — no ink texture.' },
]

const INTERFACES = [
  {
    name: 'JavaScript documents',
    desc: 'The main way to describe a map: a small JS program that calls burin.area(), burin.batch(), burin.water(), burin.shore() and ends with burin.draw(). It runs in a JS interpreter embedded in the engine (Boa), so no Node is needed. The map editor and the CLI run documents directly, and the Node and Python packages can evaluate them; every preset is one.',
    where: 'crates/burin-map-script · presets/*.js',
    use: 'burin.area({ at: "37.7,-122.62", size_km: [40, 28] });\nburin.batch({ angle: 0, style: { dash_light: 0.5 } });\nburin.water("lines", { follow: "shore", angle: 0, style: {} });\nburin.shore({ lines: 2 });\nburin.draw();',
  },
  {
    name: 'Command line: burin-map',
    desc: 'A standalone binary. Run a document, or give an area and style flags directly; it writes PNG, SVG (layered, for plotters and editors) or GeoJSON.',
    where: 'crates/burin-map',
    use: 'burin-map --script presets/engraved-sea.js --out-file sea.svg\nburin-map --terrain-at "Twin Peaks, San Francisco" --terrain-size 20km --out-file sf.png',
  },
  {
    name: 'Browser (WebAssembly)',
    desc: 'The same engine compiled to WebAssembly runs the map editor on this site, entirely in your browser: pick an area, edit the document, see it redraw.',
    where: 'crates/burin-map-wasm · crates/burin-map-script-wasm · web/',
    use: null,
  },
  {
    name: 'Node.js package',
    desc: 'Native bindings (napi-rs): draw an area or your own elevation grid and get SVG, PNG or raw polylines back, or evaluate a document into the atlas the engine reads.',
    where: 'crates/burin-map-node · node/',
    use: "const burin = require('burin-map')\nconst [w, s, e, n] = burin.boundsAround(37.75, -122.45, 20, 20)\nconst d = burin.draw(w, s, e, n, { dash_light: 0.5 })\nrequire('fs').writeFileSync('sf.svg', d.svg())",
  },
  {
    name: 'Python package',
    desc: 'Native bindings (PyO3): the same calls, with numpy arrays and a Style dataclass listing every parameter.',
    where: 'crates/burin-map-py · python/',
    use: 'import burin_map\nlat, lon, _ = burin_map.locate("Twin Peaks, San Francisco")\nd = burin_map.draw(burin_map.bounds_around(lat, lon, 20, 20))\nd.save_svg("sf.svg")',
  },
]

const REPO_MAP = [
  { path: 'crates/burin-map-core', desc: 'The engine itself: terrain/water sampling, the style model, dashing, streamlines, rasterizing/SVG output.' },
  { path: 'crates/burin-map', desc: 'The CLI binary.' },
  { path: 'crates/burin-map-script', desc: 'The JS document interpreter (the burin.* functions) used by the CLI, WASM, and tests.' },
  { path: 'crates/burin-map-wasm, crates/burin-map-script-wasm', desc: 'WASM builds of the engine and the script interpreter, for the browser.' },
  { path: 'crates/burin-map-py, python/', desc: 'Python package and bindings.' },
  { path: 'crates/burin-map-node, node/', desc: 'Node.js package and bindings.' },
  { path: 'presets/', desc: 'Example JS documents — one style idea per file.' },
  { path: 'examples/', desc: 'Example JSON5 style files (the declarative, non-scripted alternative to a JS document).' },
  { path: 'web/', desc: 'This website: the live map editor and the docs page you are reading.' },
  { path: 'docs/guide/', desc: 'Source markdown for this guide — one page per draw style, plus the shared model.' },
  { path: 'SYNTAX.md', desc: 'The full, flat parameter reference: every field, every default, in one file.' },
]

function Code({ children }) {
  return (
    <pre className="docs-code">
      <code>{children}</code>
    </pre>
  )
}

export default function Overview() {
  return (
    <article className="docs-article docs-prose docs-overview">
      <h1>Burin Map</h1>
      <p className="docs-lede">
        Burin Map turns real elevation and coastline data into engraved-looking maps —
        the kind of hand-cut, hatched linework you'd see on an old nautical chart or a
        burin engraving, not a photo-realistic render. You pick an area on Earth, pick a
        handful of styles (how land is drawn, how water is drawn, how the shoreline looks),
        and the engine traces thousands of short dashes whose spacing and darkness follow
        the terrain's slope and the sun's shadow.
      </p>
      <div className="docs-hero">
        <ZoomFigure
          w={hero.w}
          h={hero.h}
          tiles={[
            {
              src: hero.src,
              label: 'The San Francisco peninsula, default style',
              code: 'burin.batch({ angle: 0, style: {} });  // presets/contours.js',
            },
          ]}
          columns={1}
          open={hero.src}
        />
        <p className="docs-hero-more">
          One pass, nothing set: line spacing and ink follow the light on the terrain. Zoom in to see the dashes.
          Combinations of passes — an engraved sea, depth contours, colour tints — are in{' '}
          <Link to="/docs/examples">Examples</Link>.
        </p>
      </div>

      <h2>What it draws</h2>
      <p>
        Everything the engine draws falls into one of seven kinds, called <em>draw styles</em>.
        All of them share one underlying model — see{' '}
        <Link to="/docs/model">the shared model</Link> — but each has its own parameters,
        documented on its own page:
      </p>
      <div className="docs-card-grid">
        {STYLES.map((s) => (
          <Link key={s.slug} to={`/docs/${s.slug}`} className="docs-card">
            <strong>{s.title}</strong>
            <span>{s.blurb}</span>
          </Link>
        ))}
      </div>

      <h2>It's a Rust engine, with several front doors</h2>
      <p>
        All of the above is implemented once, in Rust, in <code>burin-map-core</code>. Everything
        else — the CLI, the website, the Python/Node packages — is a thin wrapper around that
        one engine, so a map rendered from the command line, from a Python script, or in this
        browser tab is pixel-for-pixel the same engine doing the same work.
      </p>
      {INTERFACES.map((i) => (
        <div key={i.name} className="docs-iface">
          <h3>{i.name}</h3>
          <p>{i.desc}</p>
          <p className="docs-iface-where">
            <code>{i.where}</code>
          </p>
          {i.use ? <Code>{i.use}</Code> : null}
        </div>
      ))}

      <h2>Where things live</h2>
      <table>
        <thead>
          <tr>
            <th>Path</th>
            <th>What's there</th>
          </tr>
        </thead>
        <tbody>
          {REPO_MAP.map((r) => (
            <tr key={r.path}>
              <td><code>{r.path}</code></td>
              <td>{r.desc}</td>
            </tr>
          ))}
        </tbody>
      </table>

      <h2>Where to go next</h2>
      <ul>
        <li>
          <Link to="/docs/examples">Examples</Link> — whole documents (engraved sea, sea depth, layer tints…)
          and how each is put together.
        </li>
        <li>
          <Link to="/docs/model">The shared model</Link> — tone, the spacing axis, the ink
          axis, and the order rules are applied in. Read this first.
        </li>
        <li>
          <Link to="/docs/land-lines">Land — lines</Link> — the default style, and the
          biggest parameter table.
        </li>
        <li>
          <a href={`${REPO}/SYNTAX.md`} target="_blank" rel="noreferrer">SYNTAX.md</a> — every
          parameter, every default, no prose, for when you just need the table.
        </li>
      </ul>
    </article>
  )
}
