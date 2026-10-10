/**
 * Ink: classic — the old default look, rebuilt from the modifiers.
 *
 * The default ink, plus every modifier the old default had on: uneven
 * dashes, the flat-ground lottery, small breaks and a cutoff in full light.
 * See docs/guide/06-ink-presets.md.
 *
 *   burin-map --script presets/ink-classic.js --out-file ink-classic.png
 */

burin.area({ at: "37.66,-122.45", size_km: [36, 25] });

burin.batch({ angle: 0, style: {
  dash_cutoff: 0.03,
  mod_jitter: 0.8,
  mod_flat: 1,
  mod_breaks: 0.15,
} });

burin.draw();
