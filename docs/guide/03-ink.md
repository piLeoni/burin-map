# Ink along the line

Each streamline is walked in cycles of length **`dash_period`** px (default **10.5**).
At the midpoint of each cycle the engine decides how much of that cycle is ink.

## Duty (share of ink)

- **`dash_light`** — ink share at tone **0**, default **0.15**  
- **`dash_dark`** — ink share at tone **1**, default **1**  
- **`dash_mid`** — optional share at tone **0.5**; below **0** = straight line between light and dark  

`Style::duty(t)` returns the target fraction before modifiers.

## Mechanical pattern

With **`mod_jitter`**, **`mod_flat`**, and **`mod_breaks`** at **0**:

- Duty **1** → the whole period is ink (adjacent full periods merge into one stroke).  
- Duty **0.5** → first half of the period inked, second half gap; equal lengths.  
- Duty **0.15** → short dash, long gap.  

## Guarantees

| Setting | Result (modifiers off) |
|---------|-------------------------|
| `dash_light: 1`, `dash_dark: 1` | Every line continuous |
| `dash_light: 0.5`, tone 0 | Regular 50% dashes |
| `mod_jitter: 0` | Period length fixed (except optional period jitter scale when jitter > 0) |

With **`mod_jitter` > 0**, dash length varies but **average ink** stays at the duty;
full duty **1** or **0** is not shifted.

## Processing order

For each period (see `dashes.rs`):

1. **`dash_cutoff`** — below this tone, skip period (0 = off)  
2. **`dash_solid`** — above this tone, full period (1 = off)  
3. **`mod_flat`** — lottery on flat lit ground (`t < input_flat`)  
4. **`dash_levels`** — quantize tone (optional)  
5. **Duty** from light / mid / dark  
6. **`mod_jitter`** — vary ink length and period, preserve mean  
7. **`mod_breaks`** — chance to end a full period early  
