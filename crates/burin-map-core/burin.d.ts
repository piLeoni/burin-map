// burin-map: pen-and-ink terrain, typed for scripts.
//
// A script lays out batches and water passes, then draws. Everything is a
// number, a field the map provides, a colour, or a small tree of those:
//
//   const atlas = burin.atlas()
//   atlas.batch(
//     burin.batch({ name: 'hills' })
//       .shade(burin.lerp('#e8dcc8', '#2a1810', burin.tone))
//       .width(burin.lerp(0.5, 2, burin.tone)),
//   )
//   atlas.water(burin.water.lines({ color: '#33556b' }))
//   atlas.shore(burin.shore({ lines: 3, width: burin.lerp(1, 0.2, burin.dist) }))
//   const drawing = atlas.draw({ elev, width, height, pxM: 20 })
//
// The colour and width may be fixed values or trees read at each dash.

/** What a value is read from, where a dash sits. */
export type Field = 'elev' | 'tone' | 'dist' | 'depth'

/** A number, or a small expression evaluated at each dash. */
export type Scalar = number | Field | ScalarTree

/** An expression: `[op, a, b]`. Numbers and fields nest freely. */
export type ScalarTree =
  | ['+', Scalar, Scalar]
  | ['-', Scalar, Scalar]
  | ['*', Scalar, Scalar]
  | ['/', Scalar, Scalar]
  | ['%', Scalar, Scalar]
  | ['**', Scalar, Scalar]
  | ['min', Scalar, Scalar]
  | ['max', Scalar, Scalar]
  | ['<', Scalar, Scalar]
  | ['>', Scalar, Scalar]
  | ['<=', Scalar, Scalar]
  | ['>=', Scalar, Scalar]
  | ['==', Scalar, Scalar]
  | ['!=', Scalar, Scalar]
  | ['&&', Scalar, Scalar]
  | ['||', Scalar, Scalar]
  | ['!', Scalar]
  | ['if', Scalar, Scalar, Scalar]
  | [Fn1, Scalar]
  | [Fn2, Scalar, Scalar]
  | [Fn3, Scalar, Scalar, Scalar]
  | ['clamp', Scalar, Scalar, Scalar]
  | ['lerp', Scalar, Scalar, Scalar]
  | ['smoothstep', Scalar, Scalar, Scalar]

type Fn1 =
  | 'abs'
  | 'floor'
  | 'ceil'
  | 'round'
  | 'sqrt'
  | 'exp'
  | 'log'
  | 'sin'
  | 'cos'
  | 'tan'
  | 'sign'
type Fn2 = 'step' | 'min' | 'max'
type Fn3 = 'clamp' | 'lerp' | 'smoothstep'

/** A colour, or a mix between two taken where a dash sits. */
export type Color = string | Scalar | [number, number, number] | ColorTree

export type ColorTree =
  | ['lerp', Color, Color, Scalar]
  | ['mix', Color, Color, number]
  | ['r' | 'g' | 'b', Color]

/** A colour or a scalar: scalars used where a colour goes are greys. */
export type Paint = Color

/** What a batch draws. */
export type Draw = 'lines' | 'dots' | 'fill'

/** What a fill is drawn over. */
export type Ground = 'land' | 'sea'

/** How a line's ends look. */
export type Cap = 'round' | 'round-inside' | 'butt'

/** Parameters of a style, as a document's `style` block names them. */
export interface Style {
  [name: string]: number
}

/** One batch on the land. */
export interface BatchOptions {
  /** What it is called in reports, and its layer unless `layer` is set. */
  name?: string
  /** The layer its dashes go on. */
  layer?: string
  /** What it draws; `lines` by default. */
  draw?: Draw
  /** What a fill is drawn over. */
  ground?: Ground
  /** The field the streamlines run along, degrees. */
  angle?: number
  /** The colour every dash is inked with. */
  color?: string
  /** The colour at each dash; wins over `color`. */
  shade?: Color
  /** The width at each dash, as a multiple of `batch_stroke`. */
  width?: Scalar
  /** How many widths `width` is rounded to, each with its own layer. */
  widthSteps?: number
  cap?: Cap
  /** Style parameters, by name. */
  style?: Style
}

/** A run of streamlines, or a stipple, or a fill. */
export interface Batch {
  /** The batch's name in reports and as its layer. */
  named(name: string): Batch
  /** The colour every dash is inked with, `#rgb` or `#rrggbb`. */
  color(c: string): Batch
  /** The colour at each dash, from a tree; wins over `color`. */
  shade(c: Color): Batch
  /** The width at each dash, as a multiple of `batch_stroke`. */
  width(w: Scalar): Batch
  cap(c: Cap): Batch
  /** The field the streamlines run along, degrees. */
  angle(deg: number): Batch
  /** What a fill is drawn over. */
  ground(g: Ground): Batch
  widthSteps(n: number): Batch
  /** One style parameter by name, as a document's `style` block does. */
  set(name: string, v: number): Batch
}

