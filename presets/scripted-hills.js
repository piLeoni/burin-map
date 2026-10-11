/**
 * Scripted hills — a loop over angles.
 *
 * Crosshatching written as a loop: each pass adds another angle to the
 * hatched set, so four directions build up a rounded hill. The spacing of a
 * pass tightens where the ground is steep, so the summits go dark.
 */

burin.area({ at: "37.66,-122.45", size_km: [36, 25] });

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

for (const angle of [0, 45, 90, 135])
  burin.batch({
    angle,
    style: {
      ...style,
      space_light: 15,
      space_dark: 4.2,
      space_steep: 0.25,
      dash_light: 0.6,
      dash_dark: 1,
      mod_breaks: 0.03,
      mod_smooth: 8,
    },
  });

// The sea: one even hatch, nothing on land.
burin.batch({ angle: 0, ground: "sea", style: { ...style, space_light: 5, space_dark: 5, water_reach: 4000 } });

burin.draw();
