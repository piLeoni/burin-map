/**
 * Ink: jitter — ink-half by hand.
 *
 * As ink-half with full jitter: dashes and periods vary, the average ink
 * stays the same, solid lines stay solid.
 * See docs/guide/land-lines.md.
 *
 *   burin-map --script presets/ink-jitter.js --out-file ink-jitter.png
 */

burin.area({ at: "37.66,-122.45", size_km: [36, 25] });

burin.batch({ angle: 0, style: { dash_light: 0.5, dash_dark: 1, mod_jitter: 1 } });

burin.draw();
