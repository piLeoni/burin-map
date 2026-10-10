# AI coding agents — burin-map

Pen-and-ink terrain maps: Rust drawing engine, CLI, Python/Node APIs, browser app (WASM). User-facing syntax lives in [SYNTAX.md](SYNTAX.md) and [docs/guide/](docs/guide/README.md).

## Project knowledge (read selectively)

**Do not** re-explore the whole repo on every task. Source code wins over docs when they disagree.

1. Open [docs/architecture/INDEX.md](docs/architecture/INDEX.md) and pick the subsystem(s) your task touches.
2. Read only those `docs/architecture/<subsystem>.md` files.
3. Inspect the specific symbols and files you need to change or debug.

Architecture decisions (rationale, not API): [docs/architecture/decisions.md](docs/architecture/decisions.md).

Session history and engine notes (informal): [NOTES.md](NOTES.md). Style checklist for agents (parameters): [docs/AGENTS.md](docs/AGENTS.md).

## Before you finish a substantial change

If responsibilities, interfaces, data flow, invariants, or build surfaces changed, update the affected `docs/architecture/*.md` and [INDEX.md](docs/architecture/INDEX.md) if files were added or renamed. Say what you updated, or that docs were unchanged.

## Repo layout (one glance)

| Path | Role |
|------|------|
| `crates/burin-map-core` | Pure draw engine (elevation grid → paths) |
| `crates/burin-map` | Tiles/DEM, geo, CLI, raster/SVG export |
| `crates/burin-map-script` | JS document interpreter (Boa) → atlas JSON |
| `crates/burin-map-{wasm,script-wasm,py,node}` | Bindings |
| `web/` | BurinMap UI (Vite + React) |
| `presets/*.js` | Example documents; web precomputes atlases |
| `docs/guide/` | User style guide (mirrored in web `/docs`) |

Workspace version is unified in root `Cargo.toml` (`workspace.package.version`); Python/Node packages must stay in sync (CI checks).

## Common commands

```bash
cargo test --workspace --features gpu
cargo clippy --workspace --features gpu --all-targets -- -D warnings
web/build-wasm.sh   # after changing wasm crates; commits include public/pkg*
```

See [docs/architecture/build-and-ci.md](docs/architecture/build-and-ci.md) for CI jobs and wasm packages.
