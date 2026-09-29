/**
 * Parameter short names, mirroring the `style!` table in
 * `crates/burin-map-core/src/style.rs`.
 *
 * These are the same abbreviations the CLI accepts (`--sl`, `--dl`, …), so the
 * compact URL notation reads like a command line.
 *
 * Kept in sync by `npm run test:render`, which compares this table against
 * `style_params()` from the WASM build and fails on any drift.
 */

/** full_name → short (e.g. `space_light` → `sl`). */
export const PARAM_SHORTS = {
  space: 's',
  space_light: 'sl',
  space_dark: 'sd',
  space_steep: 'ss',
  space_steep_gamma: 'ssg',
  dash_light: 'dl',
  dash_dark: 'dd',
  dash_period: 'dp',
  dash_cutoff: 'dc',
  dash_drop: 'ddrop',
  mod_jitter: 'mj',
  mod_breaks: 'mb',
  mod_wobble: 'mw',
  mod_wobble_size: 'mws',
  mod_smooth: 'ms',
  input_flat: 'if',
  input_gain: 'ig',
  input_dark_gamma: 'idg',
  input_light_gain: 'ilg',
  input_light_floor: 'ilf',
  input_blur: 'ib',
  input_broad_blur: 'ibb',
  input_broad: 'ibr',
  sun_azimuth: 'sa',
  sun_altitude: 'salt',
  sun_exag: 'sx',
  engine_step: 'es',
  engine_min: 'em',
  engine_stop: 'esp',
  engine_fill: 'ef',
  batch_floor: 'bf',
  batch_stroke: 'bs',
  batch_seed: 'bseed',
  water_start: 'wst',
  water_reach: 'wr',
  water_fade: 'wf',
}

/** short → full_name. */
export const SHORT_PARAMS = Object.fromEntries(
  Object.entries(PARAM_SHORTS).map(([full, short]) => [short, full]),
)

/**
 * Short names that are a prefix of another short name. The notation parser
 * resolves these by matching the longest name first, so `salt45` is the sun
 * altitude while `sa315` is the azimuth.
 */
export const AMBIGUOUS_SHORTS = Object.keys(SHORT_PARAMS).filter((a) =>
  Object.keys(SHORT_PARAMS).some((b) => b !== a && b.startsWith(a)),
)
