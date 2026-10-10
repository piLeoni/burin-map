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

Elevation: Terrarium PNG tiles (via `terramask`, optional fetch in CLI). Geocoding: Nominatim. Water vectors: OpenMapTiles-style tiles (default OpenFreeMap). Bathymetry for depth expressions may need separate fetches in the web app.
