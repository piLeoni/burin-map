# Quantized ink (`dash_levels`)

With **`dash_levels`** ≥ 2, tone `t` is rounded to **N** equal steps from 0 to 1
before duty is computed. You get **flat zones** of ink instead of a smooth gradient.

Example — three zones (solid / 50% / 10%):

```js
burin.batch({ angle: 0, style: {
  dash_levels: 3,
  dash_light: 0.1,
  dash_mid: 0.5,
  dash_dark: 1,
}});
```

With **`mod_jitter: 0`**, each zone is a **mechanical** dash pattern. Borders follow
quantized tone, not randomness.

**`dash_mid`** shapes the middle of the ramp when levels are off too: at tone 0.5 the
duty is `dash_mid` if set ≥ 0.
