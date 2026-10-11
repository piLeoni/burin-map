# Regions — lines that ease out at a boundary (proposal)

**Status: proposed, not implemented.**

Let a host program hand the engine arbitrary areas — parks, city limits, a
land-use raster — and have a batch's lines ease out towards their edge the way
the open-sea pass of `presets/engraved-sea.js` breaks up near the coast:
thinner ink, wider spacing, and a ragged edge of dashes and dots instead of a
clean cut.

Two kinds of use:

- **within** a region: lines live inside it and fade out towards its edge (a
  park drawn in hachures that dissolve at the park boundary);
- **outside** a region: lines fade out as they enter it (terrain hatching that
  gives way around a city).

This is for programs that use burin-map as a dependency — other Rust crates
first, then Python and Node. The web app and document scripts are not the
target; they only see region *names*.

## What exists today

The behaviour already exists for water, against one fixed boundary (the coast):

| Piece | Where | What it does |
|---|---|---|
| Distance to the boundary | `water::shore` → `terramask::Mask::distance()` | exact signed distance in px, + inside |
| Ink easing with distance | `water::fade_from_shore`, `water_fields` | tone from `water_start` / `water_reach` / `water_fade` |
| Ragged edge | `water::fray` → `fray_along(paths, …, whole)` | breaks paths into shorter dashes, then dots, along a noisy edge (`water_fray`) |
| Same, for depth limits | `water::fray_by_depth` | reuses `fray_along` with a different `whole` |

Land batches have none of it: `trace_land` never frays, and the only soft
boundary is the coast term (`coast[i]`) in `fields::build_fields_with`. A land
batch's `where` is rejected unless it is a fill (`style.rs`).

`fray_along` already takes any `whole(p) -> f32` (0 gone … 1 whole), so a
region is a third caller of the same code, not new machinery.

## Principle: the atlas names regions, the host supplies them

The atlas says *how* to draw and serialises to JSON; the elevation and the
water coverage are data handed in at draw time (`Atlas::draw_with`). Regions
follow the same split:

- a `Batch` (and later a `WaterBatch`) refers to regions **by name** and says
  how to react (`within` / `outside`, plus `mask_*` style parameters);
- the region **data** comes with the draw call, never inside the atlas.

So atlases stay small and serialisable, the same atlas can be drawn against
different region data, and `burin-map-core` stays pure: pixels in, paths out,
no GeoJSON, no projection, no network.

## Design in three layers

### Layer 1 — `burin-map-core`, pixel space (the real feature)

```rust
/// An area on the elevation grid's pixels, as the host has it.
pub enum Region {
    /// 0…1 per pixel, `width * height`, row 0 north. Soft edges carry through.
    Coverage(Vec<f32>),
    /// Pixel polygons (x right, y down): exteriors and holes, as `terramask::Ring`.
    Rings(Vec<terramask::Ring>),
    /// Signed distance in px, positive inside: for hosts with their own
    /// exact geometry. Skips the distance transform.
    Distance(Vec<f32>),
}

/// Everything a drawing is drawn over.
pub struct Ground<'a> {
    pub elev: &'a [f32],
    pub width: usize,
    pub height: usize,
    pub px_m: f32,
    pub water: Option<&'a [f32]>,
    pub regions: &'a [(&'a str, Region)],
    pub px_per_mm: f32,
}

let mut atlas = Atlas::new();
atlas.batch(Batch::along(hills).within("parks"));
atlas.batch(Batch::along(hatch).outside("cities"));
let drawing = atlas.draw(&Ground { /* … */ regions: &[("parks", parks), ("cities", cities)], ..ground });
```

`Ground` replaces the positional lists outright: `Atlas::draw(&Ground)` is
the one way in, and `Atlas::draw_with` and `draw_atlas`'s
`(elev, coverage, width, height, px_m, px_per_mm)` signature go away. Every
caller — the CLI (`burin-map`), the Python, Node and wasm bindings, the
tests — moves to `Ground` in the same change; no wrappers are kept.

On `Batch` (serialised, skipped when empty):

```rust
/// Regions this batch keeps to, by name; their weights multiply.
pub within: Vec<String>,
/// Regions this batch gives way to, by name; their weights multiply.
pub outside: Vec<String>,
```

with builder methods `Batch::within(name)` / `Batch::outside(name)`. JSON:
`{"within": ["parks"], "outside": ["cities"]}`.

New style parameters, group `mask`, mirroring `water_*` so users already know
them (defaults leave everything as today):

| Parameter | Default | Meaning |
|---|---|---|
| `mask_start` | 0 | no lines nearer the edge than this, px (inside for `within`, outside for `outside`) |
| `mask_reach` | 40 | distance past `mask_start` over which ink eases from 0 to full, px |
| `mask_fade` | 1 | how it eases: above 1 later, below 1 sooner |
| `mask_fray` | 0 | band past `mask_start` where lines break into dashes and dots with a ragged edge, px; 0 a clean edge |
| `mask_start_m`, `mask_reach_m`, `mask_fray_m` | 0 | the same distances in metres; when above 0 they win over the px value |

