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

// Every parameter, written out, so this drawing stays the same when the
// engine's defaults change. Each pass below starts from these.
const style = {
  space_light: 4.8, space_dark: 3.4, space_steep: 0.4, space_steep_gamma: 1,
  dash_light: 0.15, dash_dark: 1, dash_mid: -1, dash_period: 10.5, dash_levels: 0,
  dash_cutoff: 0, dash_cutoff_soft: 0, dash_solid: 1,
  mod_jitter: 0, mod_flat: 0, mod_breaks: 0, mod_wobble: 20, mod_wobble_size: 12, mod_wave: 0,
  mod_wave_size: 16, mod_wave_zone: 0, mod_wave_even: 0, mod_smooth: 6,
  input_flat: 0.25, input_gain: 3, input_dark_gamma: 0.7, input_light_gain: 1.5,
  input_light_floor: 0.06, input_blur: 1.8, input_broad_blur: 11.5, input_broad: 0.5,
  input_height: 0,
  sun_azimuth: 315, sun_altitude: 45, sun_exag: 4,
  engine_step: 1, engine_min: 4, engine_stop: 0.5, engine_fill: 0.8, engine_stripes: 0,
  batch_floor: 0, batch_stroke: 1.2, batch_seed: 1,
  water_start: 2, water_reach: 60, water_space_reach: 120, water_fade: 1, water_size: 14,
  water_patches: 0, water_fray: 0, water_depth_fray: 0,
};

// The hills: contours, lit and shaded, then a second angle over the steeper
// ground to deepen the relief. `batch_floor` keeps the crosshatch off the flats.
burin.batch({ angle: 0, style: { ...style, space_light: 8.6, space_dark: 2.9, space_steep: 0.3, dash_light: 0.1 } });
burin.batch({ angle: 45, style: { ...style, batch_floor: 0.6, dash_light: 0.6, space_light: 13.5, space_dark: 4.5 } });

// The coast: two concentric lines, then the distance contours just off it,
// out to 20 px. `water_start` leaves a clean gap between the two.
burin.shore({
  lines: 2,
  style: { ...style, space_light: 1.5, space_dark: 3, dash_light: 1, mod_wobble: 0, mod_smooth: 0 },
});
burin.water("lines", {
  follow: "shore",
  angle: 0,
  style: {
    ...style,
    space_light: 5.4,
    space_dark: 2.4,
    dash_light: 0.25,
    mod_flat: 0.6,
    mod_wobble: 0,
    mod_smooth: 3,
    water_start: 3,
    water_reach: 20,
  },
});

// The open sea: one direction over the whole map, waving together. This is the
// wavy modifier ('mod_wave') with `mod_wave_even`, so no two lines cross and the
// rows stay evenly spaced; `mod_wobble` lets the flow turn.
burin.water("lines", {
  follow: "map",
  angle: 90,
  style: {
    ...style,
    space_light: 4,
    space_dark: 4,
    water_start: 26,
    water_reach: 4000,
    water_fade: 0.2,
    water_fray: 40,
    dash_light: 0.95,
    dash_dark: 1,
    dash_period: 36,
    mod_jitter: 0.3,
    mod_flat: 0.6,
    mod_wobble: 45,
    mod_wobble_size: 120,
    mod_smooth: 3,
    mod_wave: 2.5,
    mod_wave_size: 30,
    mod_wave_even: 1,
  },
});

burin.draw();
