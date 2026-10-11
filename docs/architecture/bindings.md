# Language bindings

Thin layers over `burin-map` (full pipeline) and/or `burin-map-core` (grid-only draw). Pack paths the same as core types where possible.

Node and Python expose the **document API**: the same calls a document script makes (`area`, `batch`, `fill`, `water`, `shore`, `draw`, `json`, `parameters`, the fields and tree helpers), so a document reads the same in a preset, the web editor, `--script`, Node and Python. `draw()` hands `{ atlas, area }` to `burin_map::draw_document` and starts an empty document. `drawScript` / `draw_script` run a document file as is. `drawElevation` / `draw_elevation` stay for a caller's own grid (one batch, a flat style). There is no bounds-and-style `draw` any more.

## Python (`crates/burin-map-py` + `python/burin_map/`)

- Native module: `_burin` (PyO3). Public API: `python/burin_map/__init__.py`.
- Document calls are module-level with Python keywords (`burin.batch(angle=0, style={…})`); module state holds the entries and area until `draw()`. They mirror `library.js` rule for rule (same entry shapes, same checks), and each entry is also checked natively (`_burin.check(kind, json)` → `burin_map_core::script::*_from`). Keep the two in step when the library changes.
- `style=` takes a dict or a `Style` dataclass (which writes every value out).
- `evaluate(script)` → `{atlas, area}` dict; `draw_document(json, log)` native.
- `force_cpu()` / `engine()` — must call `force_cpu` before first draw (see `engine.rs` panic).
- Arrays: polylines packed as bytes in native layer; numpy used from Python side.
- Build: maturin from **repo root** (`pyproject.toml` points at manifest); CI activates venv per step.

## Node (`crates/burin-map-node` + `node/`)

- N-API addon. napi writes its loader to `native.js` / `native.d.ts` (`--js` / `--dts` in `package.json`); `index.js` / `index.d.ts` are hand-written.
- `index.js` runs the library's own source (`librarySource()`, i.e. `burin_map_script::LIBRARY`) in Node's realm over a private `globalThis`, with natives backed by the addon (`check`, `atlasFromJson`, `burinParameters`, `drawDocument`). No second copy of the library. Exports are `exports.x =` assignments so ESM named imports work.
- `Drawing` with packed `Float32Array` paths / `Uint32Array` ends; `svg()` is mm-sized when the area has paper.
- `evaluate` returns the parsed `{ atlas, area }`; `forceCpu`, `engine` as in Python.
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
- [cli-orchestration.md](cli-orchestration.md) — `draw_document`, `AreaDoc::resolve`
