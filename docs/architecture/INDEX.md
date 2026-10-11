# Architecture index

Compact map of subsystems. Read **only** the rows relevant to your task, then open the linked doc.

| Subsystem | Doc | When to read |
|-----------|-----|--------------|
| Draw engine | [core-engine.md](core-engine.md) | Fields, streamlines, dashes, ink, GPU/CPU fields, water depth, `Atlas::draw` |
| CLI & I/O | [cli-orchestration.md](cli-orchestration.md) | DEM/tiles, geocoding, `Document` JSON, PNG/SVG, `burin-map` binary |
| Documents & atlas | [document-and-atlas.md](document-and-atlas.md) | JS presets, `burin-map-script`, atlas JSON, `--script` / `evaluate()` |
| Bindings | [bindings.md](bindings.md) | Python, Node (document API), WASM draw + script WASM |
| Web app | [web-app.md](web-app.md) | React UI, worker draw, Mapbox frame, bathymetry tiles |
| Build & CI | [build-and-ci.md](build-and-ci.md) | `build-wasm.sh`, toolchains, GitHub Actions, `--force-cpu` |

**Decisions (why, not how):** [decisions.md](decisions.md)

**Not architecture (user docs):** [../guide/README.md](../guide/README.md), [../../SYNTAX.md](../../SYNTAX.md), [../AGENTS.md](../AGENTS.md) (style checklist).

**Informal history:** [../../NOTES.md](../../NOTES.md)
