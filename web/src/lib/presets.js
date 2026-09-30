const PENINSULA = { at: '37.66,-122.45', size_km: [36, 25] }

/** Styles offered by the Load menu, each with the area it was made for. */
export const PRESETS = [
  {
    id: 'contours',
    label: 'Contours',
    blurb: 'contour lines, lit and shaded',
    doc: { area: PENINSULA, batches: [{ angle: 0, style: {} }] },
  },
  {
    id: 'downhill',
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
    label: 'Engraved sea',
    blurb: 'crosshatched hills, coast lines, then a sea of evenly waving currents',
    doc: {
      area: { at: '37.7,-122.62', size_km: [40, 28] },
      batches: [
        { angle: 0, style: { space_light: 1.8, space_dark: 0.6, space_steep: 0.3, dash_light: 0.1 } },
        { angle: 90, style: { batch_floor: 0.55, dash_light: 0.6, space: 5 } },
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
    label: 'Sea depth',
    blurb: 'the seabed in contour lines, and nothing on land',
    doc: {
      area: { at: '37.6,-122.85', size_km: [90, 63] },
      batches: [
        {
          angle: 0,
          ground: 'sea',
          style: {
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
          },
        },
      ],
      water: { shoreline: { lines: 1 }, batches: [] },
    },
  },
]
