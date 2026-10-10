# Ink modifiers

All default to **0** (off) except path modifiers like **`mod_wobble`** on land.

## `mod_jitter`

Uneven dashes and slightly uneven period length. Strength is capped by
`min(duty, 1 − duty)` so mechanical **full** or **empty** periods stay put.
Use for a hand-drawn look without changing average ink.

## `mod_flat`

Replaces the old **`dash_drop`**. On ground with tone below **`input_flat`**, each
period may be skipped or bumped to flat-ground ink, with probability **`mod_flat`**.
Lighter tones are more likely to stay empty. Breaks monotony on plains; not a hard
threshold (use **`dash_cutoff`** / **`dash_solid`** for that).

## `mod_breaks`

Small random gaps inside otherwise **full** periods. Does not change average duty on
dashed areas; only affects runs that would be solid.

## `dash_cutoff` and `dash_solid`

Hard tone gates applied **before** flat lottery and duty:

- Below **`dash_cutoff`** — no ink  
- Above **`dash_solid`** — always solid; modifiers do not shorten those periods  

## Path character (not ink share)

These move or smooth the line geometry, not the dash duty:

- **`mod_wobble`**, **`mod_wobble_size`** — wander on flat ground  
- **`mod_wave`**, **`mod_wave_size`**, **`mod_wave_zone`**, **`mod_wave_even`** — side-to-side motion  
- **`mod_smooth`** — smoother terrain following  

See [SYNTAX.md](../../SYNTAX.md) for defaults and water-specific notes.