/** A pass on the water. */
export interface WaterOptions {
  name?: string
  color?: string
  shade?: Color
  width?: Scalar
  cap?: Cap
  /** `shore` follows the coast out; `angle` draws straight across. */
  follow?: 'shore' | 'angle'
  angle?: number
  style?: Style
  /** Keep only water this deep, in metres below the surface. */
  where?: { depthMin?: number; depthMax?: number }
}

export interface Water {
  named(name: string): Water
  color(c: string): Water
  shade(c: Color): Water
  width(w: Scalar): Water
  cap(c: Cap): Water
  follow(f: 'shore' | 'angle'): Water
  angle(deg: number): Water
  style(name: string, v: number): Water
}

/** The shoreline: the coast, and `lines - 1` more lines out to sea. */
export interface ShoreOptions {
  /** How many lines follow the coast; 1 is the coast itself. */
  lines?: number
  color?: string
  shade?: Color
  width?: Scalar
  style?: Style
}

export interface Shore {
  lines(n: number): Shore
  color(c: string): Shore
  shade(c: Color): Shore
  width(w: Scalar): Shore
  style(name: string, v: number): Shore
}

/** A drawing being put together. */
export interface Atlas {
  /** Append a batch on the land. */
  batch(b: Batch | BatchOptions): Atlas
  /** Append a pass on the water. The first one makes the sea appear. */
  water(w: Water | WaterOptions): Atlas
  /** Draw the shore. */
  shore(s: Shore | ShoreOptions): Atlas
  /**
   * Draw over an elevation grid, in metres, `width` × `height` px at `pxM`
   * metres per pixel. The sea comes from the elevation unless `coverage`
   * is given.
   */
  draw(grid: Grid, coverage?: number[]): Drawing
}

export interface Grid {
  /** Heights in metres, row 0 north, x growing east. */
  elev: number[]
  width: number
  height: number
  pxM: number
}

/** What came out: dashes, the lines they were cut from, and the runs of pen. */
export interface Drawing {
  width: number
  height: number
  /** Dashes in drawing order, as `[x, y]` pairs. */
  paths: number[][][]
  /** Streamlines before dashing. */
  lines: number[][][]
  /** Runs of pen over `paths`, with the layer of each. */
  runs: { stroke: number; color: string; layer: string | null }[]
  /** Filled areas, bottom first. */
  fills: unknown[]
  /** SVG of the drawing, as `burin.svg(drawing)`. */
  svg?: string
}

export interface Burin {
  /** A new, empty drawing. */
  atlas(): Atlas
  /** A batch: `burin.batch({ name: 'hills' })`. */
  batch(o?: BatchOptions): Batch
  /** Water passes: `burin.water.lines()`, `.ripples()`, `.dots()`. */
  water: {
    (o?: WaterOptions): Water
    lines(o?: WaterOptions): Water
    ripples(o?: WaterOptions): Water
    dots(o?: WaterOptions): Water
  }
  /** The shoreline. */
  shore(o?: ShoreOptions): Shore
  /** The known style parameters, by name. */
  parameters(): string[]
  /** The drawing as SVG, at the size of its grid. */
  svg(d: Drawing): string

  // Fields: what a dash can read where it sits.
  /** Height above the sea, metres. */
  readonly elev: Field
  /** Shading, 0…1, light to dark. */
  readonly tone: Field
  /** Distance from the shore, px. Water only. */
  readonly dist: Field
  /** Metres below the surface. Water only. */
  readonly depth: Field

  // Colours, as trees.
  /** `a` mixed `t` of the way to `b`. */
  lerp(a: Color, b: Color, t: Scalar): ColorTree
  /** `a` and `b` mixed, `t` decided once. */
  mix(a: Color, b: Color, t: number): ColorTree
  /** A grey, or a colour's channel, 0…1. */
  rgb(r: Scalar, g: Scalar, b: Scalar): ColorTree

  // Scalars, as trees.
  clamp(x: Scalar, lo: Scalar, hi: Scalar): ScalarTree
  smoothstep(edge0: Scalar, edge1: Scalar, x: Scalar): ScalarTree
  step(edge: Scalar, x: Scalar): ScalarTree
  abs(x: Scalar): ScalarTree
  floor(x: Scalar): ScalarTree
  ceil(x: Scalar): ScalarTree
  round(x: Scalar): ScalarTree
  sqrt(x: Scalar): ScalarTree
  sign(x: Scalar): ScalarTree
  min(a: Scalar, b: Scalar): ScalarTree
  max(a: Scalar, b: Scalar): ScalarTree
}

/** The global a burin script sees. */
declare const burin: Burin
