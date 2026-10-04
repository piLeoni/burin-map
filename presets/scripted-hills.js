/**
 * Scripted hills — a loop over angles.
 *
 * Crosshatching written as a loop: each pass adds another angle to the
 * hatched set, so four directions build up a rounded hill. The spacing of a
 * pass tightens where the ground is steep, so the summits go dark.
 */

burin.area({ at: "37.66,-122.45", size_km: [36, 25] });

for (const angle of [0, 45, 90, 135])
  burin.batch({
    angle,
    style: {
      space: 6,
      space_light: 2.5,
      space_dark: 0.7,
      space_steep: 0.25,
      dash_light: 0.6,
      dash_dark: 1,
      dash_cutoff: 0,
      dash_drop: 0,
      mod_breaks: 0.03,
      mod_smooth: 8,
    },
  });

// The sea: one even hatch, nothing on land.
burin.batch({ angle: 0, ground: "sea", style: { space: 5, space_light: 1, space_dark: 1, water_reach: 4000 } });

burin.draw();
