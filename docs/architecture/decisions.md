# Architectural decisions

Short rationale for choices that are easy to misread from code alone. Verify in source when implementing.

## One typed graph, many front ends

The engine reads an **atlas**: ordered land `Batch` passes plus optional `Water` (shoreline + water batches). JavaScript documents (`presets/*.js`) call `burin.batch` / `burin.water` / `burin.shore` / `burin.area` / `burin.draw()`; the interpreter (`burin-map-script`) emits atlas JSON. CLI `--script`, Python/Node `evaluate()`, and the web editor all use that path.

Flat JSON5 `Document` (`crates/burin-map/src/document.rs`) remains for `--style-file`, share links, and overrides — not the primary authoring model. See [document-and-atlas.md](document-and-atlas.md).

## GPU by default, CPU for reproducibility

Native builds use GPU terrain fields when available (`burin-map-core` `engine` + `gpu`). Users do not pick a device in the UI. `--force-cpu`, `BURIN_ENGINE=cpu`, `?force-cpu` (web), `force_cpu()` / `forceCpu()` exist because GPU f32 vs CPU f64 changes dash placement — needed for golden tests and comparable bug reports. Late `force_cpu()` panics after the device is resolved.

Browser: WebGPU is chosen in JS/WASM, not via `engine.rs` (native-only).

## Web: script text + atlas, not parallel document models

The page stores editor script text and the evaluated **atlas**. Drawing always goes atlas → engine (worker + `burin-map-wasm`). Presets ship with precomputed `web/src/presets/atlases.json` so unchanged presets draw without loading the script WASM. See [web-app.md](web-app.md).

## Committed WASM in `web/public/`

Draw and script interpreters are built with `web/build-wasm.sh` and committed under `web/public/pkg*`. CI checks exports/size for `pkg`, not byte-identical rebuilds across machines. See [build-and-ci.md](build-and-ci.md).

## External data

## Defaults: hand-drawn land lines, plain everything else; presets pin every value

`Style::default()` is the land-lines look (`mod_jitter: 1`, `mod_flat: 0.5`, `dash_cutoff: 0.15`, ink rather than spacing carrying the shadow). Water (`Style::water`), the shore (`Style::shoreline`) and dots (`Batch::dots`) build on `Style::plain()` instead: even ink, modifiers off, the earlier defaults. Every preset writes out a full style object and spreads it into each batch, so a later change of defaults never changes a preset; `presets/contours.js` spells out the current defaults. Checking that a style passed to a batch sets every parameter was considered and not done: partial styles are what the bindings, CLI flags and short examples rely on.

`dash_cutoff` is a fixed tone, not a share of the frame, so a place draws the same in any view. Its edge is set by `dash_cutoff_soft` (default 1: ink fades in from tone 0, a dash surviving with chance `(t / cutoff)²`; 0: the earlier hard edge). Soft is the default because lit slopes are clamped to `input_light_floor`, so a hard edge removes them all within a hundredth of the value.

Elevation: Terrarium PNG tiles (via `terramask`, optional fetch in CLI). Geocoding: Nominatim. Water vectors: OpenMapTiles-style tiles (default OpenFreeMap). Bathymetry for depth expressions may need separate fetches in the web app.
