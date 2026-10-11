# Architectural decisions

Short rationale for choices that are easy to misread from code alone. Verify in source when implementing.

## One typed graph, many front ends

The engine reads an **atlas**: ordered land `Batch` passes plus optional `Water` (shoreline + water batches). JavaScript documents (`presets/*.js`) call `burin.batch` / `burin.water` / `burin.shore` / `burin.area` / `burin.draw()`; the interpreter (`burin-map-script`) emits atlas JSON. CLI `--script`, the web editor, and Node and Python all use that path.

**Bindings speak the document.** Node and Python expose the document calls themselves (`burin.area` … `burin.draw()`), not a bounds-plus-one-style `draw`: one vocabulary everywhere, and multi-batch drawings, water and the shore reachable from code. Node runs the library's own JS source, so it cannot drift; Python mirrors it by hand and checks each entry natively. Area resolution lives in `AreaDoc::resolve`, shared by the CLI and `draw_document`, so a document draws the same place from every front end.

Flat JSON5 `Document` (`crates/burin-map/src/document.rs`) remains for `--style-file`, share links, and overrides — not the primary authoring model. See [document-and-atlas.md](document-and-atlas.md).

## Regions: names in the atlas, data at draw time, pixels in the core

Areas a batch keeps to or gives way to (`within` / `outside`) are named in the atlas but supplied with the draw call, like elevation and water coverage, so atlases stay small and serialisable and one atlas draws against different data. `burin-map-core` takes them as pixel coverage, pixel rings or signed distance; lon/lat polygons and GeoJSON belong to `burin-map`, which owns the projection. The fade scales the shading tone rather than replacing it, so steep dark ground keeps ink further into a fade; the ragged edge reuses water's `fray_along`. Regions named together merge as a union (no seam where two touch); `within` and `outside` combine by the minimum. A region a batch names but the host didn't give is an error, not a silent full or empty batch. See [../regions.md](../regions.md).

## GPU by default, CPU for reproducibility

Native builds use GPU terrain fields when available (`burin-map-core` `engine` + `gpu`). Users do not pick a device in the UI. `--force-cpu`, `BURIN_ENGINE=cpu`, `?force-cpu` (web), `force_cpu()` / `forceCpu()` exist because GPU f32 vs CPU f64 changes dash placement — needed for golden tests and comparable bug reports. Late `force_cpu()` panics after the device is resolved.

Browser: WebGPU is chosen in JS/WASM, not via `engine.rs` (native-only).

## Web: script text + atlas, not parallel document models

The page stores editor script text and the evaluated **atlas**. Drawing always goes atlas → engine (worker + `burin-map-wasm`). Presets ship with precomputed `web/src/presets/atlases.json` so unchanged presets draw without loading the script WASM. See [web-app.md](web-app.md).

## Committed WASM in `web/public/`

Draw and script interpreters are built with `web/build-wasm.sh` and committed under `web/public/pkg*`. CI checks exports/size for `pkg`, not byte-identical rebuilds across machines. See [build-and-ci.md](build-and-ci.md).

## Defaults: hand-drawn land lines, plain everything else; presets pin every value

`Style::default()` is the land-lines look (`mod_jitter: 1`, `mod_flat: 0.5`, `dash_cutoff: 0.15`, ink rather than spacing carrying the shadow). Water (`Style::water`), the shore (`Style::shoreline`) and dots (`Batch::dots`) build on `Style::plain()` instead: even ink, modifiers off, the earlier defaults. Every preset writes out a full style object and spreads it into each batch, so a later change of defaults never changes a preset; `presets/contours.js` spells out the current defaults. Checking that a style passed to a batch sets every parameter was considered and not done: partial styles are what the bindings, CLI flags and short examples rely on.

`dash_cutoff` is a fixed tone, not a share of the frame, so a place draws the same in any view. Its edge is set by `dash_cutoff_soft` (default 1: ink fades in from tone 0, a dash surviving with chance `(t / cutoff)²`; 0: the earlier hard edge). Soft is the default because lit slopes are clamped to `input_light_floor`, so a hard edge removes them all within a hundredth of the value.

## External data

Elevation: Terrarium PNG tiles (via `terramask`, optional fetch in CLI). Geocoding: Nominatim. Water vectors: OpenMapTiles-style tiles (default OpenFreeMap). Bathymetry for depth expressions may need separate fetches in the web app.
