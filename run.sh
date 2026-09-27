#!/usr/bin/env bash
# Homogeneous map: identical light and dark — no variation anywhere.

PLACE=(--terrain-at "Twin Peaks, San Francisco" --terrain-size 8km)

# Space flat, dash full, zero character
burin-map "${PLACE[@]}" --out-file tp-dash.png \
  --batch-angle 90 \
  --space-light 1 --space-dark 1 --space-steep 0 \
  --dash-light 1 --dash-dark 1 --dash-period 3 --dash-drop 0 \
  --mod-jitter 0 --mod-breaks 0 --mod-wobble 0
