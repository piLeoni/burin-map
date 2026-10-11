# Documents and atlas

**Atlas** = ordered list of land `Batch` values + optional `Water` (vector passes + shoreline). This is what the engine draws; everything else is a front end that produces it.

## JavaScript documents (`burin-map-script`)

- Crate: `crates/burin-map-script` — Boa interpreter, sandboxed (no I/O, no `Math.random`; loop/recursion limits in `lib.rs`).
- Embedded API: `src/library.js` (included as `burin:library`).
- `evaluate_document(source)` → `Document { atlas: Value, area: Option<Value> }`.
- Errors: `line N column M: …` (`ScriptError`, `At`).
- `ACTIVE_PATH` thread-local: host can set path so errors point at user file (web editor).

Script surface (accumulating calls, then `burin.draw()`):

```js
burin.area({ at: "…", size_km: [w, h] });
burin.batch({ angle: 0, style: { … } });
burin.water("lines", { … });
burin.shore({ … });
burin.draw();
```

Examples: `presets/*.js`. Default reference: `presets/contours.js`.

## Core typing (`burin-map-core::script`)

`src/script.rs` maps JSON/atlas objects to `Batch`, `WaterBatch`, `Shoreline`, `Band`, expression trees. Same shapes for Rust `Atlas` builder (`src/atlas.rs`) and deserialized JSON.

`Atlas::batch` / `water` / `shore` mirror the JS library semantics.

A land batch may name regions in `within` / `outside` (lines and dots only; fills refuse them). Only the names are in the atlas: the region data comes with the draw call (`Surface::regions`, `draw_document(…, regions, …)`, `--region`, `draw(regions=…)`). See [../regions.md](../regions.md).

## Saved documents (`burin-map::document`)

`Document` in `crates/burin-map/src/document.rs`: optional `area`, `batches`, `water`, frame fields — used by CLI `--style-file`, bindings, and legacy share payloads. Not the primary authoring path for the web app (JS-only editor).

## Web-specific flows

- Run script: `web/src/lib/document.js` → `pkg-script` WASM `evaluate` → `{ atlas, area }`.
- Share link `#d=…` may carry a flat atlas; `atlasToScript` can round-trip to editor text (see comments in `document.js`).
- Precomputed atlases: `web/src/presets/atlases.json` (from preset scripts) for fast first paint.

## CLI / bindings

- CLI `--script PATH`: `Document::from_script` then draw (`cli.rs`); `--region NAME=PATH` (GeoJSON, repeatable) supplies the regions batches name.
- Python / Node: the same `burin.*` calls in the host language, `burin.draw()` → `burin_map::draw_document`; `drawScript` / `draw_script` run a document file; `evaluate(script)` → `{ atlas, area }`. See [bindings.md](bindings.md).

## Related

- Engine consumption: [core-engine.md](core-engine.md)
- WASM evaluate package: [bindings.md](bindings.md)
- UI wiring: [web-app.md](web-app.md)
