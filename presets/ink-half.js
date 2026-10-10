/**
 * Ink: half — mechanical dashes in light, solid in shadow.
 *
 * Half the period inked at tone 0, solid at tone 1, no jitter: in full
 * light dash and gap are the same length.
 * See docs/guide/06-ink-presets.md.
 *
 *   burin-map --script presets/ink-half.js --out-file ink-half.png
 */

burin.area({ at: "37.66,-122.45", size_km: [36, 25] });

burin.batch({ angle: 0, style: { dash_light: 0.5, dash_dark: 1 } });

burin.draw();
