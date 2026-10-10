# Agent / LLM notes — style & parameters

**Architecture and agent workflow:** [../AGENTS.md](../AGENTS.md) and [architecture/INDEX.md](architecture/INDEX.md).

When editing Burin Map styles or explaining behaviour to users:

1. Read **`docs/guide/README.md`** (one page per draw style) and **`SYNTAX.md`** (full parameter tables).
2. **`space`** as a single base parameter is **removed**. Use **`space_light`** and **`space_dark`** in **px**.
3. **`dash_period`** is in **px**, not × spacing.
4. **`dash_drop`** is **removed** → **`mod_flat`**.
5. **`dash_light: 1` + `dash_dark: 1`** with modifiers off → continuous lines (no hidden lottery).
6. Ink preset demos: `presets/ink-*.js`; defaults document: `presets/contours.js`.
7. Draw styles: land `lines` (default) / `dots`; water `lines` / `ripples` / `dots`; `shore(…)`; `fill`. One guide page each.
7. Implement ink rules in `crates/burin-map-core/src/dashes.rs`; land spacing in `fields.rs`.

Web UI docs mirror `docs/guide/` at route **`/docs`**.
