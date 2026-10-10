#!/usr/bin/env bash
# Homogeneous map: identical light and dark — no variation anywhere.

PLACE=(--terrain-at "Twin Peaks, San Francisco" --terrain-size 8km)

# Space flat, dash full, zero character
burin-map "${PLACE[@]}" --out-file tp-dash.png \
  --batch-angle 90 \
  --space-light 4.8 --space-dark 4.8 --space-steep 0 \
  --dash-light 1 --dash-dark 1 --dash-period 14.4 \
  --mod-wobble 0
