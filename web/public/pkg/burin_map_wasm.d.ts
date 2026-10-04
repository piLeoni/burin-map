/* tslint:disable */
/* eslint-disable */

/**
 * Result of a draw: canvas-ready paths plus an SVG string.
 */
export class Drawing {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    readonly dashes: number;
    /**
     * Degrees of latitude per pixel (southward).
     */
    readonly deg_per_px_lat: number;
    /**
     * Degrees of longitude per pixel (eastward).
     */
    readonly deg_per_px_lon: number;
    readonly ends: Uint32Array;
    /**
     * GeoJSON FeatureCollection: the dashes as one MultiLineString, then
     * each fill as a MultiPolygon.
     */
    readonly geojson: string;
    readonly height: number;
    readonly lines: number;
    readonly ms: number;
    /**
     * NW corner latitude.
     */
    readonly nw_lat: number;
    /**
     * NW corner longitude.
     */
    readonly nw_lon: number;
    readonly paths: Float32Array;
    readonly svg: string;
    /**
     * `[label, ms]` for each phase of the draw, in the order they ran.
     */
    readonly timings: Array<any>;
    readonly width: number;
}

/**
 * The engine's progress panel (`burin_map_core::panel`), the one the command
 * line shows: the page tells it what happens, with the seconds since the
 * draw began, and shows what [`Panel::view`] returns.
 */
export class Panel {
    free(): void;
    [Symbol.dispose](): void;
    /**
     * The area drawn, `[west, south, east, north]` in degrees.
     */
    area(west: number, south: number, east: number, north: number, now: number): void;
    /**
     * What the board says in `slot`.
     */
    board(slot: number, value: number, now: number): void;
    /**
     * The drawing holds `count` dashes.
     */
    dashes(count: number, now: number): void;
    /**
     * The elevation grid, from `tiles` tiles at `zoom`.
     */
    elevation(elev: Float32Array, tiles: number, zoom: number, width: number, height: number, px_m: number, now: number): void;
    /**
     * The drawing, and whatever still runs, as done (`ok`) or failed.
     */
    finish(ok: boolean, now: number): void;
    /**
     * A line for the messages; `warn` when something went wrong on the way.
     */
    message(text: string, warn: boolean, now: number): void;
    /**
     * A panel for drawing `title`: these batches (JSON, as for
     * `draw_batches`) and, with water, this water spec (JSON, as `water.spec`).
     */
    constructor(title: string, json: string, water_spec?: string | null);
    /**
     * The board slots to read while the draw runs.
     */
    slots(): Uint32Array;
    /**
     * An elevation tile in; `done` of `total`.
     */
    tile(done: number, total: number, now: number): void;
    /**
     * What to show, as JSON: `{ head, title, rows: [{ state, mark, name,
     * child, bar: [lead, full, rest] | null, detail }], name_width,
     * bar_width, messages: [{ at, text, warn }], earlier }`.
     */
    view(now: number): string;
    /**
     * A water tile in; `done` of `total`.
     */
    waterTile(done: number, total: number, now: number): void;
    /**
     * Every water tile in.
     */
    water(now: number): void;
}

/**
 * The style parameters, as the editor's completion and help want them.
 */
export function burinParameters(): any;

/**
 * Draw an elevation grid: metres, row 0 = north, square pixels of `px_m`
 * metres. `elev` is a Float32Array (or any TypedArray view) of length
 * `width × height`. Values ≤ 0.5 m are sea. `style` is an object of
 * parameter overrides (missing keys keep the defaults).
 * `geo` is an optional `{ nw_lon, nw_lat, deg_per_px_lon, deg_per_px_lat }` —
 * when present, `.geojson` will contain geographic coordinates.
 */
export function draw(elev: Float32Array, width: number, height: number, px_m: number, style: any, geo: any): Drawing;

/**
 * Draw an atlas a script built: `atlas` is its list of batches and water
 * passes, the same JSON `burin-map-script` hands back and the CLI's `--script`
 * draws.
 */
