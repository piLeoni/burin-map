/**
 * Ink: flat — ink-half with only the flat-ground lottery on.
 *
 * On flat and lit ground dashes are left out or take the flat ink at
 * random; slopes keep their mechanical dashes.
 * See docs/guide/land-lines.md.
 *
 *   burin-map --script presets/ink-flat.js --out-file ink-flat.png
 */

burin.area({ at: "37.66,-122.45", size_km: [36, 25] });

burin.batch({ angle: 0, style: { dash_light: 0.5, dash_dark: 1, mod_flat: 1 } });

burin.draw();
