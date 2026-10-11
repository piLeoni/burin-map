# CLI and orchestration (`burin-map`)

Wraps `burin-map-core` with geocoding, DEM download/reprojection, water tiles, document parsing, and raster/SVG export.

## Crate layout

| Module | Role |
|--------|------|
| `cli.rs` | `burin-map` binary: flags, `--script`, terrain/area flags, output formats |
| `dem.rs` | Load/build elevation grid (`Dem`, `load_dem`, bounds) |
| `geo.rs` | `locate`, `bounds_around`, scale/length parsing |
| `document.rs` | `Document`, `AreaDoc`, `LandDoc`, `WaterDoc` — JSON5 atlas + area; `Document::from_script` / `from_atlas`; `AreaDoc::resolve` (bounds, frame, label) and the named `PLACES` |
| `raster.rs` | PNG (and related) rendering from `Drawing` |
| `water.rs` | OSM water tiles, coverage for atlas water |
| `tmerc.rs` | Projection helpers |
| `progress.rs` | Progress wiring to core stages |

Public types re-export core drawing types (`Style`, `Batch`, `Drawing`, …) from `lib.rs`.

## End-to-end draw (library)

Typical path: `AreaDoc::resolve` (bounds + `Frame`: pixel size or paper mm + pen width) → fetch/project DEM → batches and water (from script or document) → `draw_with_water` → `Map` with SVG/PNG helpers (`lib.rs`, `Map`).

`draw_document(&Document, progress)` is that whole path for a document; the Node and Python `burin.draw()` / `drawScript` call it. The CLI calls `draw_with_water` itself because `--batch` sections and `--water` can add to the document.

Errors: `Error::Input` | `Net` | `Data`.

## CLI document sources

Priority (see `cli.rs` and README): `--script` runs `Document::from_script`; `--style-file` / `--style-json` load `Document`. Area flags are laid over the document's `AreaDoc` (a flag for bounds, centre or place replaces the document's choice; a fixed area drops its `size_km`), then the same `AreaDoc::resolve` runs.

## Dependencies

- `burin-map-core`, `burin-map-script`
- `terramask` (workspace path): tile fetch when enabled on this crate
- Nominatim for place lookup (network)

## Frame model

`Frame`: optional paper `size_mm` + `pen_width` sets scale; otherwise default pixel width (`px`, default 2400). Ground vs paper aspect ratio: drawing shrinks to fit (documented on `Frame` in `lib.rs`).

## Related

- Engine internals: [core-engine.md](core-engine.md)
- Script → atlas: [document-and-atlas.md](document-and-atlas.md)