export function draw_atlas(elev: Float32Array, width: number, height: number, px_m: number, atlas: string, geo: any, mm_per_px: number | null | undefined, water: any): Drawing;

/**
 * Draw with multiple batch passes — accepts a JSON string of `Vec<Batch>`.
 * `geo` is an optional `{ nw_lon, nw_lat, deg_per_px_lon, deg_per_px_lat }`.
 * With `mm_per_px` the SVG is sized in millimetres, for paper. `water`: see
 * [`read_water`]; it decides the land, and its batches draw on it.
 */
export function draw_batches(elev: Float32Array, width: number, height: number, px_m: number, json: string, geo: any, mm_per_px: number | null | undefined, water: any): Drawing;

/**
 * [`draw_batches`] with the terrain fields built on the GPU (call
 * [`init_gpu`] first). Lines come out slightly different from the CPU path.
 */
export function draw_batches_gpu(elev: Float32Array, width: number, height: number, px_m: number, json: string, geo: any, mm_per_px: number | null | undefined, water: any): Promise<Drawing>;

/**
 * Metres on a north-up Web Mercator grid `width` × `height` over the box,
 * row 0 at the top: the sea floor below 0, 0 where no tile reaches.
 * `tiles` is an array of `[z, x, y, bytes]` as listed by [`terrain_tiles`],
 * bytes empty where the source has none. The deepest zoom is the terrain;
 * zoom-10 tiles under it give back the sea floor it flattens. Sampled with
 * Catmull-Rom, like the command line.
 */
export function elevation_grid(tiles: Array<any>, west: number, south: number, east: number, north: number, width: number, height: number): Float32Array;

/**
 * `initGpu()`: set up WebGPU for [`draw_batches_gpu`]. Resolves to the
 * adapter description, or `undefined` when WebGPU isn't available.
 */
export function initGpu(): Promise<string | undefined>;

/**
 * True when any water batch or shoreline colouring needs a depth grid.
 */
export function needsWaterDepth(spec_json: string): boolean;

/**
 * `progressBoard()`: the board, as a view on this module's memory.
 */
export function progressBoard(): Uint32Array;

/**
 * (id, blurb) for each style group — section headers in the web UI.
 */
export function style_groups(): any;

/**
 * (name, default, water_default, ripples_default, dots_default,
 * shoreline_default, help, group) for every style parameter — drives the web
 * UI. `water_default` is the value in water batches that draw lines,
 * `ripples_default` and `dots_default` in those that draw ripples or dots,
 * `shoreline_default` on the shoreline.
 */
export function style_params(): any;

/**
 * The terrain tiles (Terrarium PNG) a north-up Web Mercator grid `width` px
 * across the box needs, as flat `[z, x, y, …]`: the grid's own tiles at the
 * zoom that matches it (zoomed out until there are at most `max_tiles`),
 * then, deeper than zoom 10, their zoom-10 ancestors, which hold the sea
 * floor those zooms flatten to 0 m. `x` may run past 2^z - 1 like
 * [`water_tiles`].
 */
export function terrain_tiles(west: number, south: number, east: number, north: number, width: number, max_tiles: number): Uint32Array;

/**
 * Water tiles to fetch for a north-up Web Mercator grid `width` px across the
 * box, flat: z, x, y, z, x, y, … Across the 180° meridian x runs past
 * 2^z - 1; wrap it for the URL, but hand it back as it is.
 */
