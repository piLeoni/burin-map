/**
 * Downhill — lines running down every slope.
 *
 * Denser in shadow, and a stippled sea. `angle: 90` runs straight downhill
 * rather than following the contours.
 */

burin.area({ at: "37.66,-122.45", size_km: [36, 25] });

burin.batch({
  angle: 90,
  style: {
    space_light: 9,
    space_dark: 2.4,
    space_steep: 0.15,
    dash_light: 1,
    dash_dark: 1,
    mod_breaks: 0.03,
    mod_smooth: 8,
  },
});

burin.shore({ lines: 2, style: { space_light: 2 } });
burin.water("dots", { follow: "shore", style: { space_light: 25, space_dark: 3, water_start: 3 } });

burin.draw();
