/**
 * Sea depth — the seabed in contour lines, and nothing on land.
 *
 * A `ground: "sea"` batch draws only below the waterline, and the shoreline
 * closes it. The heights come out as continuous contours that read even on a
 * featureless seabed.
 */

burin.area({ at: "37.6,-122.85", size_km: [90, 63] });

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

burin.batch({
  angle: 0,
  ground: "sea",
  style: {
    ...style,
    engine_stripes: 1,
    space_light: 15,
    space_dark: 4.5,
    space_steep: 0.6,
    dash_light: 1,
    dash_dark: 1,
    mod_breaks: 0.02,
    mod_smooth: 8,
    input_blur: 8,
    input_broad: 0.8,
  },
});

burin.shore({ style: { ...style, space_light: 3, space_dark: 3, dash_light: 1, mod_wobble: 0, mod_smooth: 0 } });

burin.draw();
