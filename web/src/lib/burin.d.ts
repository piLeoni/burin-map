/**
 * The `burin` object a document script sees, as this page's editor offers it.
 *
 * A document builds the list of batches and water passes the engine draws.
 * The same format draw.land reads through `burin.terrain`, so a style written
 * here draws the same there.
 */

/** A point's fields: height in metres, the shading tone, the distance inland,
 * and the water depth. */
type Field = 'elev' | 'tone' | 'dist' | 'depth'

/** A scalar: a number, or a tree built from the fields. */
type Scalar = number | any[]

/** A colour: a `#rgb`/`#rrggbb` string, or a tree built from the fields. */
type Color = string | any[]

/** The heights or depths a fill keeps to, in metres. */
interface Where {
  /** lowest height of the band, metres */
  elevMin?: number
  /** highest height of the band, metres */
  elevMax?: number
  /** shallowest depth of the band, metres */
  depthMin?: number
  /** deepest depth of the band, metres */
  depthMax?: number
}

/** The water's own band, with a fade. */
interface WaterWhere {
  depthMin?: number
  depthMax?: number
  depthFade?: number
}

/** The style parameters, by name; see the YAML view's `# default` notes. */
interface Style {
  [parameter: string]: number
}

/** A batch: a run of lines, stippling, or a flat fill over what is below it. */
interface BatchOptions {
  /** a label, shown in the app's layer list */
  name?: string
  /** 0 follows the contours, 90 runs straight downhill */
  angle?: number
  /** lines (default), dots (stippling), or fill (a flat area) */
  draw?: 'lines' | 'dots' | 'fill'
  /** land (default) or sea: sea draws the seabed like land */
  ground?: 'land' | 'sea'
  /** fill colour as `#rgb` or `#rrggbb` */
  color?: string
  /** colour from a tree, for lines and dots */
  shade?: Color
  /** line width as a multiple of `batch_stroke`, a number or a tree */
  width?: Scalar
  /** for a fill: the band of heights or depths it keeps to */
  where?: Where
  /** the style parameters, by name */
  style?: Style
  [parameter: string]: any
}

/** A pass over the water. */
interface WaterOptions {
  name?: string
  /** shore (default) or map: what the lines follow */
  follow?: 'shore' | 'map'
  /** lines (default), ripples, or dots */
  draw?: 'lines' | 'ripples' | 'dots'
  angle?: number
  color?: string
  shade?: Color
  width?: Scalar
  where?: WaterWhere
  style?: Style
  [parameter: string]: any
}

/** The shoreline, the zero contour. */
interface ShoreOptions {
  /** how many parallel lines along the shore */
  lines?: number
  color?: string
  shade?: Color
  width?: Scalar
  style?: Style
  [parameter: string]: any
}

/** A parameter's help, as the app lists it. */
interface Parameter {
  name: string
  default: number
  help: string
  group: string
}

/** The document under construction, in draw order. */
interface Atlas {
  /** add a batch: what `burin.batch({…})` returns */
  batch(entry: string): Atlas
  /** add a pass over the water: what `burin.water(…)` returns */
  water(entry: string): Atlas
  /** add the shoreline, or give it its own style */
  shore(entry?: string): Atlas
  /** hand the document to the engine: the drawn map */
  draw(): string
  /** the document as JSON, without drawing it */
  json(): string
}

interface Burin {
  /** start a document */
  atlas(): Atlas
  /** a batch of lines, stippling, or a fill */
  batch(options?: BatchOptions): string
  /** a fill of a band: what a contour belt is */
  fill(options?: BatchOptions): string
  /** a pass over the water */
  water(kindOrOptions?: 'lines' | 'ripples' | 'dots' | WaterOptions, options?: WaterOptions): string
  /** the shoreline, the zero contour */
  shore(options?: ShoreOptions): string
  /** the style parameters, by name, with their help */
  parameters(): Parameter[]

  /** height in metres */
  elev: Field
  /** the shading tone, 0 to 1 */
  tone: Field
  /** distance inland, in cells */
  dist: Field
  /** water depth, in metres */
  depth: Field

  /** a colour between two others, by `t` */
  lerp(a: Color, b: Color, t: Scalar): Color
  /** an rgb colour from three trees */
  rgb(r: Scalar, g: Scalar, b: Scalar): Color
  /** `x` kept between `lo` and `hi` */
  clamp(x: Scalar, lo: number, hi: number): Scalar
  /** a smooth step between two edges */
  smoothstep(edge0: number, edge1: number, x: Scalar): Scalar
  /** 0 below `edge`, 1 above */
  step(edge: number, x: Scalar): Scalar
  abs(x: Scalar): Scalar
  floor(x: Scalar): Scalar
  ceil(x: Scalar): Scalar
  round(x: Scalar): Scalar
  min(a: Scalar, b: Scalar): Scalar
  max(a: Scalar, b: Scalar): Scalar
  /** `t` as a plain fraction, so a colour can be made from it */
  mix(a: Color, b: Color, t: Scalar): Color
}

declare const burin: Burin
