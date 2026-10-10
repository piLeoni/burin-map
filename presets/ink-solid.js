/**
 * Ink: solid — contour lines only.
 *
 * Full ink in light and in shadow, no modifier: every line is whole.
 * See docs/guide/land-lines.md.
 *
 *   burin-map --script presets/ink-solid.js --out-file ink-solid.png
 */

burin.area({ at: "37.66,-122.45", size_km: [36, 25] });

burin.batch({ angle: 0, style: { dash_light: 1, dash_dark: 1 } });

burin.draw();
