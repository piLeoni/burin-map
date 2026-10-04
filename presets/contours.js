/**
 * Contours — the default document.
 *
 * Contour lines, lit and shaded: ink gathers where the ground falls away
 * from the sun, thins on the flats. Run it:
 *
 *   burin-map --script presets/contours.js --out-file contours.png
 *
 * The area is a suggestion; a command-line flag wins over it.
 */

burin.area({ at: "37.66,-122.45", size_km: [36, 25] });

// One pass along the contours. `angle: 0` follows them; the light decides
// how much ink, so nothing else is set.
burin.batch({ angle: 0, style: {} });

burin.draw();
