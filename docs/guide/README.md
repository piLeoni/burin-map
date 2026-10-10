# Style guide

How Burin Map decides **how far apart lines are** and **how much of each line is ink**.
Same `style` object everywhere: land batches, water, shoreline. Full parameter tables
live in [SYNTAX.md](../../SYNTAX.md).

**On the web app:** open **Docs** (`/docs` on the site) for diagrams and preset previews.

## Contents

1. [Overview](01-overview.md) — tone, two axes (spacing vs ink), where it applies
2. [Spacing](02-spacing.md) — `space_light`, `space_dark`, steep ground, water
3. [Ink along the line](03-ink.md) — duty, period, guarantees (1+1 = solid, 0.5 = even dashes)
4. [Modifiers](04-modifiers.md) — jitter, flat lottery, breaks, cutoff / solid
5. [Quantization](05-levels.md) — `dash_levels`, `dash_mid`
6. [Ink presets](06-ink-presets.md) — the `ink-*` scripts as teaching examples
7. [Water & shoreline](07-water.md) — same names, distance-from-shore tone

## Quick recipes

```js
// Contour lines only (no dashes, modifiers off)
burin.batch({ angle: 0, style: { dash_light: 1, dash_dark: 1 } });

// Mechanical half-ink in light, solid in shadow
burin.batch({ angle: 0, style: { dash_light: 0.5, dash_dark: 1 } });

// Old default look (modifiers on)
burin.batch({ angle: 0, style: {
  dash_cutoff: 0.03, mod_jitter: 0.8, mod_flat: 1, mod_breaks: 0.15,
}});

// Three flat ink zones: solid / 50% / 10%
burin.batch({ angle: 0, style: {
  dash_levels: 3, dash_light: 0.1, dash_mid: 0.5, dash_dark: 1,
}});
```

## For tools and agents

- Parameter names and defaults: `Style::PARAMS` in `crates/burin-map-core/src/style.rs`
- Ink logic: `crates/burin-map-core/src/dashes.rs` (order: cutoff → solid → flat → quantize → duty → jitter → breaks)
- Land spacing field: `crates/burin-map-core/src/fields.rs`
- Shipped presets: `presets/*.js`, listed in `web/src/lib/presets.js`
