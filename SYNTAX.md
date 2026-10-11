# burin-map document syntax

**Style model (spacing, ink, modifiers):** [docs/guide/README.md](docs/guide/README.md).
On the web app, open **Docs** (`/docs`) for zoomable comparisons of every style and whole examples.

A document describes a whole drawing, and a document is a **JavaScript script**:
it calls `burin.*` and ends with `burin.draw()`, and running it builds a *graph*
of typed operations (the *atlas*) — the batches and water passes the engine
draws, in order. The CLI runs it with `--script`; the web editor edits the same
kind of file. The Node package is the same `burin` (`require('burin-map')`), and
the Python package makes the same calls with keywords
(`burin.batch(angle=0, style={...})`). See
[JavaScript documents](#5-javascript-documents) for the vocabulary.

A saved atlas — the graph by itself, without the script — is still read as
**JSON5** with `--style-file` / `--style-json`, as an escape hatch for a drawing
you kept rather than a document you write. Its shape is the same four blocks
below; every example is given both as a script (the main path) and, where it
matters, as the atlas JSON.

There are four kinds of block:

1. [Area](#1-area): where
2. [Land](#2-land): the batches on land
3. [Water](#3-water): the `water` block and its batches
4. [Shoreline](#4-shoreline): `water.shoreline`

---

## Full example

```js
burin.area({ at: "44.3035,9.2145", size_km: [14, 10] })

burin.batch({ angle: 0, style: { space_light: 4.8 } })

burin.water("lines", { follow: "shore", style: { water_start: 10, water_reach: 30, space_light: 7 } })
burin.shore({ lines: 2, style: { space_light: 4 } })
burin.draw()
```

The same drawing as a saved atlas (`--style-file`):

```yaml
area:
  at: "44.3035,9.2145"
  size_km: [14, 10]

batches:
  - angle: 0
    style:
      space_light: 4.8

water:
  classes: [ocean, lake]
  shoreline:
    lines: 2
    style:
      space_light: 4
  batches:
    - follow: shore
      style:
        water_start: 10
        water_reach: 30
        space_light: 7
```

---

## 1. Area

```yaml
area:
  at: "lat,lon"
  size_km: [14, 10]
```

| Field | What it does |
|---|---|
| `place` | named area: `bern`, `matterhorn`, `san-francisco` |
| `at` | centre, `"lat,lon"`; dragging the map changes it |
| `bounds` | `[west, south, east, north]` in degrees |
| `size_km` | ground width [and height], km |
| `size_mm` | paper width [and height], mm |
| `scale` | map scale, the N in 1:N |
| `pen_mm` | pen width on paper, mm |

---

## 2. Land

```yaml
batches:
  - name: "optional"
    angle: 0
    style:
      space_light: 4.8
```

`land: { batches: [...] }` is the long form of the same thing. Use one or the other, not both.

| Field | What it does |
|---|---|
| `name` | a label in the app, optional |
| `layer` | SVG layer id on export (`<g id="…">`); optional |
| `color` | stroke colour as `#rgb` or `#rrggbb`; black when omitted |
| `color_expr` | colour from a small expression; see [Colour expressions](#colour-expressions) |
| `width_expr` | line width as a multiple of `batch_stroke`, from an expression; see [Line width and ends](#line-width-and-ends) |
| `width_steps` | rounds those widths to this many pens, each its own layer; 0 (default) keeps every width |
| `cap` | line ends: `round` (default), `round_inside`, `square` or `butt` |
| `angle` | 0 follows the contour lines, 90 runs straight downhill |
| `ground` | `land` (default) or `sea`: `sea` draws the seabed instead, and nothing on land |
| `draw` | `lines` (default), `dots`: stippling, denser where lines would be (see [Land dots](#land-dots)), or `fill`: a flat area (see [Fills](#fills)) |
| `where` | for a fill, the heights or depths it keeps to: `elev_min`, `elev_max`, `depth_min`, `depth_max`, in metres |
| `style` | parameters, see below; those left out keep their defaults, except the ones that carry over from the batch before (see [Carries over](#carries-over)) |

`name` is only for you; `layer` is what plotters and SVG editors group by.

`ground: sea` turns the map inside out: the batch traces the seabed's depth like hills, and land is left blank. The water outline decides where the sea is, so give the document a `water` block (a `shoreline` and no water batches draws just the coast); without one, sea is wherever the elevation is at or below 0.5 m. The depth comes from the same elevation tiles (Terrarium), which carry bathymetry, in the web app and on the command line alike.

```yaml
batches:
  - angle: 0
    ground: sea
    style: { space_light: 15, space_dark: 4.5, input_blur: 8 }
water:
  shoreline: { lines: 1 }
  batches: []
```

### Shade or height

By default the shading sets how dense the lines are: dark slopes get them close, lit ones far apart. `input_height` blends in the height instead: at `1` the highest ground is darkest, at `-1` the lowest, and `0` (the default) is shade only. On a seabed batch `-1` darkens the deepest water, so one land batch at `0.4` and one `ground: sea` batch at `-0.5` read as a single relief across the coast, hills rising and the seabed falling away.

```yaml
batches:
  - { angle: 0, style: { input_height: 0.4 } }
  - { angle: 0, ground: sea, style: { space_light: 15, space_dark: 4.5, input_blur: 8, input_height: -0.5 } }
```

### Land dots

`draw: dots` stipples the land instead of tracing lines: the dots sit as close as lines would (`space_light`, `space_dark`, `space_steep`, `input_height`), and none are drawn where the input is below `dash_cutoff` (they fade in with `dash_cutoff_soft`, as dashes do). The dot size and clumping come from the water parameters `water_size` (default 1.2 on land) and `water_patches`, as for dots at sea: `mod_jitter` varies their size and `mod_wobble` bends them. Ripples are only for water.

```yaml
batches:
  - angle: 0
    draw: dots
    style: { space_light: 6.6, space_dark: 1.5, space_steep: 0.3 }
```

### Fills

`draw: fill` paints an area flat instead of drawing lines: the land, or with `ground: sea` the sea, optionally only between some heights or depths (`where`). The edge follows the same water outline as the shoreline, so a land fill and a sea fill meet exactly at the coast, and both are closed along the frame. The colour is `color` (or a plain `#hex` `color_expr`), light grey when left out; the SVG layer is `layer`, `fill` when left out.

Layers stack in batch order, the first at the bottom: a fill covers whatever the batches before it drew, and the batches after it draw over it. Land batches come first, then the water batches, then the shoreline, so the water's lines are always above every fill. Batches with the same pen still share a layer, but never across a fill.

| `where` key | Keeps the fill to |
|---|---|
| `elev_min` / `elev_max` | ground at least / at most this high, m |
| `depth_min` / `depth_max` | sea floor at least / at most this deep, m |

A limit left out is open. Depths come from the same elevation tiles as `ground: sea`. Of the style only `input_blur` matters: it smooths the band's edges, px (the coast is never smoothed); raise it where coarse elevation tiles leave steps. A fill's style is its own: it doesn't carry over to the batches after it (see [Carries over](#carries-over)). Fills don't carry `angle`, and the width and cap settings mean nothing to them.

A plotter has no use for fills; they are for the screen, print and SVG editors. The PNG paints them, the SVG has them as `<g fill="…" fill-rule="evenodd">` groups, and the GeoJSON adds one MultiPolygon feature per fill with its `fill` colour and `layer`.

```yaml
batches:
  - { draw: fill, ground: sea, color: '#d6e6ef' }                                # all the sea
  - { draw: fill, ground: sea, color: '#a0bfd5', where: { depth_min: 1000 } }    # darker past 1000 m
  - { draw: fill, color: '#eef0df' }                                             # all the land
  - { draw: fill, color: '#d8cfa4', where: { elev_min: 300 } }                   # warmer above 300 m
  - { angle: 0, color: '#5c4630' }                                               # contours over the tints
water:
  shoreline: { lines: 1 }
  batches: []
```

### Colour expressions

`color` and `color_expr` are on each batch (land or water), not in `style`, because colour is not a number like `space_light`.

- `color: "#2a1810"` sets a fixed stroke colour for the whole batch.
- `color_expr: "#336699"` works the same (a lone hex literal).
- If both are set, `color` wins.
- Any other `color_expr` is evaluated at each dash from the point where it sits (paths are grouped by resulting colour).

| Name | On land | On water |
|---|---|---|
| `elev` | height at the point, metres | — |
| `dist` | — | distance from the shore, px |
| `depth` | — | metres below the surface (when depth is active) |
| `tone` | shading input 0…1 (light/dark) | water tone from the field input |
| `lerp(a, b, t)` | mix two colours, or two numbers; `t` 0…1 | same |
| `clamp(x, lo, hi)` | scalar | same |
| `+ - * /`, `-x` | on scalars | same |

Examples:

```yaml
batches:
  - angle: 0
    color_expr: 'lerp(#e8dcc8, #2a1810, tone)'   # contours lighter in sun, darker in shade
  - angle: 90
    color_expr: 'lerp(#c8e8ff, #1a3a5c, dist/120)'   # water lines (needs per-point eval)
```

Land batches with different colours stay separate in the SVG even when they share the same line width.

### Line width and ends

`batch_stroke` sets one width for a whole batch. `width_expr` varies it from dash to dash: it is a scalar built like the ones above, evaluated where each dash sits, and it multiplies `batch_stroke`. A long line is evaluated every 8 px or so, so it thickens and thins along its way, and so does its colour with `color_expr`. At 1 a dash is `batch_stroke` wide, at 2 twice that; at 0 or below it is left out. It works on land and water batches and on the shoreline, for lines and for dots alike: a dot is a very short line, so a wider stroke is a bigger dot.

```yaml
batches:
  - angle: 0
    width_expr: 'lerp(0.4, 2.5, clamp(elev / 300, 0, 1))'   # thin in the valleys, bold on the hills
    width_steps: 3
    layer: hills
    style: { space_light: 6, space_dark: 4.5 }
water:
  shoreline: { lines: 1, width_expr: 'lerp(2, 0.8, clamp(dist / 40, 0, 1))' }
  batches:
    - follow: shore
      draw: dots
      width_expr: 'lerp(2.6, 0.5, clamp(dist / 150, 0, 1))'   # big dots at the coast, fine ones offshore
```

Each width is its own group in the SVG, rounded to a hundredth of a px. That suits the screen and print. For a pen plotter, `width_steps: 3` rounds the widths to 3, evenly spaced from the thinnest dash to the widest, each in its own layer named after the batch's `layer` (`hills-1`, `hills-2`, `hills-3`; `width-1`… without one), so each can get its own pen. Wide lines need room: raise `space_light` and `space_dark` along with the width, or the darkest areas fill in.

`cap` picks how the dashes end:

| `cap` | Ends |
|---|---|
| `round` (default) | rounded, half the width past each end, like a pen |
| `round_inside` | rounded, but within the ends: each dash is shortened by half its width, and one shorter than its width becomes a dot |
| `square` | square, half the width past each end |
| `butt` | flat, exactly at the ends |

A plotter's pen always ends round, so `cap` shows on screen, in print and in SVG editors. Dots want `round` or `square`: `butt` and `round_inside` shrink them.

### Land parameters

| Group | Parameter | Default | Carries over |
|---|---|---|---|
| Space | `space_light` | 6 | ✓ |
|  | `space_dark` | 7 | ✓ |
|  | `space_steep` | 0.4 | ✓ |
|  | `space_steep_gamma` | 1 | ✓ |
| Dash | `dash_light` | 0.3 |  |
|  | `dash_dark` | 0.8 |  |
|  | `dash_mid` | -1 |  |
|  | `dash_period` | 10.5 |  |
|  | `dash_levels` | 0 |  |
|  | `dash_cutoff` | 0.15 |  |
|  | `dash_cutoff_soft` | 1 |  |
|  | `dash_solid` | 1 |  |
| Mod | `mod_jitter` | 1 |  |
|  | `mod_flat` | 0.5 |  |
|  | `mod_breaks` | 0 |  |
|  | `mod_wobble` | 20 | ✓ |
|  | `mod_wobble_size` | 12 | ✓ |
|  | `mod_wave` | 0 |  |
|  | `mod_wave_size` | 16 |  |
|  | `mod_wave_zone` | 0 |  |
|  | `mod_wave_even` | 0 |  |
|  | `mod_smooth` | 6 | ✓ |
| Input | `input_flat` | 0.25 | ✓ |
|  | `input_gain` | 3 | ✓ |
|  | `input_dark_gamma` | 0.7 | ✓ |
|  | `input_light_gain` | 1.5 | ✓ |
|  | `input_light_floor` | 0.06 | ✓ |
|  | `input_blur` | 1.8 | ✓ |
|  | `input_broad_blur` | 11.5 | ✓ |
|  | `input_broad` | 0.5 | ✓ |
|  | `input_height` | 0 |  |
| Sun | `sun_azimuth` | 315 | ✓ |
|  | `sun_altitude` | 45 | ✓ |
|  | `sun_exag` | 4 | ✓ |
| Engine | `engine_step` | 1 |  |
|  | `engine_min` | 4 |  |
|  | `engine_stop` | 0.5 |  |
|  | `engine_fill` | 0.8 |  |
|  | `engine_stripes` | 0 |  |
| Batch | `batch_floor` | 0 |  |
|  | `batch_stroke` | 1.2 |  |
|  | `batch_seed` | 1 |  |

### Carries over

Some parameters shape what a batch follows rather than how it inks: the shading (`input_*`, `sun_*`), the spacing it sets (`space_light`, `space_dark`, `space_steep`, `space_steep_gamma`), and how lines wander and smooth (`mod_wobble`, `mod_wobble_size`, `mod_smooth`). These are marked ✓ above. A batch that leaves one out takes it from the batch above it; the first batch takes the default. Every other parameter left out falls back to its default.

This is what makes multi-pass drawings line up: a crosshatch at 45° over contours at 0° sees the same light and the same spacing field as the pass beneath it, without repeating them. Fills follow nothing, so they are skipped: they neither take these parameters nor hand them on.

```yaml
batches:
  - angle: 0
    style:
      sun_azimuth: 270    # light from the west
      dash_light: 0.5
  - angle: 90             # still sun_azimuth 270; dash_light is back to 0.3
  - angle: 45
    style:
      sun_azimuth: 135    # this one, and any batch after it, lit from the south-east
```

So one sun for the whole map needs setting only once, in the first batch, and a different sun on one pass is still possible. The same holds on the command line, section by section after each `--batch`.

### Hand-drawn waves

`mod_wave` makes every line swing from side to side along its path, as a hand draws it: `mod_wave` px to either side, about once every `mod_wave_size` px. Both drift along the way. It works on land, on water and on the shoreline, and is off (0) by default.

- `mod_wave_zone: 0`: each line waves its own way, so no two lines wave alike.
- `mod_wave_zone: 1`: neighbouring lines wave together, so parallel lines keep an even spacing. It needs lines well under `mod_wave_size` / 4 px apart.
- `mod_wave_even: 1`: every line waves evenly and in step with its neighbours along the flow, each an offset copy of the next, as in an engraved pattern. Only on traced lines (land batches and water `lines`), which have a flow to keep step with.

On land it reads best when light: `mod_wave: 0.6, mod_wave_size: 40`. Stronger waves on close lines look cross-hatched.

---

## 3. Water

```yaml
water:
  classes: [ocean, lake, river, dock]
  source: "https://tiles.openfreemap.org/planet"
  shoreline: true
  batches:
    - name: "optional"
      follow: shore
      draw: lines
      angle: 0
      style:
        water_reach: 60
```

With a `water` block, the water is downloaded from OpenStreetMap and decides where the land ends. Without one, nothing is downloaded and land is whatever lies above 0.5 m.

| Field | What it does | Default |
|---|---|---|
| `classes` | which water: `ocean`, `lake`, `river`, `dock`, `pond`, `swimming_pool` | ocean, lake, river, dock |
| `source` | TileJSON URL or `{z}/{x}/{y}` template | OpenFreeMap |
| `shoreline` | the shoreline: `true`, `false` or a block, see [Shoreline](#4-shoreline) | `true` |
| `batches` | passes of lines on the water | none: the shoreline only |

### Water batch fields

| Field | What it does | Default |
|---|---|---|
| `name` | a label, optional | |
| `follow` | `shore`: concentric lines along the coast; `map`: one direction over the whole map | `shore` |
| `draw` | `lines` fill the water; `ripples` draw broken rows of hand-drawn waves; `dots` stipple it; see [Ripples and dots](#ripples-and-dots) | `lines` |
| `angle` | with `shore`, 0 runs along the coast and 90 straight out to sea; with `map`, degrees clockwise from north | 0 |
| `layer`, `color`, `color_expr`, `width_expr`, `width_steps`, `cap` | as on land; see [Colour expressions](#colour-expressions) and [Line width and ends](#line-width-and-ends) | |
| `where` | optional depth window in metres (`depth_min`, `depth_max`); see [Depth](#depth) | |
| `style` | parameters, see below | |

### Depth

```yaml
bathymetry:
  source: terrarium   # default: reuse elevation (no extra fetch)

water:
  batches:
    - follow: shore
      where: { depth_max: 100 }   # shallow shelf: concentric lines only
      style: { space_light: 5.4, space_dark: 2.4, water_start: 3, water_reach: 4000, water_fade: 0.35 }
      # water_reach in px must be large: it is distance-from-shore, not depth.
      # depth_max ends the shelf; a tiny water_reach leaves white bands on wide shelves.
    - follow: map
      where: { depth_min: 100 }   # same depth: engraved waves in deep water
      style:
        mod_wave: 2.5
        mod_wave_size: 30
        mod_wave_even: 1
        mod_wobble: 45
        water_fray: 40
```

See `examples/ocean-depth-zones.json5` (shelf + the same look as `ocean-engraved.json5` offshore).

| `bathymetry.source` | Fetch |
|---|---|
| absent, `terrarium`, `auto`, `dem` | none; depth from elevation when needed |
| `gebco`, `url`, or an `http…` / `{z}/{x}/{y}` template | separate bathymetry tiles (reserved for precise charts) |

In `color_expr` on water, `depth` is metres below the surface (same as `where`).

| `where.depth_fade` | metres of soft blend at depth_min / depth_max (default 30) |
| `water_depth_fray` (`wdf`) | metres band at that depth limit where lines break into dashes/dots, like `water_fray` at the shore |

Without `depth_fade` and `water_depth_fray`, depth limits used to cut ink off in one pixel — a white band between shelf and open-sea batches. `water_reach` is still distance-from-shore in px, not depth.

### Water parameters

The land's names, minus those that need terrain. The defaults differ, and there are three more.

On water there is no sun shading on a flat sea. The same **names** as land are reused for **distance from the shore**:

| Name | On land | On water |
|---|---|---|
| `space_dark` / `space_light` | tighter / wider line spacing in shadow / light | tighter / wider spacing **near / far** from the coast (via `water_space_reach`) |
| `dash_dark` / `dash_light` | more / less ink in shadow / light | more / less ink **near / far** from the coast (via `water_reach`) |

Lower `space_dark` (e.g. 2 px) means **closer together** near the shore, not wider. To loosen shelf rings, **raise** `space_dark`, or lengthen `water_space_reach`.

`follow: shore` with `angle: 0` draws **distance contours** (true concentrics), not the land streamline tracer — so changing `space_dark` cannot cross the rings. A second batch at another angle (e.g. engraved open sea at 90°) can still cross the first in the `depth_fade` overlap; that reads as cross-hatching.

Ink and tracing extent use `water_reach`; **line spacing** uses `space_light`, `space_dark` and `water_space_reach` only. A second water batch at another angle (e.g. engraved sea at 90°) can cross the first in the depth-fade overlap — that reads as cross-hatching, not broken spacing.

**Depth** (metres below the surface) is optional. Nothing extra is downloaded unless a batch asks for depth *and* the document sets `bathymetry.source` to something other than `terrarium` / `auto` (GEBCO or a custom tile URL — not wired in the web app yet). By default, depth comes from the **same elevation mosaic** already fetched for the map (negative heights on water pixels). The depth grid is built only when a water batch has a `where` depth limit or a `color_expr` or `width_expr` that uses `depth`.

| Group | Parameter | Default | Notes |
|---|---|---|---|
| Space | `space_light` | 7.2 | spacing far from the shore, px (wider rings) |
| | `space_dark` | 3.2 | spacing near the shore, px (tighter rings); not “shadow” |
| Dash | `dash_light` | 0.25 | ink far from the shore |
| | `dash_dark` | 1 | ink near the shore |
| | `dash_period` | 10.5 | |
| | `dash_cutoff` | 0 | |
| Mod | `mod_jitter` | 0 | |
| | `mod_flat` | 0.6 | leaves out dashes where the tone is below `input_flat` |
| | `mod_breaks` | 0 | |
| | `mod_wobble` | 0 | |
| | `mod_wobble_size` | 12 | |
| | `mod_wave` | 0 | see [Hand-drawn waves](#hand-drawn-waves) |
| | `mod_wave_size` | 16 | |
| | `mod_wave_zone` | 0 | |
| | `mod_wave_even` | 0 | |
| | `mod_smooth` | 3 | |
| Input | `input_flat` | 0.25 | the tone below which `mod_flat` works |
| Engine | `engine_step` | 1 | |
| | `engine_min` | 4 | |
| | `engine_stop` | 0.5 | |
| | `engine_fill` | 0.8 | |
| Batch | `batch_stroke` | 1.2 | |
| | `batch_seed` | 1 | |
| Water | `water_start` | 2 | gap between the shore and the first line, px |
| | `water_reach` | 60 | how far out ink and tracing stay active, px; not line spacing |
| | `water_space_reach` | 120 | how far out spacing eases from `space_dark` to `space_light`, px; 0 = fixed `space_light` |
| | `water_fade` | 1 | how they fade out: above 1 sooner, below 1 later |
| | `water_fray` | 0 | how far out from `water_start`, px, the lines break up into shorter dashes and dots towards the shore, with a ragged edge; 0 a clean edge |

Left out, as they mean nothing on water: `space_steep`, `space_steep_gamma`, `batch_floor`, the `sun_*` group and the rest of the `input_*` group.

### Currents

`follow: map` lines with a strong `mod_wobble` turn into swirling currents; `mod_wave` with `mod_wave_zone: 1` makes them wave together. `water_start` keeps them clear of the concentric lines along the coast: set it a few px beyond where those end (`water_start` + `water_reach` of the coast batch). Beautiful on screen, very long to plot. See `examples/ocean-currents.json5`.

```yaml
water:
  batches:
    - follow: shore        # concentric lines along the coast, out to 23 px
      style: { space_light: 5.4, space_dark: 2.4, water_start: 3, water_reach: 20 }
    - follow: map
      angle: 90
      style:
        space_light: 5
        space_dark: 5
        water_start: 27    # clear of the coast lines
        water_reach: 4000
        dash_light: 0.85
        mod_wobble: 70     # the flow turns up to 70 degrees
        mod_wobble_size: 60
        mod_wave: 2.5
        mod_wave_size: 60
        mod_wave_zone: 1   # neighbours wave together
```

### Ripples and dots

With `draw: ripples` or `dots`, a batch covers the open sea with something other than traced lines:

| `draw` | What it draws |
|---|---|
| `ripples` | rows `space_light` px apart, broken into strokes about `water_size` px long, bent together by a swell and waving by hand, as in Olaus Magnus' Carta Marina |
| `dots` | stippling, closer near the coast, as on Ortelius' maps |

```yaml
water:
  batches:
    - follow: map          # rows at `angle` from north; shore: along the coast
      angle: 90
      draw: ripples
```

`follow: map` with `angle: 90` keeps ripple rows level; `follow: shore` runs them along the coast. `water_patches` gathers either kind in patches, leaving open sea in between. Several batches can be combined: lines near the coast and ripples further out.

Ripples use:

| Group | Parameter | Default | What it does |
|---|---|---|---|
| Space | `space_light` | 9 | gap between rows, px |
| Dash | `dash_dark` | 0.9 | how much of a row is inked at `water_start` |
| | `dash_light` | 0.6 | how much is inked from `water_reach` further out |
| Mod | `mod_wobble` | 18 | how much the swell bends the rows, degrees |
| | `mod_wobble_size` | 150 | length of each swell, px |
| | `mod_wave` | 1.5 | hand-drawn wave on each stroke, px |
| | `mod_wave_size` | 40 | length of each wave, px |
| | `mod_wave_zone` | 0 | how much neighbouring rows wave together |
| | `mod_smooth` | 3 | rounds off rows along the coast (`follow: shore`) |
| Batch | `batch_stroke` | 1.2 | line width |
| | `batch_seed` | 1 | random seed |
| Water | `water_start` | 4 | no ripples nearer the shore than this, px |
| | `water_reach` | 300 | distance over which the ink eases from `dash_dark` to `dash_light`, px |
| | `water_fade` | 1 | how it eases |
| | `water_size` | 140 | length of each stroke, px |
| | `water_patches` | 0 | how much ripples gather in patches: 0 evenly everywhere, 1 a few patches |

Dots use:

| Group | Parameter | Default | What it does |
|---|---|---|---|
| Space | `space_dark` | 2.25 | distance between dots at `water_start`, px |
| | `space_light` | 6.5 | distance between dots from `water_reach` further out, px |
| Mod | `mod_jitter` | 0 | how much dot size varies |
| Batch | `batch_stroke` | 1.2 | line width |
| | `batch_seed` | 1 | random seed |
| Water | `water_start` | 3 | no dots nearer the shore than this, px |
| | `water_reach` | 250 | distance over which the spacing eases from `space_dark` to `space_light`, px |
| | `water_fade` | 1 | how it eases |
| | `water_size` | 1.2 | length of each dot, px |
| | `water_patches` | 0 | how much dots gather in patches |

---

## 4. Shoreline

```yaml
water:
  shoreline: true
```

```yaml
water:
  shoreline: false
```

```yaml
water:
  shoreline:
    lines: 2
    style:
      space_light: 4
```

| Field | What it does | Default |
|---|---|---|
| `lines` | how many: the shore, then more out to sea, `space_light` px apart | 1 |
| `layer`, `color`, `color_expr`, `width_expr`, `width_steps`, `cap` | as on land; the layer is `shoreline` unless set | |
| `style` | parameters, see below | one clean, solid line |

### Shoreline parameters

Only those that mean something on a line that is already drawn.

| Group | Parameter | Default | What it does |
|---|---|---|---|
| Space | `space_light` | 3 | gap between lines, when `lines` is above 1 |
| Dash | `dash_dark` | 1 | ink on the line at the shore (1 is solid) |
| | `dash_light` | 1 | ink on the lines further out (1 is solid) |
| | `dash_period` | 10.5 | length of one dash plus gap, px |
| | `dash_cutoff` | 0 | no ink below this tone; `dash_cutoff_soft` fades it in instead |
| Mod | `mod_jitter` | 0 | random variation in dash length, same average ink |
| | `mod_breaks` | 0 | chance of a small break in the line |
| | `mod_wobble` | 0 | hand-drawn wander, degrees |
| | `mod_wobble_size` | 12 | length of each wander, px |
| | `mod_wave` | 0 | hand-drawn wave, px; see [Hand-drawn waves](#hand-drawn-waves) |
| | `mod_wave_size` | 16 | length of each wave, px |
| | `mod_wave_zone` | 0 | how much the lines wave together |
| | `mod_smooth` | 0 | rounds off the pixel steps, px |
| Batch | `batch_stroke` | 1.2 | the shoreline's own line width |
| | `batch_seed` | 1 | random seed |
| Water | `water_reach` | 60 | distance over which lines further out lighten, px |
| | `water_fade` | 1 | how they lighten |

### Shoreline recipes

| Look | `shoreline` |
|---|---|
| clean double line | `{ lines: 2, style: { space_light: 4 } }` |
| dashed | `{ style: { dash_dark: 0.6 } }` |
| hand-drawn | `{ style: { mod_wobble: 25, mod_smooth: 2 } }` |
| thicker | `{ style: { batch_stroke: 2 } }` |

---

## On the command line

Run a document with `--script`:

```bash
burin-map --script presets/contours.js --out-file bay.png
```

A saved atlas is still one JSON argument to `--water`, or a `--style-file` /
`--style-json` document:

```bash
burin-map --terrain-at "44.3035,9.2145" --terrain-size 14km \
  --water '{"shoreline":{"lines":2,"style":{"space_light":4}},"batches":[{"follow":"shore","style":{"water_start":10,"water_reach":30,"space_light":7}}]}'
```

## In a share link

A `#d=…` link has three sections separated by `|`: area, land and water. For example:

```
#d=at44.3035,9.2145,ts14x10|a0|l2,s4;shore,a0,wst10,wr30,s7
```

- **Land**: `a0`, one batch at angle 0; `sea,a0` is a batch with `ground: sea`, `dots,a0` one with `draw: dots`, and `fill,a0` one with `draw: fill` (its colour and `where` stay out of the link, as the other ink settings do).
- **Water**:
  - before the first `;` comes the shoreline: `l2,s4` is 2 lines, 4 px apart (`noshore` turns it off, `c…` picks the classes);
  - then one batch after each `;`: `shore,a0,wst10,wr30,s7`.
  - a batch that draws ripples or dots says so after `shore` / `map`: `map,ripples,a90`.

---

## 5. JavaScript documents

A document is a script: it calls `burin.*` and ends with `burin.draw()`. Each
call appends one entry to the drawing, in order, so what you write is what the
engine draws:

```js
// the sea floor, pale to blue, deepening every 100 m
const sea = ["#d6e6ef", "#a9c6db", "#7ba3c0"]
for (let i = 0; i < sea.length; i++)
  burin.batch({ ground: "sea", draw: "fill", color: sea[i], where: { depthMin: i * 100 } })

// contours coloured by the shading, and the coast
burin.batch({ angle: 0, shade: burin.lerp("#e8dcc8", "#2a1810", burin.tone) })
burin.shore({ color: "#1d3a52" })
burin.draw()
```

- `burin.batch(options)` — one land pass: the same keys as an atlas batch
  (`angle`, `ground`, `draw`, `color`, `shade`, `width`, `where`, `style`, and
  any parameter by name). `draw: "fill"` makes it a flat fill.
- `burin.water(kind, options)` — a pass over the sea; `kind` is `"lines"`,
  `"ripples"` or `"dots"`. `burin.shore(options)` — the shoreline.
- `burin.area(options)` — where to draw: the same fields as the area block.
- `burin.draw()` — hand the built drawing to the engine. `burin.json()` returns
  it as JSON instead (what a saved atlas is), and `burin.parameters()` is the
  parameter catalogue.
- The fields `burin.elev`, `burin.tone`, `burin.dist` and `burin.depth`, and
  the helpers `burin.lerp`, `rgb`, `clamp`, `smoothstep`, `step`, `abs`,
  `floor`, `ceil`, `round`, `min`, `max` and `mix`, build the same colour and
  width trees that a saved atlas writes as `color_expr` / `width_expr`.

Calls accumulate; there is no builder object to hold. A loop, a condition or a
function is why this is code and not a data format — "a belt for every 100 m",
"union these areas", "the same pass for each of these angles".

The script runs in a sandbox — no clock, no network, no `Math.random`, capped
loops and recursion — so the same script always draws the same map. A saved
atlas reads back with both the camelCase names a script writes (`elevMin`) and
the snake_case ones an atlas saves (`elev_min`), so a drawing and the script
that made it are two views of one thing.
