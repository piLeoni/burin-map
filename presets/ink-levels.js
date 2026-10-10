/**
 * Ink: levels — three flat zones, solid, 50 %, 10 %.
 *
 * The tone rounded to three levels before inking: no gradient, three
 * mechanical textures with sharp borders.
 * See docs/guide/06-ink-presets.md.
 *
 *   burin-map --script presets/ink-levels.js --out-file ink-levels.png
 */

burin.area({ at: "37.66,-122.45", size_km: [36, 25] });

burin.batch({ angle: 0, style: { dash_levels: 3, dash_light: 0.1, dash_mid: 0.5, dash_dark: 1 } });

burin.draw();
