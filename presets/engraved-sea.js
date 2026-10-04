/**
 * Engraved sea — crosshatched hills, coast lines, then a sea of evenly waving currents.
 *
 * The old chart look: contour lines and a second pass at 45° for the hills,
 * two concentric lines along the coast, and — past those — a sea of hand-drawn
 * currents. Every current waves evenly and in step with its neighbours, each an
 * offset copy of the next, so the open water reads as engraving, not hatching.
 * Made for the screen: a plotter would take hours over the sea.
 */

burin.area({ at: "37.7,-122.62", size_km: [40, 28] });

// The hills: contours, lit and shaded, then a second angle over the steeper
// ground to deepen the relief. `batch_floor` keeps the crosshatch off the flats.
burin.batch({ angle: 0, style: { space_light: 1.8, space_dark: 0.6, space_steep: 0.3, dash_light: 0.1 } });
burin.batch({ angle: 45, style: { batch_floor: 0.6, dash_light: 0.6, space: 7.5 } });

// The coast: two concentric lines, then the distance contours just off it,
// out to 20 px. `water_start` leaves a clean gap between the two.
burin.shore({ lines: 2, style: { space: 1.5 } });
burin.water("lines", { follow: "shore", angle: 0, style: { space: 3, water_start: 3, water_reach: 20 } });

// The open sea: one direction over the whole map, waving together. This is the
// wavy modifier ('mod_wave') with `mod_wave_even`, so no two lines cross and the
// rows stay evenly spaced; `mod_wobble` lets the flow turn.
burin.water("lines", {
  follow: "map",
  angle: 90,
  style: {
    space: 4,
    space_light: 1,
    space_dark: 1,
    water_start: 26,
    water_reach: 4000,
    water_fade: 0.2,
    water_fray: 40,
    dash_light: 0.95,
    dash_dark: 1,
    dash_period: 9,
    dash_drop: 0,
    mod_jitter: 0.3,
    mod_wobble: 45,
    mod_wobble_size: 120,
    mod_smooth: 3,
    mod_wave: 2.5,
    mod_wave_size: 30,
    mod_wave_even: 1,
  },
});

burin.draw();
