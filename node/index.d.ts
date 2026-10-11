// The burin-map package: the `burin` a document script calls, plus what is
// around it. A document written for the web page or `--script` runs here as is:
//
//   const burin = require('burin-map')
//   burin.area({ at: '37.66,-122.45', size_km: [36, 25] })
//   burin.batch({ angle: 0, style: { space_light: 6 } })
//   const map = burin.draw()
//   fs.writeFileSync('map.svg', map.svg())

import type { Drawing } from './native'

export { Drawing } from './native'

/** A point's fields: height in metres, the shading tone, the distance from
 * the shore, and the water depth. */
export type Field = 'elev' | 'tone' | 'dist' | 'depth'

/** A number, or a tree built from the fields. */
export type Scalar = number | Field | any[]

/** A `#rgb` / `#rrggbb` colour, or a tree built from the fields. */
export type Color = string | any[]

/** The style parameters, by name (long or short). */
export interface Style {
  [parameter: string]: number
}

/** Where to draw, and on what paper. */
export interface AreaOptions {
  /** a named area: `bern`, `matterhorn`, `san-francisco` */
  place?: string
  /** the centre: an address or place name, or `"lat,lon"` */
  at?: string
  /** `[west, south, east, north]` in degrees */
  bounds?: [number, number, number, number]
  /** ground size around `at`, km: `[side]` or `[width, height]` */
  size_km?: number[]
  /** paper size, mm: `[side]` or `[width, height]` */
  size_mm?: number[]
  /** the map scale's `N` in `1:N` */
  scale?: number
  /** pen width on paper, mm */
  pen_mm?: number
}

/** The heights or depths a fill keeps to, in metres. */
export interface Where {
  elevMin?: number
  elevMax?: number
  depthMin?: number
  depthMax?: number
}

/** A batch: a run of lines, stippling, or a flat fill. */
export interface BatchOptions {
  /** a label, shown in reports and as its layer */
  name?: string
  /** 0 follows the contours, 90 runs straight downhill */
  angle?: number
  /** lines (default), dots, or fill */
  draw?: 'lines' | 'dots' | 'fill'
  /** land (default) or sea: sea draws the seabed like land */
  ground?: 'land' | 'sea'
  color?: string
  /** colour from a tree, read at each dash */
  shade?: Color
  /** width as a multiple of `batch_stroke`, a number or a tree */
  width?: Scalar
  cap?: 'round' | 'round-inside' | 'butt'
  /** for a fill: the band it keeps to */
  where?: Where
  style?: Style
  /** a style parameter given by name, outside `style` */
  [parameter: string]: any
}

/** A pass over the water. */
export interface WaterOptions {
  name?: string
  /** shore (default) or map: what the lines follow */
  follow?: 'shore' | 'map'
  angle?: number
  color?: string
  shade?: Color
  width?: Scalar
  cap?: 'round' | 'round-inside' | 'butt'
  where?: { depthMin?: number; depthMax?: number; depthFade?: number }
  style?: Style
  [parameter: string]: any
}

/** The shoreline, the zero contour. */
export interface ShoreOptions {
  /** how many lines follow the coast; 1 is the coast itself */
  lines?: number
  color?: string
  shade?: Color
  width?: Scalar
  cap?: 'round' | 'round-inside' | 'butt'
  style?: Style
  [parameter: string]: any
}

/** A style parameter and its help. */
export interface Parameter {
  name: string
  short: string
  help: string
  group: string
}

/** What a document builds: the atlas the engine draws, and its area. */
export interface Built {
  atlas: object[]
  area: AreaOptions | null
}

// ---- the document, as a script writes it ----------------------------------

/** Where to draw. */
export declare function area(options: AreaOptions): AreaOptions
/** A batch of lines, stippling, or a fill. */
export declare function batch(options?: BatchOptions): object
/** A fill of a band: what a contour belt is. */
export declare function fill(options?: BatchOptions): object
/** A pass over the water: `water('lines' | 'ripples' | 'dots', {…})`. */
export declare function water(kind?: 'lines' | 'ripples' | 'dots' | WaterOptions, options?: WaterOptions): object
/** The shoreline, or its own style. */
export declare function shore(options?: ShoreOptions): object
/** Draw what has been built (downloading elevation and water), and start a
 * new, empty document. */
export declare function draw(): Drawing
/** The atlas built so far, without drawing it. */
export declare function json(): object[]
/** The style parameters, by name, with their help. */
export declare function parameters(): Parameter[]

export declare const elev: Field
export declare const tone: Field
export declare const dist: Field
export declare const depth: Field

/** A colour between two others, by `t`. */
export declare function lerp(a: Color, b: Color, t: Scalar): Color
/** An rgb colour from three trees. */
export declare function rgb(r: Scalar, g: Scalar, b: Scalar): Color
export declare function clamp(x: Scalar, lo: Scalar, hi: Scalar): Scalar
export declare function smoothstep(edge0: Scalar, edge1: Scalar, x: Scalar): Scalar
export declare function step(edge: Scalar, x: Scalar): Scalar
export declare function abs(x: Scalar): Scalar
export declare function floor(x: Scalar): Scalar
export declare function ceil(x: Scalar): Scalar
export declare function round(x: Scalar): Scalar
export declare function min(a: Scalar, b: Scalar): Scalar
export declare function max(a: Scalar, b: Scalar): Scalar
/** Two colours mixed, `t` decided once. */
export declare function mix(a: Color, b: Color, t: number): Color

// ---- documents as files, and everything around them -------------------------

/** Run a document script (its source) and draw it. */
export declare function drawScript(source: string): Drawing
/** Run a document script and return what it built. */
export declare function evaluate(source: string): Built
/** Draw any elevation grid (metres, row 0 = north). Values ≤ 0.5 m are sea. */
export declare function drawElevation(elev: Float32Array, width: number, height: number, pxM: number, style?: Style | null): Drawing
/** `[lat, lon, label]` for coordinates or a place name (Nominatim). */
export declare function locate(query: string): [number, number, string]
/** Lon/lat box of roughly `widthKm` × `heightKm` centred on `(lat, lon)`. */
export declare function boundsAround(lat: number, lon: number, widthKm: number, heightKm: number): [number, number, number, number]
export declare function projectedSize(west: number, south: number, east: number, north: number): [number, number]
/** `[name, default, help, group]` for every style parameter. */
export declare function styleParams(): Array<[string, number, string, string]>
/** `[id, blurb]` for each style group. */
export declare function styleGroups(): Array<[string, string]>
/** Where the fields are built: "GPU: Apple M4 (Metal)", "CPU (forced)", … */
export declare function engine(): string
/** Never build the fields on the GPU: reproducible output. */
export declare function forceCpu(): void
/** Run the `burin-map` CLI (`argv[0]` is the program name). */
export declare function runCli(argv: string[]): void
export declare function version(): string