Metres and pixels are the same setting through `px_m`: the engine resolves
each distance once per draw as `px = m / px_m` when the `_m` value is set, and
uses the px value otherwise. Pixels suit a fixed look on the page (a 40 px
fade is the same on every map); metres suit geography (a 200 m park fade stays
200 m at any zoom). `mask_fade` has no unit.

### Several regions on one batch

Regions listed together are merged into one area (their union) before the
distance is taken, so where two parks touch or overlap there is no seam
between them:

- `within: ["park_a", "park_b"]` — lines live in either park, fading at the
  outer edge of the two together;
- `outside: ["city_a", "city_b"]` — lines give way to both cities.

In weights this is the maximum of the `within` regions' weights and the
minimum of the `outside` ones. A batch with both keeps the **minimum** of the
two results: inside the parks *and* away from the cities, each edge fading on
its own. A pixel near either edge is as light as the stronger fade asks; fades
don't stack darker or lighter where they meet.

### Missing regions

A batch can name a region the host didn't supply (`within: ["parks"]` with no
`"parks"` in the draw call), for example a script written for a host program
run somewhere else. That is an error at draw time naming the batch and the
region, not a silent full-strength or empty batch.

### Layer 2 — `burin-map`, geographic (optional convenience)

This crate owns the projection (`dem::projector(bounds, w, h)`, transverse
Mercator) and fetches the elevation, so lon/lat polygons belong here:

```rust
pub enum GeoRegion {
    /// Polygons in lon/lat degrees: polygons → rings → points.
    Polygons(Vec<Vec<Vec<[f64; 2]>>>),
    /// Already on the DEM grid the document will be drawn on.
    Pixels(Region),
}

burin_map::draw_document(&doc, &[("parks", GeoRegion::Polygons(parks))], progress)?;
```

Regions become a parameter of `draw_document` itself (an empty slice when
there are none), not a second `_with` entry point.

It projects every ring with the same projector as the DEM, then passes
`Region::Rings` down. Optional cargo features, off by default:

- `geo-types`: `From<geo_types::MultiPolygon<f64>>` for `GeoRegion`, which
  connects the `geo`, `geojson`, `shapefile` and `gdal` crates without
  burin-map parsing their formats;
- `geojson`: parse a GeoJSON `Polygon` / `MultiPolygon` / `FeatureCollection`
  string, for callers that only have that.

Hosts that bring their own elevation already know their pixel grid and use
Layer 1 directly.

### Layer 3 — Python and Node

- **Python:** `burin.draw(regions={...})` and `draw_elevation(..., regions={...})`.
  A 2-D numpy array is `Coverage`; a GeoJSON dict or anything with
  `__geo_interface__` (shapely, geopandas) is lon/lat polygons for Layer 2
  (`draw`), or pixel rings for `draw_elevation`. `batch(within=..., outside=...)`.
- **Node:** a `Float32Array` (with the grid size) is coverage; a GeoJSON object
  is polygons. Same `within` / `outside` on `batch`.
- **Scripts / web app:** nothing new beyond the two name fields in the schema.
  A script that names a region nobody supplies fails at draw time (see open
  questions); the web app never supplies any.

## Engine work, step by step

1. **Shared fray.** Move `fray_along` and `noise_field` out of `water.rs` into
   a small module (e.g. `src/fray.rs`), `pub(crate)`. `water::fray` and
   `fray_by_depth` call it unchanged. No output change; existing water tests
   must still pass byte-for-byte.
2. **Region → distance.** `src/regions.rs`:
   - `Rings` → coverage with `terramask::Mask::from_rings`.
   - `Coverage` → signed Euclidean distance transform.
   - `Distance` → used as given.
   - Merge each batch's `within` list and its `outside` list into one coverage
     each (per pixel maximum) before the distance, as described under
     "Several regions on one batch".
   - Resolve once per draw, keyed by the set of names; only what some batch
     names is computed. Check sizes against `width * height` and missing
     names, and return an error instead of panicking.

   Both come from terramask, which core already depends on with
   `default-features = false` (no fetching, no DEM decoding). They are
   generic geometry, nothing about tiles or OpenStreetMap, so region data
   doesn't have to come from terramask to go through them:
   - `terramask::Mask::from_rings(width, height, &[Ring], supersample)` fills
     pixel rings (`Ring { exterior, points }`) the way `Features::mask` fills
     areas: exteriors union, holes cut, partial coverage at the edges
     (nonzero winding after orienting each ring by its `exterior` flag);
   - `terramask::Mask { width, height, coverage }.distance()` is the exact
     signed distance, + inside, for any coverage.

   `Mask::from_rings` is on terramask `main`, part of the unreleased 0.2.0
   that burin-map already requires through its path dependency.
