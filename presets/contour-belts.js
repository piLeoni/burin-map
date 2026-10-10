/**
 * Contour belts — a loop in a real language.
 *
 * A coloured belt for every 100 m of height and depth: one fill per band,
 * each between its two limits. This is the loop a flat format could not
 * hold, and the reason a document is code.
 */

burin.area({ at: "37.66,-122.45", size_km: [36, 25] });

// The sea floor, pale to blue, deepening every 100 m.
const sea = ["#d6e6ef", "#a9c6db", "#7ba3c0", "#4d7fa2", "#2b5c80", "#153b5c"];
for (let i = 0; i < sea.length; i++)
  burin.fill({ ground: "sea", color: sea[i], where: { depthMin: i * 100 }, style: { input_blur: 5 } });

// The land, pale green to brown, rising every 100 m.
const land = ["#eef0df", "#dfe3c2", "#d3d2a4", "#cbb589", "#bd9c6e", "#a67f52", "#8a6440"];
for (let i = 0; i < land.length; i++)
  burin.fill({ color: land[i], where: { elevMin: i * 100 }, style: { input_blur: 5 } });

// Contours over the belts, coloured by height, and the coast.
burin.batch({ angle: 0, shade: burin.lerp("#6a5230", "#241206", burin.clamp(burin.tone, 0, 1)), style: { dash_light: 0.5 } });
burin.batch({ angle: 0, ground: "sea", shade: burin.lerp("#4a6f88", "#0b2236", burin.tone), style: { space_light: 5, space_dark: 3.5, engine_stripes: 1 } });
burin.shore({ color: "#1d3a52" });

burin.draw();
