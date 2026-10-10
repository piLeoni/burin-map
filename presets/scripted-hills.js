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
burin.batch({ angle: 0, ground: "sea", style: { space_light: 5, space_dark: 5, water_reach: 4000 } });

burin.draw();
