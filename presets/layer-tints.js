/**
 * Layer tints — bands of colour by height and depth, as on atlas maps.
 *
 * Fills first, each kept to a band of heights or depths; then the contours
 * over them. The depth bands are written highest first, so each darker tint
 * sits over the paler one.
 */

burin.area({ at: "37.6,-122.85", size_km: [90, 63] });

const smooth = { input_blur: 5 };

// The sea, pale to blue with depth.
burin.fill({ ground: "sea", color: "#d6e6ef", style: {} });
burin.fill({ ground: "sea", color: "#bcd4e3", where: { depthMin: 100 }, style: smooth });
burin.fill({ ground: "sea", color: "#a0bfd5", where: { depthMin: 1000 }, style: smooth });
burin.fill({ ground: "sea", color: "#83a8c4", where: { depthMin: 2500 }, style: smooth });

// The land, pale green to brown with height.
burin.fill({ color: "#eef0df", style: {} });
burin.fill({ color: "#e0e2c2", where: { elevMin: 100 }, style: smooth });
burin.fill({ color: "#d8cfa4", where: { elevMin: 300 }, style: smooth });
burin.fill({ color: "#cbb589", where: { elevMin: 600 }, style: smooth });

// Contours over the tints, and a coloured coast.
burin.batch({ angle: 0, color: "#5c4630", style: { dash_light: 0.5 } });
burin.batch({ angle: 0, ground: "sea", color: "#2f5573", style: { engine_stripes: 1, space: 5, space_light: 3, space_dark: 0.9, space_steep: 0.6, dash_light: 1, dash_dark: 1, dash_cutoff: 0, mod_breaks: 0.02, mod_smooth: 8 } });
burin.shore({ color: "#1d3a52" });

burin.draw();
