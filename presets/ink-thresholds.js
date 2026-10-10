/**
 * Ink: thresholds — a constant 30 % cut at both ends.
 *
 * The same 30 % everywhere, then no ink below tone 0.15 and solid above
 * tone 0.6: only the two thresholds draw the relief.
 * See docs/guide/06-ink-presets.md.
 *
 *   burin-map --script presets/ink-thresholds.js --out-file ink-thresholds.png
 */

burin.area({ at: "37.66,-122.45", size_km: [36, 25] });

burin.batch({ angle: 0, style: { dash_light: 0.3, dash_dark: 0.3, dash_cutoff: 0.15, dash_solid: 0.6 } });

burin.draw();
