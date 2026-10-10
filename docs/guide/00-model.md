# The shared model

Every draw style below reads the **same two axes** off the **same tone**.

## Tone `t` (0 … 1)

On **land**, `t` comes from relief and sun (`input_*`, `sun_*`):

- **0** — fully lit slope
- **1** — full shadow
- **`input_flat`** (default 0.25) — typical flat ground

On **water**, `t` usually means **distance from the shore** instead: 0 at the
shore, 1 far out (over `water_reach` / `water_space_reach`). Light = far,
dark = near.

## Axis 1 — spacing (between lines or dots)

- **`space_light`** — distance at tone 0, in px
- **`space_dark`** — distance at tone 1, in px
- **`space_steep`**, **`space_steep_gamma`** — extra tightening on steep land

## Axis 2 — ink (along a line, or how dense dots are)

Lines are walked in cycles of **`dash_period`** px. At each cycle's midpoint:

1. **`dash_cutoff`** — below this tone, no ink (0 = off)
2. **`dash_solid`** — above this tone, always full (1 = off)
3. **`mod_flat`** — on flat and lit ground, a lottery to leave a dash out
4. **`dash_levels`** — optionally round tone to N flat zones first
5. **Duty** — `dash_light` at tone 0, `dash_dark` at tone 1, `dash_mid` at 0.5
6. **`mod_jitter`** — uneven dash/period length, same average ink
7. **`mod_breaks`** — a chance to break an otherwise full period

**Guarantees:** `dash_light: 1, dash_dark: 1` with modifiers off → every line
is continuous. `dash_light: 0.5` with `mod_jitter: 0` → even dashes, tone 0.

## What is *not* this model

- **`mod_wobble`**, **`mod_wave*`**, **`mod_smooth`** shape the line's path, not
  its ink.
- **Dots** read spacing the same way, but "ink" becomes "place a dot or not"
  (see [land-dots.md](land-dots.md), [water-dots.md](water-dots.md)).
- **Fills** have neither axis — see [fills.md](fills.md).

Each style page gives its own full parameter table, since not every group
applies everywhere (land has `sun_*`/`input_*`; water has `water_*` instead).
