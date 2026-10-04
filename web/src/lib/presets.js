const PENINSULA = { at: '37.66,-122.45', size_km: [36, 25] }
const OFFSHORE = { at: '37.6,-122.85', size_km: [90, 63] }

/** Continuous contours, sparse on flat ground: they read even on a featureless seabed.
 * Solid lines show where stripes meet, so they are traced in one piece. */
const SEABED = {
  engine_stripes: 1,
  space: 5,
  space_light: 3,
  space_dark: 0.9,
  space_steep: 0.6,
  dash_light: 1,
  dash_dark: 1,
  dash_cutoff: 0,
  dash_drop: 0,
  mod_breaks: 0.02,
  mod_smooth: 8,
  input_blur: 8,
  input_broad: 0.8,
}

/** Lines evenly spaced and solid: neither the sun nor the slope changes them. */
const EVEN = { space_light: 1, space_dark: 1, space_steep: 0, dash_light: 1, dash_dark: 1, dash_cutoff: 0, dash_drop: 0 }

/** Straight, even, solid hatching over the whole sea. */
const HATCH = {
  space: 5,
  space_light: 1,
  space_dark: 1,
  water_start: 2,
  water_reach: 4000,
  dash_light: 1,
  dash_dark: 1,
  dash_drop: 0,
  mod_breaks: 0,
  mod_jitter: 0,
  mod_smooth: 0,
}

/** A fill band's edges smoothed past the steps of coarse elevation tiles. */
const SMOOTH_BAND = { input_blur: 5 }

/**
 * Styles offered by the Load menu, each with the area it was made for.
 * `screen` is the drawing time in the browser: fast under 0.4 s, medium under 1 s.
 * `plot` is a pen plotter at A3 (30 mm/s down, 100 mm/s up, 0.25 s a lift):
 * quick under 1 h, medium under 1.5 h, slow under 2 h, very slow beyond.
 */
