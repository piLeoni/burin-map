# burin

Pen-and-ink terrain maps from real elevation data: thousands of short black
strokes that follow the lie of the land, dense and solid on shadowed slopes,
sparse stipple on flat ground, a few scattered marks on lit slopes. Up close
it is line drawing; from afar it reads as hillshading.

Give it a place (a name, an address, coordinates or a bounding box) and it
downloads the elevation, draws, and writes a PNG and an SVG ready for a pen
plotter.

*A burin is the steel tool maps were engraved with on copper plates for
centuries.*

## Install

The engine is written in Rust and usable three ways: a command-line tool, a
Python package, and (planned) a web page. None is published yet; from a
clone you need a [Rust toolchain](https://rustup.rs).

Command line only:

```bash
cargo install --path crates/burin      # puts `burin` in ~/.cargo/bin
```

Python package (also installs the same `burin` command):

```bash
python -m venv .venv && source .venv/bin/activate
pip install .                          # compiles the Rust engine, ~1 min
```

Once on PyPI it will be `pip install burin-maps` (`burin` is taken there;
the command and the import are still `burin`), with prebuilt wheels so no
Rust is needed. Python ≥ 3.10, numpy is the only dependency.

Elevation is downloaded on demand (see [Data](#data)); addresses are looked
up with OpenStreetMap's [Nominatim](https://nominatim.org/) (one request
per run, no key).

## Use

Command line:

```bash
burin --place san-francisco -o sf.png --svg
burin --at "Twin Peaks, San Francisco" --size 14km -o tp.png   # address, 14 km square
burin --at "37.8716,-122.2727" --size 12km 8km -o berkeley.png # lat,lon; width height
burin --at "Mount Tamalpais" --size 8mi -o tam.png            # m, km or mi
burin --bounds -122.75 37.45 -122.05 38.05 -o bay.png          # west south east north
burin --help                                                   # every option
```

The image is 2400 px wide unless you give `--width` or a page (see
[Plotter and print](#plotter-and-print)).

Python:

```python
import burin

lat, lon, name = burin.locate("Twin Peaks, San Francisco")   # or "37.75,-122.45"
area = burin.bounds_around(lat, lon, 14, 14)                 # km

d = burin.draw(area)                     # 2400 px wide; width=... to change
d.save_png("tp.png")
d.save_svg("tp.svg")
d.paths                                  # the dashes: list of N×2 numpy arrays, px

# Tweak the style (print(burin.style_help()) lists every parameter)
d = burin.draw(area, burin.Style(shade_gain=4, wobble=30))

# For paper: sized in mm, scale follows from page and pen
d = burin.draw(area, page_width_mm=297, page_height_mm=420, pen_mm=0.3, log=print)

# Any elevation grid you already have (metres, square pixels, row 0 = north)
d = burin.draw_elevation(elev, px_m=30.0)
```

Rust:

```rust
let (lat, lon, _) = burin::locate("Twin Peaks, San Francisco")?;
let area = burin::bounds_around(lat, lon, 14.0, 14.0);
let map = burin::draw(&area, &burin::Style::default(), &burin::Page::default(), |m| println!("{m}"))?;
std::fs::write("tp.svg", map.to_svg())?;
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
burin --place san-francisco --page-width 267 --page-height 390 --pen 0.3 -o bay.png --svg
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
burin --at "Mont Blanc" --size 40km --page-width 267 --page-height 390 --svg
# paper + scale → ground: 297 mm at 1:50,000 is 14.85 km
burin --at "Zermatt" --page-width 297 --page-height 420 --scale 1:50000 --svg
# ground + scale → paper
burin --place bern --scale 1:100000 --svg
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
| `spacing` | 4.8 | distance between lines, px |
| `shade_gain` | 3.0 | contrast of the relief: higher = more solid shadows and whiter lights |
| `flat_level` | 0.25 | how much stipple flat ground keeps |
| `lit_floor` | 0.06 | how many scattered dashes stay on fully lit slopes (0 = pure white) |
| `broad_weight` | 0.5 | how much the massif-scale shading counts |
| `flow_blur` | 10 | smoothness of the line curves |
| `jitter` | 0.45 | irregularity of the dashes; 0 looks machine-made |
| `wobble` | 20 | how much lines wander on flat ground, degrees |
| `breaks` | 0.15 | how often solid shadow lines get a small break |
| `fill_ratio` | 0.8 | lower = fewer wide gaps between lines, but more short lines (1 = off) |
| `azimuth`, `altitude` | 315, 45 | light direction, degrees |
| `stroke` | 1.2 | stroke width, px |
| `seed` | 1 | change it for a different drawing of the same place |

`burin --help` (or `print(burin.style_help())` in Python) lists all of
them with a one-line description each. They are declared once, in
`crates/burin-core/src/style.rs`, and the command line and Python pick them
up from there.

## Data

Elevation is SRTM 1 arc-second (≈30 m, 60°S–60°N), fetched from the
[AWS Terrain Tiles](https://registry.opendata.aws/terrain-tiles/) open
dataset and cached in `~/.cache/burin` (override with `BURIN_CACHE`).
SRTM is courtesy of NASA/USGS; see the dataset's
[attribution](https://github.com/tilezen/joerd/blob/master/docs/attribution.md).

Limits worth knowing:

- Nothing above 60°N or below 60°S (Scandinavia's north, Alaska, Antarctica).
- At 30 m, areas narrower than ~10–15 km start to show the data's grain.
  SRTM also partly records trees and buildings, so cities and forests can
  show relief that isn't ground.
- Sea is anything at or below 0.5 m, so land below sea level is left blank.

## Code layout

One repository, one engine, several front ends:

```text
crates/burin-core/      the drawing engine: pure computation, no I/O
  style.rs              every parameter, with its description
  fields.rs             direction field, tone, line spacing
  streamlines.rs        evenly spaced line placement
  dashes.rs             lines → dashes following the tone
  order.rs              plotting order
  grid.rs, rng.rs       blur, gradient, percentiles; deterministic RNG
crates/burin/           Rust library + `burin` command
  dem.rs                SRTM download, cache, reprojection
  tmerc.rs              transverse Mercator
  geo.rs                addresses, coordinates, lengths, scales
  raster.rs             PNG
  cli.rs                command line (also run by the Python package)
crates/burin-py/        Python bindings (PyO3), built by maturin
python/burin/           the Python package: a thin layer over the bindings
python/tests/           Python API tests
web/                    (planned) WASM build and live map page
```

Each module opens with a comment explaining its step. The engine has no
dependencies at all, so the same code builds for the browser.

## Development

```bash
cargo test                                  # engine, CLI, projection
python -m venv .venv && source .venv/bin/activate
pip install -e ".[dev]" && pytest           # Python package
cargo run --release -- --place bern -o bern.png   # try the CLI without installing
```

After changing Rust code, `pip install -e .` again to rebuild the Python
extension.

## Licence

None chosen yet: all rights reserved for now.
