# Ink presets (`presets/ink-*.js`)

Same area as **Contours** (`37.66,-122.45`, 36×25 km). Run:

```bash
burin-map --script presets/ink-<name>.js --out-file out.png
```

In the web app, pick them from the preset menu (labels start with **Ink:**).

| Preset | Teaches |
|--------|---------|
| **ink-classic** | Previous default *look*: `dash_cutoff`, `mod_jitter`, `mod_flat`, `mod_breaks` |
| **ink-solid** | `dash_light: 1`, `dash_dark: 1` — contour lines only |
| **ink-half** | `dash_light: 0.5`, `dash_dark: 1`, no jitter — mechanical 50% in light |
| **ink-jitter** | Same as half with `mod_jitter: 1` — same average, rougher |
| **ink-levels** | `dash_levels: 3` with 10% / 50% / 100% zones |
| **ink-flat** | Half + `mod_flat: 1` only |
| **ink-thresholds** | Constant 30% duty, `dash_cutoff` / `dash_solid` carving zones |

**Contours** itself uses engine defaults (modifiers off). For the classic engraved
look, start from **ink-classic** or copy its `style` block.
