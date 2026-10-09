# Land edge fray (proposed)

Optional fraying and fade of **land** batches toward the edge of a drawable
region — the same visual idea as `water_fray` / `water_fade` / `water_reach`
near the **shore**, but measured from the **boundary of a host-supplied mask**
instead of from the sea.

**Status:** design only — not implemented in `burin-map-core` yet.

**Motivation:** [draw.land](https://draw.land) draws burin terrain only on
“unclaimed” ground (`burin.terrain({ in: area })`). Today the host clips ink to
that polygon **after** burin runs, so contours stop on a **hard** line. City maps
need a soft transition into urban white (residential pad, roads, buildings)
without hand-drawn geometry fringes in every preset.

---

## Problem today

### Water (works)

1. OpenMapTiles / coverage defines where the sea is.
2. `water::shore` builds a per-pixel **distance from the coast** grid `dist`.
3. Water batches use `dist` for tone, spacing, and optional
   [`water::fray`](../crates/burin-map-core/src/water.rs) so lines break into
   shorter dashes and dots as they approach the shore (`water_fray`, `water_start`,
   `water_fade`, `water_reach`).

### Land (gap)

1. Land batches trace over the full elevation grid (`trace_land` in
   [`lib.rs`](../crates/burin-map-core/src/lib.rs)).
2. `expr::ColorCtx` sets `dist: None` for land — no distance field.
3. [`water::fray`](../crates/burin-map-core/src/water.rs) is only called from
   `trace_water`, never from `trace_land`.
4. Hosts (draw.land) clip the resulting lines to `in_area` with a geometric
   intersection — **no** fray at that boundary.

So the “beige” category-0 terrain in city styles ends abruptly at the urban
mask unless the preset fakes a fringe with extra polygons.

---

## Goal

When a host passes a **land mask** (polygon in pixel space, same size as the
elevation grid):

- Land batches may **fade** ink and spacing toward the mask edge (optional).
- Land batches may **fray** strokes into shorter dashes and dots near the edge
  (optional), reusing the existing `fray` / `fray_along` machinery.
- Syntax parallels the water shore parameters so documents and presets stay
  familiar.
- With no mask, or with all edge parameters at 0, behaviour is **unchanged**
  from today.

Standalone burin-map (CLI, full-frame maps) is unaffected unless a future
feature passes a mask.

---

## Proposed style parameters (land)

Mirror the water shore group with an **`edge_`** prefix on land batches (clearer
than overloading `water_*` on land):

| Parameter | Default | Role |
|-----------|---------|------|
| `edge_start` | `0` | Inset from the mask boundary, in **px**, before fray/fade begin (like `water_start` from the shore). |
| `edge_reach` | `60` | Distance in **px** over which ink intensity eases off (like `water_reach`). |
| `edge_fade` | `1` | Exponent on the fade curve (like `water_fade`). |
| `edge_fray` | `0` | Width in **px** of the band where full strokes break up toward the boundary (like `water_fray`). `0` = hard edge, current behaviour. |
| `edge_space_reach` | `0` | Optional: distance in **px** over which line **spacing** eases (`space_dark` / `space_light` toward the edge), like `water_space_reach`. `0` = fixed `space`. |

Short aliases in the style table (same pattern as water): e.g. `est`, `er`,
`ef`, `efr`, `esr`.

**Semantics:** At each pixel **inside** the mask, `edge_dist` is the Euclidean
distance to the nearest point **outside** the mask (distance to the boundary,
measured inward). Near the boundary `edge_dist` is small:

- **Fray:** reuse `fade_from_shore(edge_dist, edge_start, edge_fray, edge_fade)`
  as the `whole` factor in `fray_along` (same as water, but inverted role of
  “shore” — we fray as `edge_dist` **decreases**).
- **Tone / spacing:** same mapping water uses from `dist`, but driven by
  `edge_dist` when `edge_reach` / `edge_fray` > 0.

Exact parity with water should be verified in one golden test (see below).

---

## Distance grid

Add a small module (or functions in `water.rs` / new `edge.rs`):

```text
land_edge_distance(mask: &[f32], w, h) -> Vec<f32>
```

- Input: `mask[i] > 0` = drawable land (same convention as water `coverage`).
- Output: per pixel, distance in **pixels** to the nearest non-drawable pixel
  (0 outside the mask).
- Algorithm: chamfer / two-pass distance transform, or reuse whatever
  `water::shore` uses for `dist` with inverted semantics — prefer **one**
  implementation shared where possible.

Cost: one `w×h` float buffer per draw when any land batch has
`edge_fray > 0` or `edge_reach` used for tone. Same order as water shore
distance.

---

## Engine changes (`burin-map-core`)

### 1. API entry

Extend [`Atlas::draw_with`](../crates/burin-map-core/src/atlas.rs) (or
`draw_atlas`) with an optional **`land_mask: Option<&[f32]>`** (or
`land_coverage`), same length as `elev`.

- CLI / full-frame: `None` → no edge grid, no behaviour change.
- draw.land: rasterize `in_area` to the terrain grid and pass it in.

### 2. `trace_land`

When `land_mask` is present and the batch style has active edge parameters:

1. Build `edge_dist` once per draw (shared across land batches).
2. After `trace_batch` / land dots, run `water::fray(paths, &edge_dist, …)`
   **or** a thin wrapper `edge::fray_inward(...)` if the `whole(p)` sign must
   differ from water.
3. Optionally modulate `batch_fields` input / spacing from `edge_dist` before
   trace (mirror water’s `tone` / `spacing_tone`).

Land **fills** (`draw: fill`) are out of scope for v1 unless we only apply
`input_blur` at the mask edge later.

### 3. Expressions

Today `dist` in `color_expr` / `width_expr` is **water-only**
([SYNTAX.md](../SYNTAX.md)). Proposal:

- On land batches with a mask, expose **`edge`** (or reuse `dist` with
  documentation that on land it means edge distance when a mask is set).

Document in SYNTAX when implemented.

### 4. Clipping

Hosts may still clip to the mask for correctness. Fray should happen **before**
clip so dashes can die out **inside** the polygon; the clip then removes any
overflow. draw.land should pass the mask into burin **and** keep clip (or rely
on mask alone if fray never produces ink outside).

---

## Host integration (draw.land)

Today (`drawland/crates/drawland-core/src/terrain.rs` in the draw.land repo):

```rust
draw_burin(grid, spec) → lines → clip_lines(drawn, in_area)
```

Proposed:

1. Rasterize `in_area` to the terrain grid (`width×height`, same as elevation).
2. Pass raster to `draw_atlas(..., land_mask: Some(&raster))`.
3. Keep `clip_lines` as a safety net, or skip when mask + edge fray are active.

Preset example (city grain):

```js
const groundBurin = burin.terrain(
  (b) => {
    b.batch({
      angle: 0,
      style: { edge_fray: 24, edge_start: 2, edge_fade: 0.8 },
    })
  },
  { in: burinIn },
)
```

`in: burinIn` drives both clip and mask raster until burin accepts mask
natively.

---

## Non-goals (v1)

- Fray toward **holes** inside the mask (e.g. a lake hole in ground): optional
  later; v1 can treat hole boundaries like exterior edges (fray on all sides of
  drawable region).
- Automatic mask from OSM — host’s job.
- New stroke types; only parameters + mask plumbing.
- Changing water batch behaviour or renaming existing `water_*` keys.

---

## Tests

1. **Unit:** small `w×h` mask (rectangle with fray on); assert path count or
   segment length distribution increases near edge vs `edge_fray: 0`.
2. **Regression:** default atlas / `presets/contours.js` bitwise or
   perceptually unchanged with `--force-cpu`.
3. **Integration:** draw.land `check_web_presets` for `city-grain` with fray
   enabled (visual review).

---

## Documentation updates (when implemented)

- [SYNTAX.md](../SYNTAX.md): new “Land edge” subsection under land parameters;
  table row for `edge_*`; note on `edge` in colour expressions.
- [README.md](../README.md): one line under library API for `land_mask`.
- This file: move status to **implemented** and link to the commit.

---

## Open questions

1. **Naming:** `edge_*` vs reusing `water_*` on land when `land_mask` is set
   (shorter presets, confusing in YAML).
2. **Mask convention:** float 0…1 vs binary; must match water `coverage`.
3. **Performance on WASM:** one extra distance pass per terrain draw — acceptable
   for draw.land’s single burin layer per map?

---

## References in tree

| Piece | Location |
|-------|----------|
| Water fray | `crates/burin-map-core/src/water.rs` — `fray`, `fade_from_shore` |
| Land trace | `crates/burin-map-core/src/lib.rs` — `trace_land` |
| Style keys | `crates/burin-map-core/src/style.rs` |
| draw.land clip | drawland: `crates/drawland-core/src/grammar_run.rs` — `Op::Burin` |
| Use case | drawland: `web/src/presets/city-grain.js` — `burinIn`, `urbanPad` |
