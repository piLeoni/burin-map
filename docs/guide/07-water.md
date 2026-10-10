# Water and shoreline

Water batches use the **same parameter names**; the driving scalar is usually
**distance from shore**, not hillshade.

## Spacing

- **`space_dark`** — tighter spacing **near** the shore (px)  
- **`space_light`** — wider spacing **far** out (px)  
- Ease over **`water_space_reach`** px (not `water_reach`)  

Lower **`space_dark`** means **closer** rings near the coast, not “darker ink”.

## Ink

- **`dash_dark`** — more ink **near** the shore  
- **`dash_light`** — less ink farther out  
- Ease over **`water_reach`**  

Shoreline paths use the same dash pipeline with a uniform tone per ring.

## Defaults

`Style::water()` and `Style::shoreline()` set their own defaults (see
[SYNTAX.md](../../SYNTAX.md) water tables). Ripples and dots use
`space_light` / `space_dark` for row or dot spacing with specialised placement code.
