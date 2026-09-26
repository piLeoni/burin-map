#!/usr/bin/env bash
# Same scene, two shading grammars.

PLACE=(--at "Twin Peaks, San Francisco" --size 8km)

# # Gap shading: continuous lines; light/shadow = denser / thinner spacing
# burin-map "${PLACE[@]}" -o tp-combined.png \
#   --hatch --spacing 6 \
#   --shade-dash 0 --shade-gap 1 \
#   --spacing-dark 0.5 --spacing-lit 1.8 --spacing-steep 1 --angle 90 \
#   --hatch --spacing 6 --breaks 0.45

# Classic dash shading (previous default look)
burin-map "${PLACE[@]}" -o tp-dash.png \
    --duty-min 0.5 --duty-max 0.5 --period 5 --jitter 0.8 --breaks 0
