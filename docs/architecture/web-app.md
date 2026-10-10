# Web app (`web/`)

BurinMap: Vite + React (Jotai state). Same engine as CLI via WASM in a Web Worker.

## Bootstrap

- `web/src/main.jsx` → `Root.jsx` → `App.jsx`
- State: `web/src/state.js` (`atlasAtom`, `drawingAtom`, frame/map atoms, `forceCpu` from URL)

## Document → draw pipeline

1. Sidebar/editor holds **script text**; on change, `document.js` `runJs` evaluates to `{ atlas, area }` (unless using frozen precomputed atlas for presets).
2. `App.jsx` derives `batches` / `water` via `atlasBatches` / `atlasWater` (`document.js`).
3. `MapView.jsx` loads DEM from Mapbox terrain, optional bathymetry (`lib/bathymetry.js`, `lib/dem.js`), water tiles (`lib/water.js`), then `drawBatches` in worker (`lib/burin.js` → `burin.worker.js`).

Key files:

| Area | Files |
|------|--------|
| Interpreter | `lib/document.js`, `public/pkg-script/*` |
| Engine | `lib/burin.js`, `lib/burin.worker.js`, `public/pkg/*`, optional `pkg-mt` |
| Frame / ground size | `lib/frame.js`, `state.js` |
| Map UX | `components/MapView.jsx`, `Sidebar.jsx`, `LoadPanel.jsx` |
| Docs route | `src/docs/` — see below |
| Presets | `lib/presets.js`, `presets/` at repo root, `src/presets/atlases.json` |

## Docs route (`/docs/*`)

- `Docs.jsx`: grouped sidebar + routes. `Overview.jsx` and `Examples.jsx` are hand-written JSX; the per-style pages render `docs/guide/*.md` (`?raw`, `guideSections.js`) through `MarkdownBody.jsx` (react-markdown + remark-gfm).
- Figures: a guide image `img/<id>.png` whose `<id>` is in `src/docs/figures.json` is replaced by `ZoomFigure.jsx` — the variants' SVGs side by side, zoomed/panned together. On GitHub the same markdown shows the PNG.
- Generated assets: `public/docs/svg/*.svg` (engine SVG cropped to the shown box and simplified), `public/docs/svg/example-*.svg` (whole preset maps, for Examples and the Overview), `docs/guide/img/*.png` (GitHub only; the site uses the SVGs), and `figures.json`. They are rendered from the release CLI by `web/scripts/docs-figures/figures.py`; regenerate after changing defaults or presets.
- `public/docs/**` is excluded from the PWA precache (`vite.config.js` `globIgnores`).

## Mapbox

`MapView` uses `mapbox-gl` for basemap and camera; drawing canvas overlays georeferenced output. `VITE_MAPBOX_TOKEN` env. Password gate: `components/PasswordGate.jsx` (deploy config).

## CPU forcing

`?force-cpu` (and state atom) must be applied before first WASM draw — same reproducibility contract as native `force_cpu`.

## Stubs / build

Vite stubs for optional deps: `web/scripts/stub-*.js`. Production build: `vite build`; oxlint in CI.

## Related

- [bindings.md](bindings.md) — WASM packages
- [document-and-atlas.md](document-and-atlas.md) — atlas shape
- User-facing style pages: `docs/guide/` (not duplicated here)
