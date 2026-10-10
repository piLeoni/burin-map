# Overview

Burin Map traces **streamlines** over terrain (or distance from shore on water), then
**breaks each line into dashes** according to local tone.

## Tone `t` (0 … 1)

On **land**, `t` comes from relief and sun (`input_*`, `sun_*`):

- **0** — fully lit slope  
- **1** — full shadow  
- **`input_flat`** (default 0.25) — typical flat ground  

Spacing and ink both read this `t` (unless you quantize ink with `dash_levels`).

On **water**, the same parameters often mean **near vs far from the shore**;
see [Water & shoreline](07-water.md).

## Two independent axes

| Axis | Parameters | Question |
|------|------------|----------|
| **Between lines** | `space_light`, `space_dark`, `space_steep` | How far apart are neighbouring lines? (px) |
| **Along each line** | `dash_light`, `dash_dark`, `dash_period`, … | What fraction of each dash cycle is inked? |

Defaults are tuned for shaded contours: **wider in light**, **tighter in shadow**;
**sparse dashes in light**, **solid in shadow**. With all ink modifiers off,
behaviour is predictable (see [Ink](03-ink.md)).

## What shares this model

- Land `burin.batch({ draw: lines })` — full model  
- Land dots — spacing from the field; `dash_cutoff` gates placement  
- Water lines, shore rings — spacing/ink vs distance; see water doc  
- Shoreline — same dash parameters on drawn shore paths  
- Ripples — custom row logic but still `dash_light` / `dash_dark` for duty  
- Fills — no line spacing or dashing  

## Documents and presets

- **Contours** (`presets/contours.js`) — empty `style`: engine defaults only.  
- **`ink-*` presets** — same area, isolated behaviours; see [Ink presets](06-ink-presets.md).
