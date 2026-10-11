# Regions — lines that ease out at a boundary

**Status: implemented for land lines and dots.** Water batches, fills and
expressions are not covered yet (see the end).

A host program hands the engine arbitrary areas — parks, city limits, a
land-use raster — and a batch's lines ease out towards their edge the way the
open-sea pass of `presets/engraved-sea.js` breaks up near the coast: thinner
ink, wider spacing, and a ragged edge of dashes and dots instead of a clean
cut.

- **within** a region: lines live inside it and fade out towards its edge (a
  park drawn in hachures that dissolve at the park boundary);
- **outside** a region: lines fade out as they enter it (terrain hatching that
  gives way around a city).

This is for programs that use burin-map as a dependency — Rust first, then
Python and Node. Document scripts and the web app only see region *names*;
the web app never supplies any.

## Principle: the atlas names regions, the host supplies them

The atlas says *how* to draw and serialises to JSON; elevation, water
coverage and regions are data handed in at draw time:

- a `Batch` names regions in `within` / `outside` and says how to react with
  the `mask_*` style parameters;
- the region **data** comes with the draw call, never inside the atlas.

So atlases stay small, the same atlas draws against different region data,
and `burin-map-core` stays pure: pixels in, paths out, no GeoJSON, no
projection.

## `burin-map-core`: pixel space

```rust
pub enum Region {
    /// 0…1 per pixel, `width * height`, row 0 north. Soft edges carry through.
    Coverage(Vec<f32>),
    /// Pixel polygons (x right, y down): exteriors and holes.
    Rings(Vec<Ring>),
    /// Signed distance in px, positive inside, for hosts with exact geometry.
    Distance(Vec<f32>),
}

let surface = Surface::new(&elev, w, h, px_m)
    .px_per_mm(k)
    .regions(&[("parks", parks), ("cities", cities)]);

let mut atlas = Atlas::new();
atlas.batch(Batch::along(hills).within("parks"));
atlas.batch(Batch::along(hatch).outside("cities"));
let drawing = atlas.draw(&surface, water_coverage)?;
```

`Surface` is everything a drawing is drawn over; `Atlas::draw`, `draw_atlas`,
`draw_batches_auto_told` and `draw_fields_timed` take it and return
`Result<Drawing, String>`. `Ring` is `terramask::Ring`, re-exported.

`Batch` gains `within` / `outside` (`Vec<String>`, skipped in JSON when
empty), with builders `Batch::within(name)` / `Batch::outside(name)`. JSON:
`{"within": ["parks"], "outside": ["cities"]}`; a script may give one name as
a string. Fills refuse them.

### Style parameters (group `mask`)

| Parameter | Short | Default | Meaning |
|---|---|---|---|
| `mask_start` | `mst` | 0 | no lines nearer the edge than this, px (inside for `within`, outside for `outside`) |
| `mask_reach` | `mr` | 40 | distance past `mask_start` over which ink eases from 0 to full, px |
| `mask_fade` | `mfd` | 1 | how it eases: above 1 later, below 1 sooner |
| `mask_fray` | `mfr` | 0 | band past `mask_start` where lines break into dashes and dots, px; 0 a clean edge |
| `mask_start_m`, `mask_reach_m`, `mask_fray_m` | `mstm`, `mrm`, `mfrm` | 0 | the same distances in metres; above 0 they win, as `px = m / px_m` |

Pixels suit a fixed look on the page; metres suit geography (a 200 m park
fade stays 200 m at any zoom).

### Several regions, missing regions

Regions listed together are merged before the distance is taken: coverage by
the per-pixel maximum (so touching parks leave no seam), `Distance` regions by
the maximum distance. A batch with both lists keeps the **minimum** of the two
weights: inside the parks *and* away from the cities, each edge fading on its
own.

A batch naming a region the draw call didn't supply, a region given twice, or
a region of the wrong size is an error naming the batch and region.

### How it draws (`regions.rs`, `fields::keep_to`, `fray.rs`)

1. Per draw, each distinct set of names a batch uses is resolved once into a
   signed distance in px (`terramask::Mask::from_rings` for rings,
   `Mask::distance()` for coverage).
2. With `edge = min(d_within, -d_outside)` (a missing list counts as far
   inside), the weight is

   ```text
   k = 0                                          if edge < start
     = clamp((edge - start) / reach, 0, 1) ^ fade
   ```

3. In `trace_land`, after `heighten`, `fields::keep_to` multiplies the input
   tone by `k`, rescales spacing as `heighten` does, and makes `k = 0`
   unwalkable. Scaling the shading (rather than replacing it) keeps dark
   slopes inked further into the fade than flat ground.
4. For lines, when `mask_fray > 0`, the traced paths go through the shared
   `fray_along` with
   `whole = (edge - start) / fray + (noise - 0.5) * 0.8` (salt `0x5EA1`, apart
   from water's `0xF4A7`).

This runs per batch on the CPU after fields are built, so terrain-group
caching and the GPU field path are untouched. No region named → identical
output.

## `burin-map`: geographic

```rust
pub enum GeoRegion {
    /// Polygons in lon/lat degrees: polygons → rings → points; the first ring
    /// of each polygon is its exterior, the rest holes.
    Polygons(Vec<Vec<Vec<[f64; 2]>>>),
    /// Already on the DEM grid the document will be drawn on.
    Pixels(Region),
}

let regions = vec![("parks".to_string(), GeoRegion::from_geojson(&geojson)?)];
burin_map::draw_document(&doc, &regions, progress)?;
```

Polygons are projected with the DEM's own projector (`dem::projector`).
`GeoRegion::from_geojson` reads a Polygon, MultiPolygon, Feature,
FeatureCollection or GeometryCollection; `regions_from_json` reads an object
of them by name. Missing names fail before any download.

CLI: `--region NAME=PATH` (repeatable), PATH a GeoJSON file.

## Python and Node

```python
burin.batch(within="park", mask_reach_m=150, mask_fray_m=60)
burin.draw(regions={"park": park_geojson})       # or a shapely geometry
burin.draw_script(source, regions={...})
```

```js
burin.batch({ within: 'park', mask_reach_m: 150 })
burin.draw({ regions: { park: parkGeoJSON } })
burin.drawScript(source, { regions })
```

Python takes GeoJSON dicts or anything with `__geo_interface__` (Shapely,
GeoPandas). Both send `{"atlas", "area", "regions"}` to the native
`draw_document`.

## Not done yet

- **Water batches:** `within` / `outside` on `WaterBatch` — multiply the weight
  into `water_fields`' input, one more `fray_along` in `trace_water`.
- **Fills:** intersect rings with the region, no fray.
- **Expressions:** the region distance as a field for colour and width trees.
- **Pixel regions from Python / Node:** a numpy array or `Float32Array` as
  `Coverage`, and regions for `draw_elevation`.
- **`geo-types` feature:** `From<geo_types::MultiPolygon<f64>>` for
  `GeoRegion`.
