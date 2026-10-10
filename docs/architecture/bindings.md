# Language bindings

Thin layers over `burin-map` (full pipeline) and/or `burin-map-core` (grid-only draw). Pack paths the same as core types where possible.

## Python (`crates/burin-map-py` + `python/burin_map/`)

- Native module: `_burin` (PyO3). Public API: `python/burin_map/__init__.py`.
- `draw` / `draw_elevation`, `Style`, `Frame`, geocoding helpers mirror CLI concepts.
- `evaluate(script)` → atlas + area (script interpreter in Rust).
- `force_cpu()` / `engine()` — must call `force_cpu` before first draw (see `engine.rs` panic).
- Arrays: polylines packed as bytes in native layer; numpy used from Python side.
- Build: maturin from **repo root** (`pyproject.toml` points at manifest); CI activates venv per step.

## Node (`crates/burin-map-node` + `node/`)

- N-API addon; `index.js` + `cli.js` for `npx burin-map`.
- `Drawing` with packed `Float32Array` paths / `Uint32Array` ends.
- `evaluate`, `forceCpu`, `engine` analogous to Python.
- Tests: `node --test`.

## WASM — draw (`crates/burin-map-wasm`)

- Built to `web/public/pkg` (and copy `web/pkg`) via `web/build-wasm.sh`.
- Exports: `style_params`, `style_groups`, draw entry points, packed path buffers, progress hooks, optional `init_thread_pool` (`threads` feature → `pkg-mt`).
- Browser loads from `/pkg/burin_map_wasm.js` (`web/src/lib/burin.js`, worker in `burin.worker.js`).
- WebGPU field path: feature `gpu`; isolation/COOP for threaded build documented in build script.

## WASM — script (`crates/burin-map-script-wasm`)

- `evaluate(source)` → JSON string `{ atlas, area }`.
- `librarySource()` for editor typings beside `burin.d.ts`.
- Profile `wasm-script` in root `Cargo.toml` (size-optimized).
- Output: `web/public/pkg-script/`.

## Version sync

Workspace `version` in root `Cargo.toml`; CI checks `node/package.json` (and maturin package version) stay aligned.

## Related

- [web-app.md](web-app.md) — how the page loads pkg vs pkg-script
- [build-and-ci.md](build-and-ci.md) — wasm CI checks
