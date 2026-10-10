# CLI and orchestration (`burin-map`)

Wraps `burin-map-core` with geocoding, DEM download/reprojection, water tiles, document parsing, and raster/SVG export.

## Crate layout

| Module | Role |
|--------|------|
| `cli.rs` | `burin-map` binary: flags, `--script`, terrain/area flags, output formats |
| `dem.rs` | Load/build elevation grid (`Dem`, `load_dem`, bounds) |
| `geo.rs` | `locate`, `bounds_around`, scale/length parsing |
| `document.rs` | `Document`, `AreaDoc`, `LandDoc`, `WaterDoc` — JSON5 atlas + area |
| `raster.rs` | PNG (and related) rendering from `Drawing` |
| `water.rs` | OSM water tiles, coverage for atlas water |
| `tmerc.rs` | Projection helpers |
| `progress.rs` | Progress wiring to core stages |

Public types re-export core drawing types (`Style`, `Batch`, `Drawing`, …) from `lib.rs`.

## End-to-end draw (library)

Typical path: resolve `AreaDoc` → fetch/project DEM → build `Frame` (pixel size or paper mm + pen width) → evaluate batches (from script or document) → `Atlas` or `draw_batches` → `Map` with SVG/PNG helpers (`lib.rs`, `Map`).

Errors: `Error::Input` | `Net` | `Data`.

## CLI document sources

Priority (see `cli.rs` and README): `--script` runs `burin_map_script::evaluate_document`; `--style-file` / `--style-json` load `Document`; flags override document area/frame fields.

## Dependencies

- `burin-map-core`, `burin-map-script`
- `terramask` (workspace path): tile fetch when enabled on this crate
- Nominatim for place lookup (network)

## Frame model

`Frame`: optional paper `size_mm` + `pen_width` sets scale; otherwise default pixel width (`px`, default 2400). Ground vs paper aspect ratio: drawing shrinks to fit (documented on `Frame` in `lib.rs`).

## Related

- Engine internals: [core-engine.md](core-engine.md)
- Script → atlas: [document-and-atlas.md](document-and-atlas.md)
