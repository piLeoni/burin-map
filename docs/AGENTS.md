# Agent / LLM notes — style & parameters

**Architecture and agent workflow:** [../AGENTS.md](../AGENTS.md) and [architecture/INDEX.md](architecture/INDEX.md).

When editing Burin Map styles or explaining behaviour to users:

1. Read **`docs/guide/README.md`** (one page per draw style) and **`SYNTAX.md`** (full parameter tables).
2. **`space`** as a single base parameter is **removed**. Use **`space_light`** and **`space_dark`** in **px**.
3. **`dash_period`** is in **px**, not × spacing.
4. **`dash_drop`** is **removed** → **`mod_flat`**.
5. **`dash_light: 1` + `dash_dark: 1`** with modifiers off → continuous lines (no hidden lottery).
6. Ink preset demos: `presets/ink-*.js`; defaults document: `presets/contours.js`. Land-line defaults are hand-drawn (`mod_jitter: 1`, `mod_flat: 0.5`, `dash_cutoff: 0.15`); water, shore and dots default to `Style::plain()` (modifiers off). Presets write out a full `style` object and spread it into each batch, so they never depend on the defaults.
6. **`dash_cutoff`** removes ink below a fixed tone; **`dash_cutoff_soft`** (0 hard … 1 default) fades it in instead, survival `(x)²` across the fade. Pinned preset blocks use `dash_cutoff_soft: 0`, the earlier hard edge.
7. Draw styles: land `lines` (default) / `dots`; water `lines` / `ripples` / `dots`; `shore(…)`; `fill`. One guide page each.
7. Implement ink rules in `crates/burin-map-core/src/dashes.rs`; land spacing in `fields.rs`.

Web UI docs mirror `docs/guide/` at route **`/docs`**.