export const PRESETS = [
  {
    id: 'contours',
    screen: 'fast',
    plot: 'quick',
    label: 'Contours',
    blurb: 'contour lines, lit and shaded',
    doc: { area: PENINSULA, batches: [{ angle: 0, style: {} }] },
  },
  {
    id: 'downhill',
    screen: 'medium',
    plot: 'medium',
    label: 'Downhill',
    blurb: 'lines running down every slope, denser in shadow; a stippled sea',
    doc: {
      area: PENINSULA,
      batches: [
        {
          angle: 90,
          style: {
            space: 3,
            space_light: 3,
            space_dark: 0.8,
            space_steep: 0.15,
            dash_light: 1,
            dash_dark: 1,
            dash_cutoff: 0,
            dash_drop: 0,
            mod_breaks: 0.03,
            mod_smooth: 8,
          },
        },
      ],
      water: {
        shoreline: { lines: 2, style: { space: 2 } },
        batches: [
          {
            follow: 'shore',
            draw: 'dots',
            angle: 0,
            style: { space: 10, space_light: 2.5, space_dark: 0.3, water_start: 3 },
          },
        ],
      },
    },
  },
  {
    id: 'engraved-sea',
    screen: 'medium',
    plot: 'slow',
    label: 'Engraved sea',
    blurb: 'crosshatched hills, coast lines, then a sea of evenly waving currents',
    doc: {
      area: { at: '37.7,-122.62', size_km: [40, 28] },
      batches: [
        { angle: 0, style: { space_light: 1.8, space_dark: 0.6, space_steep: 0.3, dash_light: 0.1 } },
        { angle: 45, style: { batch_floor: 0.6, dash_light: 0.6, space: 7.5 } },
      ],
      water: {
        shoreline: { lines: 2, style: { space: 1.5 } },
        batches: [
          { follow: 'shore', angle: 0, style: { space: 3, water_start: 3, water_reach: 20 } },
          {
            follow: 'map',
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
          },
        ],
      },
    },
  },
  {
    id: 'sea-depth',
    screen: 'fast',
    plot: 'quick',
    label: 'Sea depth',
    blurb: 'the seabed in contour lines, and nothing on land',
    doc: {
      area: OFFSHORE,
      batches: [{ angle: 0, ground: 'sea', style: SEABED }],
      water: { shoreline: { lines: 1 }, batches: [] },
    },
  },
  {
    id: 'stipple',
    screen: 'medium',
    plot: 'very slow',
    label: 'Stipple',
    blurb: 'hills in dots, dense in shadow; a stippled sea',
    doc: {
      area: PENINSULA,
      batches: [
        {
          angle: 0,
          draw: 'dots',
          style: { space: 3, space_light: 2.2, space_dark: 0.5, space_steep: 0.3, dash_cutoff: 0 },
        },
      ],
      water: {
        shoreline: { lines: 1 },
        batches: [
          {
            follow: 'shore',
            draw: 'dots',
            angle: 0,
            style: { space: 10, space_light: 2.5, space_dark: 0.3, water_start: 3, water_reach: 150 },
          },
        ],
      },
    },
  },
  {
    id: 'land-and-sea',
    screen: 'fast',
    plot: 'medium',
    label: 'Land and sea',
    blurb: 'one relief across the coast: hills up, the seabed down, darker the further from the shore',
    doc: {
      area: OFFSHORE,
      batches: [
        { angle: 0, style: { input_height: 0.4 } },
        { angle: 0, ground: 'sea', style: { ...SEABED, input_height: -0.5 } },
      ],
      water: { shoreline: { lines: 1 }, batches: [] },
    },
  },
  {
    id: 'colour-relief',
    screen: 'fast',
    plot: 'slow',
    label: 'Colour relief',
    blurb: 'contours coloured by height, green to brown, and by depth, pale to deep blue; for screen and print, every line has its own colour',
    doc: {
      area: OFFSHORE,
      batches: [
        {
          angle: 0,
          color_expr: 'lerp(lerp(#4d7a3a, #b39245, clamp(elev / 250, 0, 1)), #6e3d22, clamp((elev - 250) / 450, 0, 1))',
          style: { dash_light: 0.5 },
        },
        {
          angle: 0,
          ground: 'sea',
          color_expr: 'lerp(#8cc8de, #0d2c57, clamp((0 - elev) / 2000, 0, 1))',
          style: SEABED,
        },
      ],
      water: { shoreline: { lines: 1, color: '#0d2c57' }, batches: [] },
    },
  },
  {
    id: 'height-size',
    screen: 'fast',
    plot: 'very slow',
    label: 'Height map · size',
    blurb: 'height sets the size, not the sun: contours thicken up the hills and down into the deep; every width its own pen, so add width_steps to plot it',
    doc: {
      area: OFFSHORE,
      batches: [
        {
          angle: 0,
          width_expr: 'lerp(0.4, 3, clamp(elev / 500, 0, 1))',
          style: { ...EVEN, space: 6, mod_breaks: 0.02 },
        },
        {
          angle: 0,
          ground: 'sea',
          width_expr: 'lerp(0.3, 3, clamp(-elev / 3000, 0, 1))',
          style: { ...SEABED, ...EVEN, space: 6 },
        },
      ],
      water: { shoreline: { lines: 1 }, batches: [] },
    },
  },
  {
    id: 'height-density',
    screen: 'fast',
    plot: 'slow',
    label: 'Height map · density',
    blurb: 'height sets how much ink, not the sun: dots crowd the summits, the hatching crosses again as the sea deepens',
    doc: {
      area: OFFSHORE,
      batches: [
        {
          angle: 0,
          draw: 'dots',
          style: { space: 3, space_light: 3.5, space_dark: 0.35, space_steep: 0, dash_cutoff: 0, input_height: 1 },
        },
      ],
      water: {
        shoreline: { lines: 1 },
        batches: [
          { follow: 'map', angle: 45, style: { ...HATCH, space: 7 } },
          { follow: 'map', angle: -45, where: { depth_min: 150, depth_fade: 100 }, style: { ...HATCH, space: 7 } },
          { follow: 'map', angle: 0, where: { depth_min: 1500, depth_fade: 400 }, style: { ...HATCH, space: 7 } },
        ],
      },
    },
  },
  {
    id: 'hatched-sea',
    screen: 'fast',
    plot: 'very slow',
    label: 'Hatched sea',
    blurb: 'dots grow up the hills; straight hatching thickens as the sea deepens, as on old charts; every width its own pen, so add width_steps to plot it',
    doc: {
      area: OFFSHORE,
      batches: [
        {
          angle: 0,
          draw: 'dots',
          width_expr: 'lerp(0.8, 3, clamp(elev / 500, 0, 1))',
          style: { space: 4, space_light: 1, space_dark: 1, space_steep: 0, dash_cutoff: 0 },
        },
      ],
      water: {
        shoreline: { lines: 1 },
        batches: [
          { follow: 'map', angle: 45, width_expr: 'lerp(0.25, 2.5, clamp(depth / 3000, 0, 1))', style: HATCH },
        ],
      },
    },
  },
  {
    id: 'layer-tints',
    screen: 'fast',
    plot: 'slow',
    label: 'Layer tints',
    blurb: 'bands of colour by height and depth under coloured contours, as on atlas maps; the tints are fills for screen and print, the plotter draws the lines',
    doc: {
      area: OFFSHORE,
      batches: [
        { draw: 'fill', ground: 'sea', color: '#d6e6ef', angle: 0, style: {} },
        { draw: 'fill', ground: 'sea', color: '#bcd4e3', where: { depth_min: 100 }, angle: 0, style: SMOOTH_BAND },
        { draw: 'fill', ground: 'sea', color: '#a0bfd5', where: { depth_min: 1000 }, angle: 0, style: SMOOTH_BAND },
        { draw: 'fill', ground: 'sea', color: '#83a8c4', where: { depth_min: 2500 }, angle: 0, style: SMOOTH_BAND },
        { draw: 'fill', color: '#eef0df', angle: 0, style: {} },
        { draw: 'fill', color: '#e0e2c2', where: { elev_min: 100 }, angle: 0, style: SMOOTH_BAND },
        { draw: 'fill', color: '#d8cfa4', where: { elev_min: 300 }, angle: 0, style: SMOOTH_BAND },
        { draw: 'fill', color: '#cbb589', where: { elev_min: 600 }, angle: 0, style: SMOOTH_BAND },
        { angle: 0, color: '#5c4630', style: { dash_light: 0.5 } },
        { angle: 0, ground: 'sea', color: '#2f5573', style: SEABED },
      ],
      water: { shoreline: { lines: 1, color: '#1d3a52' }, batches: [] },
    },
  },
  {
    id: 'contour-belts',
    screen: 'fast',
    plot: 'slow',
    label: 'Contour belts',
    blurb: 'a loop in JavaScript: a coloured belt for every 100 m of height and depth — the JS view shows the script',
    script: `// A loop makes a belt for every step of height and depth: one batch per
// band, each filled between its two limits. Open the YAML view to see it
// written out flat; edit the numbers here to change the steps.
const doc = burin.atlas()

// the sea floor, pale to blue, deepening every 100 m
const sea = ["#d6e6ef", "#a9c6db", "#7ba3c0", "#4d7fa2", "#2b5c80", "#153b5c"]
for (let i = 0; i < sea.length; i++)
  doc.batch(burin.fill({ ground: "sea", color: sea[i], where: { depthMin: i * 100 }, angle: 0, style: { input_blur: 5 } }))

// the land, pale green to brown, rising every 100 m
const land = ["#eef0df", "#dfe3c2", "#d3d2a4", "#cbb589", "#bd9c6e", "#a67f52", "#8a6440"]
for (let i = 0; i < land.length; i++)
  doc.batch(burin.fill({ color: land[i], where: { elevMin: i * 100 }, angle: 0, style: { input_blur: 5 } }))

// contours over the belts, coloured by height, and the coast
doc.batch(burin.batch({ angle: 0, shade: burin.lerp("#6a5230", "#241206", burin.clamp(burin.tone, 0, 1)), style: { dash_light: 0.5 } }))
doc.batch(burin.batch({ angle: 0, ground: "sea", shade: burin.lerp("#4a6f88", "#0b2236", burin.tone), style: { space: 5, engine_stripes: 1 } }))
doc.shore(burin.shore({ color: "#1d3a52" }))

doc.draw()`,
  },
  {
    id: 'scripted-hills',
    screen: 'fast',
    plot: 'very slow',
    label: 'Scripted hills',
    blurb: 'a loop over angles: crosshatching that tightens with the slope, drawn by a script you can tune',
    script: `// Crosshatching written as a loop: each pass adds another angle to the
// hatched set, so four lines build up a rounded hill. The spacing of a
// pass tightens where the ground is steep, so the summits go dark.
const doc = burin.atlas()

for (const angle of [0, 45, 90, 135])
  doc.batch(burin.batch({
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
  }))

// the sea: one even hatch, nothing on land
doc.batch(burin.batch({ angle: 0, ground: "sea", style: { space: 5, space_light: 1, space_dark: 1, water_reach: 4000 } }))

doc.draw()`,
  },
]
