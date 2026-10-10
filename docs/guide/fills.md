# Fills

A flat-colour band over land or sea, by height or depth — no spacing axis, no
ink axis: `burin.batch({ draw: "fill", ground, where, color | color_expr })`.

```js
burin.batch({ draw: "fill", ground: "sea", where: { depth_min: 200 }, color: "#2a4a63" });
```

## Parameters

Fills take no terrain parameters (they neither read nor hand on `space_*`,
`input_*`, `sun_*`, `mod_wobble*`, `mod_smooth`): a fill's edge comes only
from `where` (the height/depth band) and `input_blur` (how smooth that edge
is). The only style knob that matters is:

| Group | Parameter | Default | What it does |
|---|---|---|---|
| Input | `input_blur` | 1.8 | smooths the edge between bands |

Everything else about a fill's look is on the batch itself, not `style`:
`color`, `color_expr` (a tone-driven gradient, see `burin.lerp`), and `where`
(`height_min`/`height_max` on land, `depth_min`/`depth_max` on sea).

## Example

`presets/layer-tints.js` and `presets/contour-belts.js` loop over bands to
build a coloured relief under the contour lines; see those scripts for the
full `color_expr` / loop pattern.