export function water_tiles(west: number, south: number, east: number, north: number, width: number, max_zoom: number, max_tiles: number): Uint32Array;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_drawing_free: (a: number, b: number) => void;
    readonly __wbg_panel_free: (a: number, b: number) => void;
    readonly burinParameters: () => [number, number, number];
    readonly draw: (a: number, b: number, c: number, d: number, e: number, f: any, g: any) => [number, number, number];
    readonly draw_atlas: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: any, i: number, j: any) => [number, number, number];
    readonly draw_batches: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: any, i: number, j: any) => [number, number, number];
    readonly draw_batches_gpu: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: any, i: number, j: any) => any;
    readonly drawing_dashes: (a: number) => number;
    readonly drawing_deg_per_px_lat: (a: number) => number;
    readonly drawing_deg_per_px_lon: (a: number) => number;
    readonly drawing_ends: (a: number) => any;
    readonly drawing_geojson: (a: number) => [number, number];
    readonly drawing_height: (a: number) => number;
    readonly drawing_lines: (a: number) => number;
    readonly drawing_ms: (a: number) => number;
    readonly drawing_nw_lat: (a: number) => number;
    readonly drawing_nw_lon: (a: number) => number;
    readonly drawing_paths: (a: number) => any;
    readonly drawing_svg: (a: number) => [number, number];
    readonly drawing_timings: (a: number) => any;
    readonly drawing_width: (a: number) => number;
    readonly elevation_grid: (a: any, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number, number];
    readonly initGpu: () => any;
    readonly needsWaterDepth: (a: number, b: number) => number;
    readonly panel_area: (a: number, b: number, c: number, d: number, e: number, f: number) => void;
    readonly panel_board: (a: number, b: number, c: number, d: number) => void;
    readonly panel_dashes: (a: number, b: number, c: number) => void;
    readonly panel_elevation: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: number, i: number) => void;
    readonly panel_finish: (a: number, b: number, c: number) => void;
    readonly panel_message: (a: number, b: number, c: number, d: number, e: number) => void;
    readonly panel_new: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number, number];
    readonly panel_slots: (a: number) => [number, number];
    readonly panel_tile: (a: number, b: number, c: number, d: number) => void;
    readonly panel_view: (a: number, b: number) => [number, number];
    readonly panel_water: (a: number, b: number) => void;
    readonly panel_waterTile: (a: number, b: number, c: number, d: number) => void;
    readonly progressBoard: () => any;
    readonly style_groups: () => [number, number, number];
    readonly style_params: () => [number, number, number];
    readonly terrain_tiles: (a: number, b: number, c: number, d: number, e: number, f: number) => [number, number];
    readonly water_tiles: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number];
    readonly wasm_bindgen_13dab8c06ff4562a___convert__closures_____invoke___js_sys_620190cd17e8d420___Function_fn_wasm_bindgen_13dab8c06ff4562a___JsValue_____wasm_bindgen_13dab8c06ff4562a___sys__Undefined___js_sys_620190cd17e8d420___Function_fn_wasm_bindgen_13dab8c06ff4562a___JsValue_____wasm_bindgen_13dab8c06ff4562a___sys__Undefined_______true_: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen_13dab8c06ff4562a___convert__closures_____invoke___wasm_bindgen_13dab8c06ff4562a___JsValue__core_ed718c3d60ebd546___result__Result_____wasm_bindgen_13dab8c06ff4562a___JsError___true_: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen_13dab8c06ff4562a___convert__closures_____invoke___wasm_bindgen_13dab8c06ff4562a___sys__JsNullable_wgpu_6427849c780d150b___backend__webgpu__webgpu_sys__gen_GpuError__GpuError___core_ed718c3d60ebd546___result__Result_____wasm_bindgen_13dab8c06ff4562a___JsError___true_: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen_13dab8c06ff4562a___convert__closures_____invoke___wasm_bindgen_13dab8c06ff4562a___sys__JsNullable_wgpu_6427849c780d150b___backend__webgpu__webgpu_sys__gen_GpuError__GpuError___core_ed718c3d60ebd546___result__Result_____wasm_bindgen_13dab8c06ff4562a___JsError___true__42: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen_13dab8c06ff4562a___convert__closures_____invoke___wasm_bindgen_13dab8c06ff4562a___sys__JsNullable_wgpu_6427849c780d150b___backend__webgpu__webgpu_sys__gen_GpuError__GpuError___core_ed718c3d60ebd546___result__Result_____wasm_bindgen_13dab8c06ff4562a___JsError___true__43: (a: number, b: number, c: any) => [number, number];
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_destroy_closure: (a: number, b: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
