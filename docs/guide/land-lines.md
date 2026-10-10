# Land — lines

The default draw style: streamlines traced over the terrain, dashed by light
and shadow. `burin.batch({ angle, style })`, or just `burin.batch({})` for the
engine defaults.

```js
burin.batch({ angle: 0, style: { dash_light: 0.5, dash_dark: 1 } });
```

## Parameters

| Group | Parameter | Default | Carries over |
|---|---|---|---|
| Space | `space_light` | 4.8 | ✓ |
| | `space_dark` | 3.4 | ✓ |
| | `space_steep` | 0.4 | ✓ |
| | `space_steep_gamma` | 1 | ✓ |
| Dash | `dash_light` | 0.15 | |
| | `dash_dark` | 1 | |
| | `dash_mid` | -1 | |
| | `dash_period` | 10.5 | |
| | `dash_levels` | 0 | |
| | `dash_cutoff` | 0 | |
| | `dash_solid` | 1 | |
| Mod | `mod_jitter` | 0 | |
| | `mod_flat` | 0 | |
| | `mod_breaks` | 0 | |
| | `mod_wobble` | 20 | ✓ |
| | `mod_wobble_size` | 12 | ✓ |
| | `mod_wave` | 0 | |
| | `mod_wave_size` | 16 | |
| | `mod_wave_zone` | 0 | |
| | `mod_wave_even` | 0 | |
| | `mod_smooth` | 6 | ✓ |
| Input | `input_flat` | 0.25 | ✓ |
| | `input_gain` | 3 | ✓ |
| | `input_dark_gamma` | 0.7 | ✓ |
| | `input_light_gain` | 1.5 | ✓ |
| | `input_light_floor` | 0.06 | ✓ |
| | `input_blur` | 1.8 | ✓ |
| | `input_broad_blur` | 11.5 | ✓ |
| | `input_broad` | 0.5 | ✓ |
| | `input_height` | 0 | |
| Sun | `sun_azimuth` | 315 | ✓ |
| | `sun_altitude` | 45 | ✓ |
| | `sun_exag` | 4 | ✓ |
| Engine | `engine_step` | 1 | |
| | `engine_min` | 4 | |
| | `engine_stop` | 0.5 | |
| | `engine_fill` | 0.8 | |
| | `engine_stripes` | 0 | |
| Batch | `batch_floor` | 0 | |
| | `batch_stroke` | 1.2 | |
| | `batch_seed` | 1 | |

**Carries over (✓):** in a document with several batches, a batch that leaves one of these out
takes it from the batch above, so a second pass (say a 45° crosshatch) follows the same light and
spacing as the first without repeating them. Every other parameter left out goes back to its
default. Details: [SYNTAX.md](../../SYNTAX.md#carries-over).

## Spacing: `space_light` / `space_dark`

![Spacing comparison: uniform, default, exaggerated](img/land-lines-spacing.png)

Left to right: no variation (`space_light` = `space_dark` = 5), the default
(wider in light, tighter in shadow), and an exaggerated gap for teaching.

## Ink: duty and jitter

![Ink comparison: solid, mechanical, jitter](img/land-lines-ink.png)

Left to right: `dash_light: 1, dash_dark: 1` (always solid), `dash_light: 0.5`
(mechanical 50% dashes in light), and the same with `mod_jitter: 1` (same
average ink, hand-drawn look).

## Modifiers: `mod_flat`, `mod_breaks`

![Modifiers comparison: off, mod_flat, mod_breaks](img/land-lines-modifiers.png)

Left to right: modifiers off, `mod_flat: 1` (lottery on flat/lit ground),
`mod_breaks: 1` (random breaks in otherwise solid dashes).

## Quantized ink: `dash_levels`

![Levels comparison: smooth vs quantized](img/land-lines-levels.png)

Left: the default smooth gradient. Right: `dash_levels: 3` — tone rounded to
three flat zones (10% / 50% / solid) before duty is computed.

## Ink presets

`presets/ink-*.js` isolate each behaviour above, all on the same area as
Contours. Below, the same few hills from each:

![Ink presets: classic, solid, half, jitter, levels, flat, thresholds](img/ink-presets.png)

Run any of them, e.g.:

```bash
burin-map --script presets/ink-levels.js --out-file out.png
```