3. **Weight.** With `d` the signed distance to the merged `within` area, and
   `e` the signed distance to the merged `outside` area negated (so it is
   positive away from it), and the distances already converted to px:

   ```text
   k(d) = 0                                          if d < mask_start
        = clamp((d - mask_start) / mask_reach, 0, 1) ^ mask_fade

   weight = min(k(d), k(e))     // either term is 1 when its list is empty
   ```

   Note the direction: water's `fade_from_shore` goes *down* with distance;
   this goes *up* (full ink deep inside the kept area).
4. **Apply to fields.** In `trace_land`, right after `batch_fields(...)` and
   next to `fields::heighten`, call `fields::mask(&mut bf, &weight, &st)`:
   - `input *= k`, so the existing ink rules (`dash_light`, `dash_cutoff`,
     `mod_flat`) thin the lines on their own. It scales the shading instead of
     replacing it, so dark slopes hold their ink further into the fade than
     flat ground — the "aware of what's around" quality;
   - `space` follows the new input as `heighten` does
     (`s * space_mul(t1) / space_mul(t)`, clamped);
   - `walkable &= k > 0`, so the tracer skips what can't be drawn.

   This runs per batch on the CPU after the fields are built, so terrain-group
   caching and the GPU path (`gpu.rs`, `fields.wgsl`) are untouched.
5. **Fray the paths.** After `trace_batch` (and `land_dots` for dots), when
   `mask_fray > 0`, run the shared `fray_along` with
   `whole(p) = (min(d(p), e(p)) - mask_start) / mask_fray + (noise(p) - 0.5) * 0.8`,
   with `d` and `e` as in step 3, so the lines fray at whichever edge is
   nearer. Seed from `batch_seed` with its
   own salt so it doesn't correlate with water fray.
6. **Fills (optional, later).** A fill with `within` / `outside` intersects its
   rings with the region (or masks its raster before tracing rings in
   `fills::rings`). No fray; `input_blur` already softens fill edges.
7. **Water batches (optional, cheap).** Same `within` / `outside` on
   `WaterBatch`: multiply the weight into `water_fields`' input and add one
   more `fray_along` call in `trace_water`.
8. **Expressions (optional).** Expose the region distance to colour and width
   trees (like `dist` on water) so stroke width can taper towards an edge.

## Invariants to keep

- No regions named → identical output to today, on CPU and GPU. Add a test
  that draws the presets' atlases with an empty region list and compares.
- `burin-map-core` gains no geographic code and no new heavy dependencies.
- Region data never enters the atlas JSON; only names do.
- Deterministic for a given `batch_seed`.

## Tests

- Unit: `k(d)` at the edge, at `mask_start`, past `mask_reach`; `within` and
  `outside` give complementary weights for the same region.
- Unit: `mask_reach_m: 200` at `px_m: 10` behaves as `mask_reach: 20`.
- Unit: two overlapping `within` regions give no seam where they meet; a
  `within` and an `outside` region together take the minimum.
- Unit: rasterising a square ring gives the expected coverage and distance
  sign; holes are holes.
- Draw: a hand-made `Coverage` (left half 1, right half 0) with `within` —
  total ink on the right half is ~0, ink near the seam is between, and with
  `mask_fray` there are more, shorter paths near the seam than without.
- Draw: `outside` on the same mask mirrors it.
- Errors: missing region, wrong grid size.
- Bindings: a Python test with a numpy mask and one with a GeoJSON dict; a
  Node test with a `Float32Array`.

## Docs to update when it lands

- `docs/architecture/core-engine.md`: regions as a draw input; the `mask`
  step in the land data flow.
- `docs/architecture/document-and-atlas.md`: `within` / `outside` on batches;
  region data is not part of the atlas.
- `docs/architecture/bindings.md`: `regions=` in Python and Node.
- `docs/architecture/decisions.md`: why region data is passed at draw time and
  why core stays in pixel space.
- `SYNTAX.md` and `docs/guide/land-lines.md`: the `mask_*` parameters.
- `crates/burin-map-core/burin.d.ts`, `web/src/lib/burin.d.ts`,
  `node/index.d.ts`, `python/burin_map/__init__.py`: the two batch fields.
- Version bump through `workspace.package.version` (Python/Node kept in sync).

## Decided

- **Units:** both. Every mask distance has a px parameter and an `_m` twin;
  the `_m` one wins when set, converted with `px_m`.
- **Missing region:** error at draw time.
- **Several regions:** merged (union) within each list; `within` and
  `outside` combine by the minimum.
- **Geometry helpers:** exported from terramask, which stays a dependency;
  region data itself never has to come from terramask.
