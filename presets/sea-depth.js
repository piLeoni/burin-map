/**
 * Sea depth — the seabed in contour lines, and nothing on land.
 *
 * A `ground: "sea"` batch draws only below the waterline, and the shoreline
 * closes it. The heights come out as continuous contours that read even on a
 * featureless seabed.
 */

burin.area({ at: "37.6,-122.85", size_km: [90, 63] });

burin.batch({
  angle: 0,
  ground: "sea",
  style: {
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

burin.shore();

burin.draw();
