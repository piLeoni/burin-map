# Core draw engine (`burin-map-core`)

Pure computation: square-pixel elevation grid in metres → polylines (dashes), pen runs, and fills. No network or filesystem.

## Entry points

| API | File | Role |
|-----|------|------|
| `draw` / `draw_batches` | `src/lib.rs` | Single or multi-batch land draw |
| `draw_atlas` | `src/lib.rs` | Land + water from `Atlas` over a `Surface` |
| `Atlas` | `src/atlas.rs` | Accumulate batches; `draw(&Surface, coverage)` |
| `Surface` | `src/lib.rs` | What a drawing is drawn over: elevation, size, `px_m`, `px_per_mm`, named regions |
| `Region`, masks | `src/regions.rs` | Host areas by name → signed distance → per-batch weight and fray (`within` / `outside`, `mask_*`); see [../regions.md](../regions.md) |
| `fray_along` | `src/fray.rs` | Break paths into dashes and dots along a noisy edge; shared by water and regions |
| `Style`, `Batch` | `src/style.rs` | Parameters and batch passes (angle, ground, caps). `Style::default()` is the hand-drawn land-lines look; `Style::plain()` (modifiers off) is the base for water, shore and dot defaults |
| `batch_fields`, `build_fields` | `src/fields.rs` | Direction, input, spacing fields |
| `trace_streamlines`, `trace_stripes` | `src/streamlines.rs` | Contour-parallel lines; striped parallel trace |
| `dash_lines`, `dash_lines_by` | `src/dashes.rs` | Input-driven dashing; `dash_cutoff` thins by `Style::passes_cutoff` (also used by land dots) |
| `plot_order`, `pen_up_travel` | `src/order.rs` | Plotting order |
| Water / shoreline | `src/water.rs` | Water batches, shoreline passes |
| Depth / bathymetry grid | `src/depth.rs` | Depth-weighted expressions on water |
| `expr` | `src/expr.rs` | Colour/width trees (`lerp`, fields, scalars) |
| `script` | `src/script.rs` | Deserialize atlas JSON → `Batch`, `WaterBatch`, `Shoreline` |
| `geojson` | `src/geojson.rs` | GeoJSON helpers for outputs |

Module-level pipeline summary: `src/lib.rs` crate docs (fields → streamlines → order → dashes; batches in parallel).

## Data flow (land)

1. Build terrain **fields** once per distinct terrain style (direction, input, spacing).
2. For each `Batch`: rotate direction by `angle`; if it names regions, `fields::keep_to` scales its input and spacing by the region weight (CPU, after the shared fields, so caching and the GPU path are unaffected); trace streamlines (striped parallel tracer; `STRIPE` = 256 px in `lib.rs`), then fray lines near the region edge when `mask_fray` > 0.
3. **Order** lines, convert to **dashes** from input/lighting rules (`dashes.rs`; land spacing logic ties to `fields.rs`).
4. Merge batch results into `Drawing` (`paths`, `lines`, `runs`, `fills`).

Sea coverage can come from elevation NaNs/sea level or an external coverage grid (the `coverage` argument of `Atlas::draw` / `draw_atlas`).

Regions are resolved once per draw (`Masks::resolve`), only when some batch names one, keyed by the set of names; a missing, duplicate or wrong-sized region is an `Err`, so the draw entry points taking a `Surface` return `Result`.

## GPU / CPU (native)

- Feature `gpu`: `src/engine.rs` (not `wasm32`) picks GPU when present; `force_cpu()` / `BURIN_ENGINE=cpu` before first draw.
- Implementation: `src/gpu/` (wgpu). CPU path uses f64 in blurs; GPU uses f32 — reproducibility implication documented in `decisions.md`.
- Tests: `tests/force_cpu_order.rs` (ordering guard for late `force_cpu`).

## Water and depth

`WaterBatch` draws lines, ripples, or dots on water; `Shoreline` for coast. Depth-related weights use optional bathymetry grid (`depth.rs`, `needs_depth`, `build_depth_grid`). CLI/web fetch bathymetry outside the core.

## Progress

`src/progress.rs`: staged callbacks (`Stage`, `Step`) used by CLI and WASM for UI progress bars.

## Invariants / constraints

- Elevation slice length must equal `width * height`.
- `draw_batches` requires non-empty `batches`.
- Region data is a draw input (`Surface::regions`), never part of the atlas; the core stays in pixels (no projection, no GeoJSON). No region named → output identical to a draw without regions.
- Pixel coords: (0,0) NW, y south; `GeoTransform` maps to lon/lat.
- Ink/style parameter names and groups: `Style::PARAMS`, `GROUPS` in `style.rs` (also exposed to WASM for the web sidebar).

## Related

- Orchestration and rasters: [cli-orchestration.md](cli-orchestration.md)
- Atlas JSON shape: [document-and-atlas.md](document-and-atlas.md)
