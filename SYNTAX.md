# burin-map document syntax

A document describes a whole drawing. Write it in YAML or JSON5 in the web editor, or pass it to the CLI with `--style-file`.

There are four kinds of block:

1. [Area](#1-area): where
2. [Land](#2-land): the batches on land
3. [Water](#3-water): the `water` block and its batches
4. [Shoreline](#4-shoreline): `water.shoreline`

---

## Full example

```yaml
area:
  at: "44.3035,9.2145"
  size_km: [14, 10]

batches:
  - angle: 0
    style:
      space: 4.8

water:
  classes: [ocean, lake]
  shoreline:
    lines: 2
    style:
      space: 4
  batches:
    - follow: shore
      style:
        water_start: 10
        water_reach: 30
        space: 7
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
      space: 4.8
```

`land: { batches: [...] }` is the long form of the same thing. Use one or the other, not both.

| Field | What it does |
|---|---|
| `name` | a label in the app, optional |
| `layer` | SVG layer id on export (`<g id="…">`); optional |
| `color` | stroke colour as `#rgb` or `#rrggbb`; black when omitted |
| `color_expr` | colour from a small expression; see [Colour expressions](#colour-expressions) |
| `angle` | 0 follows the contour lines, 90 runs straight downhill |
| `style` | parameters, see below; those left out keep their defaults, except the terrain ones (see [Terrain carries over](#terrain-carries-over)) |

`name` is only for you; `layer` is what plotters and SVG editors group by.

### Colour expressions

`color` and `color_expr` are on each batch (land or water), not in `style`, because colour is not a number like `space`.

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
| `lerp(a, b, t)` | mix two colours; `t` 0…1 | same |
| `clamp(x, lo, hi)` | scalar | same |
| `+ - * /` | on scalars | same |

Examples:

```yaml
batches:
  - angle: 0
    color_expr: 'lerp(#e8dcc8, #2a1810, tone)'   # contours lighter in sun, darker in shade
  - angle: 90
    color_expr: 'lerp(#c8e8ff, #1a3a5c, dist/120)'   # water lines (needs per-point eval)
```

Land batches with different colours stay separate in the SVG even when they share the same line width.

### Land parameters

| Group | Parameter | Default | Terrain |
|---|---|---|---|
| Space | `space` | 4.8 |  |
|  | `space_light` | 1 | yes |
|  | `space_dark` | 0.7 | yes |
|  | `space_steep` | 0.4 | yes |
|  | `space_steep_gamma` | 1 | yes |
| Dash | `dash_light` | 0.15 |  |
|  | `dash_dark` | 1 |  |
|  | `dash_period` | 2.2 |  |
|  | `dash_cutoff` | 0.03 |  |
|  | `dash_drop` | 1 |  |
| Mod | `mod_jitter` | 0.45 |  |
|  | `mod_breaks` | 0.15 |  |
|  | `mod_wobble` | 20 | yes |
|  | `mod_wobble_size` | 12 | yes |
|  | `mod_wave` | 0 |  |
|  | `mod_wave_size` | 16 |  |
|  | `mod_wave_zone` | 0 |  |
|  | `mod_wave_even` | 0 |  |
|  | `mod_smooth` | 6 | yes |
| Input | `input_flat` | 0.25 | yes |
|  | `input_gain` | 3 | yes |
|  | `input_dark_gamma` | 0.7 | yes |
|  | `input_light_gain` | 1.5 | yes |
|  | `input_light_floor` | 0.06 | yes |
|  | `input_blur` | 1.8 | yes |
|  | `input_broad_blur` | 11.5 | yes |
|  | `input_broad` | 0.5 | yes |
| Sun | `sun_azimuth` | 315 | yes |
|  | `sun_altitude` | 45 | yes |
|  | `sun_exag` | 4 | yes |
| Engine | `engine_step` | 1 |  |
|  | `engine_min` | 4 |  |
|  | `engine_stop` | 0.5 |  |
|  | `engine_fill` | 0.8 |  |
| Batch | `batch_floor` | 0 |  |
|  | `batch_stroke` | 1.2 |  |
|  | `batch_seed` | 1 |  |

### Terrain carries over

The terrain parameters (marked above: the `input_*` and `sun_*` groups, `space_light`, `space_dark`, `space_steep`, `space_steep_gamma`, `mod_wobble`, `mod_wobble_size`, `mod_smooth`) shape the terrain a batch follows. A batch that leaves one out takes it from the batch above; the first batch takes the default. Every other parameter always falls back to its default.

```yaml
batches:
  - angle: 0
    style:
      sun_azimuth: 270    # light from the west
      space: 6
  - angle: 90             # still sun_azimuth 270; space is back to 4.8
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
| `where` | optional depth window in metres (`depth_min`, `depth_max`); see [Depth](#depth) | |
| `style` | parameters, see below | |

### Depth

```yaml
bathymetry:
  source: terrarium   # default: reuse elevation (no extra fetch)

water:
  batches:
    - follow: shore
      where: { depth_max: 200 }   # shelf only
      style: { water_start: 3, water_reach: 60, space: 3 }
    - follow: map
      draw: ripples
      where: { depth_min: 400 }   # deep ocean only
      style: { mod_wave: 2.5, mod_wave_size: 70 }
```

| `bathymetry.source` | Fetch |
|---|---|
| absent, `terrarium`, `auto`, `dem` | none; depth from elevation when needed |
| `gebco`, `url`, or an `http…` / `{z}/{x}/{y}` template | separate bathymetry tiles (reserved for precise charts) |

In `color_expr` on water, `depth` is metres below the surface (same as `where`).

### Water parameters

The land's names, minus those that need terrain. The defaults differ, and there are three more.

On water, the "tone" that land takes from terrain and sun is the distance from the shore instead: 1 at the first line (`water_start` out), fading to 0 `water_reach` px further out. That tone drives `space_light` / `space_dark` and `dash_light` / `dash_dark` exactly as shading does on land.

**Depth** (metres below the surface) is optional. Nothing extra is downloaded unless a batch asks for depth *and* the document sets `bathymetry.source` to something other than `terrarium` / `auto` (GEBCO or a custom tile URL — not wired in the web app yet). By default, depth comes from the **same elevation mosaic** already fetched for the map (negative heights on water pixels). The depth grid is built only when a water batch has a `where` depth limit or a `color_expr` that uses `depth`.

| Group | Parameter | Default | Notes |
|---|---|---|---|
| Space | `space` | 4 | |
| | `space_light` | 1.8 | spacing far from the shore |
| | `space_dark` | 0.8 | spacing near the shore |
| Dash | `dash_light` | 0.25 | ink far from the shore |
| | `dash_dark` | 1 | ink near the shore |
| | `dash_period` | 2.2 | |
| | `dash_cutoff` | 0.03 | |
| | `dash_drop` | 0.6 | leaves out dashes where the tone is below `input_flat` |
| Mod | `mod_jitter` | 0.45 | |
| | `mod_breaks` | 0.15 | |
| | `mod_wobble` | 0 | |
| | `mod_wobble_size` | 12 | |
| | `mod_wave` | 0 | see [Hand-drawn waves](#hand-drawn-waves) |
| | `mod_wave_size` | 16 | |
| | `mod_wave_zone` | 0 | |
| | `mod_wave_even` | 0 | |
| | `mod_smooth` | 3 | |
| Input | `input_flat` | 0.25 | the tone below which `dash_drop` works |
| Engine | `engine_step` | 1 | |
| | `engine_min` | 4 | |
| | `engine_stop` | 0.5 | |
| | `engine_fill` | 0.8 | |
| Batch | `batch_stroke` | 1.2 | |
| | `batch_seed` | 1 | |
| Water | `water_start` | 2 | gap between the shore and the first line, px |
| | `water_reach` | 60 | how far out the lines go, px; lower means fewer lines |
| | `water_fade` | 1 | how they fade out: above 1 sooner, below 1 later |
| | `water_fray` | 0 | how far out from `water_start`, px, the lines break up into shorter dashes and dots towards the shore, with a ragged edge; 0 a clean edge |

Left out, as they mean nothing on water: `space_steep`, `space_steep_gamma`, `batch_floor`, the `sun_*` group and the rest of the `input_*` group.

### Currents

`follow: map` lines with a strong `mod_wobble` turn into swirling currents; `mod_wave` with `mod_wave_zone: 1` makes them wave together. `water_start` keeps them clear of the concentric lines along the coast: set it a few px beyond where those end (`water_start` + `water_reach` of the coast batch). Beautiful on screen, very long to plot. See `examples/ocean-currents.json5`.

```yaml
water:
  batches:
    - follow: shore        # concentric lines along the coast, out to 23 px
      style: { space: 3, water_start: 3, water_reach: 20 }
    - follow: map
      angle: 90
      style:
        space: 5
        space_light: 1
        space_dark: 1
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
| `ripples` | rows `space` px apart, broken into strokes about `water_size` px long, bent together by a swell and waving by hand, as in Olaus Magnus' Carta Marina |
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
| Space | `space` | 9 | gap between rows, px |
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
| Space | `space` | 5 | distance between dots, px |
| | `space_dark` | 0.45 | spacing multiplier at `water_start` |
| | `space_light` | 1.3 | spacing multiplier from `water_reach` further out |
| Mod | `mod_jitter` | 0.45 | how much dot size varies |
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
      space: 4
```

| Field | What it does | Default |
|---|---|---|
| `lines` | how many: the shore, then more out to sea, `space` px apart | 1 |
| `style` | parameters, see below | one clean, solid line |

### Shoreline parameters

Only those that mean something on a line that is already drawn.

| Group | Parameter | Default | What it does |
|---|---|---|---|
| Space | `space` | 3 | gap between lines, when `lines` is above 1 |
| Dash | `dash_dark` | 1 | ink on the line at the shore (1 is solid) |
| | `dash_light` | 1 | ink on the lines further out (1 is solid) |
| | `dash_period` | 2.2 | length of one dash plus gap, times `space` |
| | `dash_cutoff` | 0.03 | below this tone a dash disappears |
| Mod | `mod_jitter` | 0.45 | random variation in dash length |
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
| clean double line | `{ lines: 2, style: { space: 4 } }` |
| dashed | `{ style: { dash_dark: 0.6 } }` |
| hand-drawn | `{ style: { mod_wobble: 25, mod_smooth: 2 } }` |
| thicker | `{ style: { batch_stroke: 2 } }` |

---

## On the command line

The `water` block is one JSON argument:

```bash
burin-map --terrain-at "44.3035,9.2145" --terrain-size 14km \
  --water '{"shoreline":{"lines":2,"style":{"space":4}},"batches":[{"follow":"shore","style":{"water_start":10,"water_reach":30,"space":7}}]}'
```

## In a share link

A `#d=…` link has three sections separated by `|`: area, land and water. For example:

```
#d=at44.3035,9.2145,ts14x10|a0|l2,s4;shore,a0,wst10,wr30,s7
```

- **Land**: `a0`, one batch at angle 0.
- **Water**:
  - before the first `;` comes the shoreline: `l2,s4` is 2 lines, 4 px apart (`noshore` turns it off, `c…` picks the classes);
  - then one batch after each `;`: `shore,a0,wst10,wr30,s7`.
  - a batch that draws ripples or dots says so after `shore` / `map`: `map,ripples,a90`.
