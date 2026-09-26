#!/usr/bin/env bash
# Same scene, two shading grammars.

PLACE=(--terrain-at "Twin Peaks, San Francisco" --terrain-size 8km)

# Even dashes, no drop, no breaks
burin-map "${PLACE[@]}" --out-file tp-dash.png \
  --dash-light 0.5 --dash-dark 0.5 --dash-period 5 --dash-drop 0 \
  --mod-jitter 0.8 --mod-breaks 0
