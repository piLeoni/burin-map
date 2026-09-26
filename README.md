# burin-map

Pen-and-ink terrain maps from real elevation data: thousands of short black
strokes that follow the lie of the land, dense and solid on shadowed slopes,
sparse stipple on flat ground, a few scattered marks on lit slopes. Up close
it is line drawing; from afar it reads as hillshading.

**burin-map** is the project: Rust engine, CLI, Python and Node APIs, and the
interactive site **BurinMap** at [burin-map.vercel.app](https://burin-map.vercel.app)
(password `folder`) — same engine in the browser, not a different product.

```bash
cargo install burin-map
pip install burin-map          # or: conda install -c conda-forge burin-map
npm install -g burin-map       # or: npx burin-map …
```

*A burin is the steel tool maps were engraved with on copper plates for
centuries.*

<p align="center">
  <img src="examples/bay-area.png" width="720" alt="San Francisco Bay Area" />
  <br />
  <em>Bay Area</em>
</p>

Same place, same extent (Twin Peaks, 8 km), two styles:

<p align="center">
  <img src="examples/twin-peaks-freehand.png" width="360" alt="Twin Peaks — freehand" />
  &nbsp;
  <img src="examples/twin-peaks-mechanical.png" width="360" alt="Twin Peaks — mechanical" />
</p>

| | Freehand | Mechanical |
| --- | --- | --- |
| `spacing` | 6.5 | 10 |
| `flow_blur` | 2.5 | 20 |
| `wobble` | 55 | 0 |
| `jitter` | 0.75 | 0 |
| `breaks` | 0.4 | 0 |
| `shade_gain` | 5 | 2.8 |
| `lit_floor` | 0.18 | 0 |
| `flat_level` | 0.4 | 0.12 |
| `stroke` | 0.95 | 1.5 |

## Rust

```bash
cargo install burin-map
# library: cargo add burin-map   /   cargo add burin-map-core
```

Until crates.io has it, from this repo (needs a [Rust toolchain](https://rustup.rs)):

```bash
cargo install --path crates/burin-map      # → ~/.cargo/bin/burin-map
```

```bash
burin-map --place san-francisco -o bay.png --svg
burin-map --at "Twin Peaks, San Francisco" --size 14km -o tp.png
burin-map --at "37.8716,-122.2727" --size 12km 8km -o berkeley.png
burin-map --bounds -122.75 37.45 -122.05 38.05 -o bay.png
burin-map --help
```

The image is 2400 px wide unless you give `--width` or a page (see
[Plotter and print](#plotter-and-print)).

As a library:

```rust
let (lat, lon, _) = burin_map::locate("Twin Peaks, San Francisco")?;
let area = burin_map::bounds_around(lat, lon, 14.0, 14.0);
let map = burin_map::draw(&area, &burin_map::Style::default(), &burin_map::Page::default(), |m| println!("{m}"))?;
std::fs::write("tp.svg", map.to_svg())?;
```

## Python

Same project. The package name is `burin-map`; the module you import is
`burin_map`. Python ≥ 3.10; numpy is the only runtime dependency of the
wrapper.

```bash
pip install burin-map
# or:  conda install -c conda-forge burin-map
```

That installs `import burin_map` and the `burin-map` command. Prebuilt wheels
cover the usual platforms; from source you need a
[Rust toolchain](https://rustup.rs).

Until the package is on PyPI / conda-forge, install from this repo:

```bash
pip install "git+https://github.com/piLeoni/burin-map.git"
# or from a local clone:  pip install .    # editable: pip install -e ".[dev]"
```

```python
import burin_map

lat, lon, name = burin_map.locate("Twin Peaks, San Francisco")   # or "37.75,-122.45"
area = burin_map.bounds_around(lat, lon, 14, 14)                 # km

d = burin_map.draw(area)                     # 2400 px wide; width=... to change
d.save_png("tp.png")
d.save_svg("tp.svg")
d.paths                                  # dashes: list of N×2 numpy arrays, px

# Tweak the style (print(burin_map.style_help()) lists every parameter)
d = burin_map.draw(area, burin_map.Style(shade_gain=4, wobble=30))

# For paper: sized in mm, scale follows from page and pen
d = burin_map.draw(area, page_width_mm=297, page_height_mm=420, pen_mm=0.3, log=print)

# Any elevation grid you already have (metres, square pixels, row 0 = north)
d = burin_map.draw_elevation(elev, px_m=30.0)
```

Elevation is downloaded on demand (see [Data](#data)); addresses are looked
up with OpenStreetMap's [Nominatim](https://nominatim.org/) (one request
per run, no key).

## Node

Same engine as a native Node addon ([napi-rs](https://napi.rs/)): usable from
code and as `npx burin-map` / the `burin-map` command. Node ≥ 18.

```bash
npm install burin-map
# or:  npm install -g burin-map
# or:  npx burin-map --at "Twin Peaks, San Francisco" --size 8km -o tp.png
```

That installs `import` / `require("burin-map")` and the `burin-map` command.
Prebuilt binaries cover the usual platforms; from source you need a
[Rust toolchain](https://rustup.rs).

```js
import { locate, boundsAround, draw, drawElevation } from "burin-map";
import { writeFileSync } from "node:fs";

const [lat, lon] = locate("Twin Peaks, San Francisco");
const [west, south, east, north] = boundsAround(lat, lon, 14, 14);

const d = draw(west, south, east, north, { shade_gain: 4, wobble: 30 });
writeFileSync("tp.png", d.png());
writeFileSync("tp.svg", d.svg());

// Or any elevation grid you already have (metres, row 0 = north):
// const d = drawElevation(elevFloat32Array, width, height, pxM, { spacing: 6 });
```

## How it works

1. **Elevation.** SRTM tiles are mosaicked and reprojected to a transverse
   Mercator centred on the area, so pixels are square in metres, slopes are
   true and scale is exact at the centre.
2. **Direction.** The terrain is heavily smoothed and its gradient turned by
   90°: a field of unit vectors running along the contour lines. Lines that
   follow it wrap concentrically around peaks, like contours, but with broad
   even curves instead of bunching up on steep ground.
3. **Lines.** Streamlines are traced through that field at an even distance
   from each other (Jobard & Lefer, 1997), a little tighter on steep ground
   and in shadow. Gaps too narrow for a new line at full spacing but still
   wide get one in the middle, so lines don't bunch into bands.
4. **Tone.** Lambert shading at two scales: detail (ridges, valleys) and
   whole massifs, so an entire mountain side reads as one shadow from afar.
   Flat ground sits at a fixed tone, shadow raises it towards solid, light
   lowers it towards white.
5. **Dashes.** Every line is broken into dashes whose length follows the
   tone: solid in shadow, stipple on the flat. On lit slopes dashes are
   dropped at random rather than shortened, so a few strokes still hint at
   the form.
6. **Hand.** Dash length and pitch are randomised, on flat ground the line
   direction wanders slightly, and solid lines in shadow get an occasional
   small break, so nothing reads as machine-made.

Every length in the style is in output pixels, so the look stays the same at
any scale: a whole region or a single city.

## Plotter and print

```bash
burin-map --place san-francisco --page-width 267 --page-height 390 --pen 0.3 -o bay.png --svg
```

The pen takes the place of the stroke width: every proportion of the style
is kept, and resolution, line pitch and map scale follow from page and pen.
The drawing fits inside the page, the SVG has its size in millimetres and
no background, and lines are ordered to keep pen-up travel short. The log
reports page size, line pitch, scale and pen-up travel.

Ground size, paper size and scale are tied: give any two and the third
follows.

```bash
# ground + paper → scale (the area takes the page's proportions)
burin-map --at "Mont Blanc" --size 40km --page-width 267 --page-height 390 --svg
# paper + scale → ground: 297 mm at 1:50,000 is 14.85 km
burin-map --at "Zermatt" --page-width 297 --page-height 420 --scale 1:50000 --svg
# ground + scale → paper
burin-map --place bern --scale 1:100000 --svg
```

Giving all three is an error, since they can't all hold at once.

Lines are always 4 pen widths apart, so a finer pen means more lines and
more detail on the same sheet, and a longer plot:

| pen | line pitch | A3 bay, lines | scale |
|---|---|---|---|
| 0.3 mm | 1.2 mm | ~3,500 | 1:232,000 |
| 0.1 mm | 0.4 mm | ~26,000 | 1:232,000 |

## Style parameters

The ones worth touching first:

| parameter | default | effect |
|---|---|---|
| `spacing` | 4.8 | base distance between lines, px |
| `shade_dash` | 1 | light/shadow as **dashes** (0 = continuous strokes) |
| `shade_gap` | 1 | light/shadow as **spacing** (0 = uniform gaps) |
| `spacing_dark` | 0.7 | denser lines in shadow (with `shade_gap`) |
| `spacing_lit` | 1 | thinner lines in light if >1 (with `shade_gap`) |
| `spacing_steep` | 0.6 | denser on steep ground (`1` = off) |
| `shade_gain` | 3.0 | contrast of the relief |
| `flat_level` | 0.25 | tone of flat ground |
| `lit_floor` | 0.06 | tone kept in full light |
| `flow_blur` | 6 | smoothness of the line curves |
| `jitter` | 0.45 | dash irregularity; 0 looks machine-made |
| `wobble` | 20 | wander on flat ground, degrees |
| `breaks` | 0.15 | breaks in solid runs (scaled by `shade_dash`) |
| `stroke` | 1.2 | stroke width, px |
| `seed` | 1 | different drawing of the same place |

Shading is two parallel channels: turn **`shade_dash`** down for continuous
lines, turn **`shade_gap`** up with `spacing_dark` / `spacing_lit` for relief
by density. Use either, or both (the default).

`burin-map --help` (or `print(burin_map.style_help())` in Python) lists all of
them with a one-line description each. They are declared once, in
`crates/burin-map-core/src/style.rs`, and the command line and Python pick them
up from there.

### Multiple hatching passes

By default there is one pass at angle 0° (lines along the contours). Each
`--hatch` starts another pass; style flags after it apply only to that pass
until the next `--hatch`. `--angle` is degrees from the contour tangent
(0 = along, 90 = downslope) 
Passes run in parallel.

```bash
# simple — one pass, all defaults
burin-map --place bern

# still one pass, with style
burin-map --place bern --spacing 5

# along + broken cross-hatch in shadow
burin-map --at "Twin Peaks, San Francisco" --size 8km -o tp.png \
  --hatch --spacing 6 --breaks 0.12 \
  --hatch --angle 90 --breaks 0.85 --hatch-floor 0.45

# three angles
burin-map --place bern \
  --hatch --spacing 6 \
  --hatch --angle 90 --breaks 0.8 --hatch-floor 0.4 \
  --hatch --angle 45 --breaks 0.7 --hatch-floor 0.5
```

## Data

Elevation is SRTM 1 arc-second (≈30 m, 60°S–60°N), fetched from the
[AWS Terrain Tiles](https://registry.opendata.aws/terrain-tiles/) open
dataset and cached in `~/.cache/burin-map` (override with `BURIN_MAP_CACHE`).
SRTM is courtesy of NASA/USGS; see the dataset's
[attribution](https://github.com/tilezen/joerd/blob/master/docs/attribution.md).

Limits worth knowing:

- Nothing above 60°N or below 60°S (Scandinavia's north, Alaska, Antarctica).
- At 30 m, areas narrower than ~10–15 km start to show the data's grain.
  SRTM also partly records trees and buildings, so cities and forests can
  show relief that isn't ground.
- Sea is anything at or below 0.5 m, so land below sea level is left blank.

## Architecture

Everything that turns elevation into strokes lives in **`burin-map-core`**: a
dependency-free Rust crate (fields, streamlines, dashes, style parameters).
That crate is the single source of truth. Thin shells sit around it:

| Shell | Crate / folder | Role |
| --- | --- | --- |
| CLI + I/O | `crates/burin-map` | SRTM download, transverse Mercator, geocoding, PNG/SVG, `burin-map` command |
| Python | `crates/burin-map-py` + `python/burin_map` | PyO3 bindings; the importable `burin_map` package |
| Node | `crates/burin-map-node` + `node/` | napi-rs bindings; `require` / `import` and `npx burin-map` |
| Web | `crates/burin-map-wasm` + `web/` | WASM bindings; React + Mapbox demo that draws the current view |

```text
  elevation / place
         │
         ▼
   ┌─────────────────┐     style params (style.rs)
   │  burin-map-core │◄──────────────────────────
   └────────┬────────┘
            │ Drawing { paths, svg, … }
      ┌─────┼──────────┬──────────┐
      ▼     ▼          ▼          ▼
     CLI  Python      Node      WASM → browser map
```

The browser map does not reimplement the look: it stitches Terrarium DEM
tiles for the viewport, calls the same WASM-built engine, and overlays the
SVG. Local style and camera are remembered in the browser; a light password
gate keeps casual traffic out of the public demo.

## Code layout

One repository, one engine, several front ends:

```text
crates/burin-map-core/  the drawing engine: pure computation, no I/O
  style.rs              every parameter, with its description
  fields.rs             direction field, tone, line spacing
  streamlines.rs        evenly spaced line placement
  dashes.rs             lines → dashes following the tone
  order.rs              plotting order
  grid.rs, rng.rs       blur, gradient, percentiles; deterministic RNG
crates/burin-map/       Rust library + `burin-map` command
  dem.rs                SRTM download, cache, reprojection
  tmerc.rs              transverse Mercator
  geo.rs                addresses, coordinates, lengths, scales
  raster.rs             PNG
  cli.rs                command line (also run by the Python package)
crates/burin-map-py/    Python bindings (PyO3), built by maturin
crates/burin-map-node/  Node bindings (napi-rs), built from `node/`
crates/burin-map-wasm/  WebAssembly bindings (wasm-bindgen)
python/burin_map/       the Python package: a thin layer over the bindings
python/tests/           Python API tests
node/                   npm package: loader, `cli.js` for `npx`, native `.node`
web/                    React + DaisyUI (wireframe) + Mapbox + WASM overlay
  src/                  UI and terrarium → DEM stitching
  public/pkg/           wasm-pack output (served statically)
  serve.sh              build WASM + `npm run dev` (Vite proxies elevation tiles)
examples/               PNG gallery for the README
```

Each module opens with a comment explaining its step. The engine has no
dependencies at all, so the same code builds for the browser.

## WebAssembly

The same drawing engine runs in the browser via `burin-map-wasm`. Build it
with [wasm-pack](https://rustwasm.github.io/wasm-pack/):

```bash
cd crates/burin-map-wasm
wasm-pack build --target web --release --out-dir ../../web/pkg
cp -R ../../web/pkg ../../web/public/pkg   # Vite serves static files from public/
```

That produces `burin_map_wasm.js` + `.wasm`. From a page (or the demo in
`web/`):

```js
import init, { draw, style_params } from "./pkg/burin_map_wasm.js";
await init();

// elev: Float32Array of height in metres, row 0 = north
const result = draw(elev, width, height, pxM /* metres per pixel */, {
  spacing: 4.8,
  wobble: 20,
});
result.paths();   // packed Float32Array for canvas stroking
result.svg();     // SVG document string
```

You still need elevation yourself (the web demo stitches Terrarium DEM tiles
for the map viewport). Style knobs match the CLI / Python API; call
`style_params()` for the full list.

## Web map (BurinMap)

Live demo: **[burin-map.vercel.app](https://burin-map.vercel.app)** (password
`folder`). It is a PWA you can add to a phone home screen.

React + DaisyUI (wireframe) + Mapbox + the WASM module above. Explore the
basemap freely; when you stop, elevation tiles for that view are fetched,
drawn in WASM, and shown as an overlay. Moving again clears the overlay so
you can navigate, then redraws when you settle. On mobile the map is
full-screen; **Controls** opens the style panel.

```bash
cd web && npm install
cp .env.example .env.local
# edit .env.local: VITE_MAPBOX_TOKEN=pk.…  (from https://account.mapbox.com/access-tokens/)
./serve.sh                   # builds WASM, then `npm run dev`
# → http://127.0.0.1:8765
```

Or, with WASM already in `web/public/pkg`:

```bash
cd web && npm run dev
```

Vite proxies `/terrarium/…` to AWS (S3 has no CORS on GET). The Vercel
deploy uses a rewrite for the same path (`web/vercel.json`).

## Development

```bash
cargo test                                  # engine, CLI, projection
python -m venv .venv && source .venv/bin/activate
pip install -e ".[dev]" && pytest           # Python package
cd node && npm install && npm run build && npm test   # Node addon
cargo run --release -- --place bern -o bern.png   # try the CLI without installing
./web/serve.sh                              # WASM map demo
```

After changing Rust code, `pip install -e .` again to rebuild the Python
extension, `cd node && npm run build` for the Node addon, and re-run
`./web/serve.sh` (or just the `wasm-pack` line in it) for the browser.

## Licence

None chosen yet: all rights reserved for now.
