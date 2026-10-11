/**
 * Contour belts — a loop in a real language.
 *
 * A coloured belt for every 100 m of height and depth: one fill per band,
 * each between its two limits. This is the loop a flat format could not
 * hold, and the reason a document is code.
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

// The sea floor, pale to blue, deepening every 100 m.
const sea = ["#d6e6ef", "#a9c6db", "#7ba3c0", "#4d7fa2", "#2b5c80", "#153b5c"];
for (let i = 0; i < sea.length; i++)
  burin.fill({ ground: "sea", color: sea[i], where: { depthMin: i * 100 }, style: { ...style, input_blur: 5 } });

// The land, pale green to brown, rising every 100 m.
const land = ["#eef0df", "#dfe3c2", "#d3d2a4", "#cbb589", "#bd9c6e", "#a67f52", "#8a6440"];
for (let i = 0; i < land.length; i++)
  burin.fill({ color: land[i], where: { elevMin: i * 100 }, style: { ...style, input_blur: 5 } });

// Contours over the belts, coloured by height, and the coast.
burin.batch({
  angle: 0,
  shade: burin.lerp("#6a5230", "#241206", burin.clamp(burin.tone, 0, 1)),
  style: { ...style, dash_light: 0.5 },
});
burin.batch({
  angle: 0,
  ground: "sea",
  shade: burin.lerp("#4a6f88", "#0b2236", burin.tone),
  style: { ...style, space_light: 5, space_dark: 3.5, engine_stripes: 1 },
});
burin.shore({
  color: "#1d3a52",
  style: { ...style, space_light: 3, space_dark: 3, dash_light: 1, mod_wobble: 0, mod_smooth: 0 },
});

burin.draw();
