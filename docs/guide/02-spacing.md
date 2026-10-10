# Spacing between lines

Distances are in **pixels** on the output map.

## Light and shadow

- **`space_light`** — line spacing at tone **0** (full light), default **4.8 px**  
- **`space_dark`** — spacing at tone **1** (full shadow), default **3.4 px**  

The engine interpolates between them along `t`. Steeper shadow → closer lines.

## Steep ground

- **`space_steep`** (0 … 1) — extra tightening on steep slopes  
- **`space_steep_gamma`** — how much of that tightening applies only to the steepest ground  

These multiply the light/dark spacing; they do not replace it.

## Single spacing value

When only one number is needed (tracer seeds, ripple row gap, shore step), the code uses
**`space_light`** as the base (`Style::space_base()`).

## Water

Near the shore spacing tends toward **`space_dark`**; farther out it eases to
**`space_light`** over **`water_space_reach`** px. That is independent of
**`water_reach`** (how far ink/tracing stay active).
