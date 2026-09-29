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
| `name` | a label, optional |
| `angle` | 0 follows the contour lines, 90 runs straight downhill |
| `style` | parameters, see below; those left out keep their defaults, except the terrain ones (see [Terrain carries over](#terrain-carries-over)) |

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
| `draw` | `lines` fill the water; `arcs`, `dashes` or `waves` scatter wave marks on the open sea, see [Wave marks](#wave-marks) | `lines` |
| `angle` | with `shore`, 0 runs along the coast and 90 straight out to sea; with `map`, degrees clockwise from north | 0 |
| `style` | parameters, see below | |

### Water parameters

The land's names, minus those that need terrain. The defaults differ, and there are three more.

There is no seabed: nothing about depth is downloaded. On water, the "tone" that land takes from terrain and sun is the distance from the shore instead: 1 at the first line (`water_start` out), fading to 0 `water_reach` px further out. That tone drives `space_light` / `space_dark` and `dash_light` / `dash_dark` exactly as shading does on land. Water depth could be an optional input one day.

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

Left out, as they mean nothing on water: `space_steep`, `space_steep_gamma`, `batch_floor`, the `sun_*` group and the rest of the `input_*` group.

### Wave marks

With `draw: arcs`, `dashes` or `waves`, a batch scatters small groups of marks over the open sea, gathered in patches, instead of filling it with lines:

| `draw` | Each mark |
|---|---|
| `arcs` | two low, staggered arcs, like the waves of old charts |
| `dashes` | four short dashes in staggered rows, as engravers drew the sea |
| `waves` | two short wavy lines, one over the other |

```yaml
water:
  batches:
    - follow: map          # all at `angle` from north; shore: along the nearest coast
      angle: 90            # 90: level, like waves on a chart
      draw: waves
      style:
        space: 70          # about this far apart
        water_size: 26     # each this long
        water_patches: 0.6 # gathered in patches: 0 evenly everywhere, 1 a few patches
```

Marks never come closer than `space` to each other, nor nearer the shore than `water_start`; `water_patches` leaves the sea between patches empty. `follow: map` with `angle: 90` keeps them level, as on a chart; `follow: shore` turns them with the coast. Several batches can be combined: lines near the coast and marks further out, or two kinds of mark.

| Group | Parameter | Default | What it does |
|---|---|---|---|
| Space | `space` | 70 | distance between marks, px |
| | `space_dark` | 1 | spacing multiplier at `water_start` |
| | `space_light` | 1 | spacing multiplier from `water_reach` further out; with `space_dark` below it, marks crowd towards the coast |
| Mod | `mod_jitter` | 0.45 | how much size and turn vary from mark to mark |
| | `mod_wobble` | 0 | hand-drawn wander, degrees |
| | `mod_wobble_size` | 12 | length of each wander, px |
| | `mod_smooth` | 8 | how smoothly the marks turn with the coast (`follow: shore`) |
| Batch | `batch_stroke` | 1.2 | line width |
| | `batch_seed` | 1 | random seed: another scattering |
| Water | `water_start` | 20 | no marks nearer the shore than this, px |
| | `water_reach` | 60 | distance over which the spacing eases from `space_dark` to `space_light`, px |
| | `water_fade` | 1 | how it eases |
| | `water_size` | 26 | length of each mark, px |
| | `water_patches` | 0.6 | how much marks gather in patches: 0 evenly everywhere, 1 a few patches |

---|---|---|---|
| Space | `space` | 4 | |
| | `space_light` | 1.8 | |
| | `space_dark` | 0.8 | |
| | `space_steep` | 0.4 | |
| | `space_steep_gamma` | 1 | |
| Dash | `dash_light` | 0.25 | |
| | `dash_dark` | 1 | |
| | `dash_period` | 2.2 | |
| | `dash_cutoff` | 0.03 | |
| | `dash_drop` | 0.6 | |
| Mod | `mod_jitter` | 0.45 | |
| | `mod_breaks` | 0.15 | |
| | `mod_wobble` | 0 | |
| | `mod_wobble_size` | 12 | |
| | `mod_smooth` | 3 | |
| Engine | `engine_step` | 1 | |
| | `engine_min` | 4 | |
| | `engine_stop` | 0.5 | |
| | `engine_fill` | 0.8 | |
| Batch | `batch_floor` | 0 | |
| | `batch_stroke` | 1.2 | |
| | `batch_seed` | 1 | |
| Water | `water_start` | 2 | gap between the shore and the first line, px |
| | `water_reach` | 60 | how far out the lines go, px; lower means fewer lines |
| | `water_fade` | 1 | how they fade out: above 1 sooner, below 1 later |

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
  - a batch that draws wave marks says so after `shore` / `map`: `map,arcs,a70,s50`.
