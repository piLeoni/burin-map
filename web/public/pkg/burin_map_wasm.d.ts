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
     * GeoJSON MultiLineString FeatureCollection of the dashes.
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
    readonly width: number;
}

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
 * Draw with multiple batch passes — accepts a JSON string of `Vec<Batch>`.
 * `geo` is an optional `{ nw_lon, nw_lat, deg_per_px_lon, deg_per_px_lat }`.
 * With `mm_per_px` the SVG is sized in millimetres, for paper.
 */
export function draw_batches(elev: Float32Array, width: number, height: number, px_m: number, json: string, geo: any, mm_per_px?: number | null): Drawing;

/**
 * [`draw_batches`] with the terrain fields built on the GPU (call
 * [`init_gpu`] first). Lines come out slightly different from the CPU path.
 */
export function draw_batches_gpu(elev: Float32Array, width: number, height: number, px_m: number, json: string, geo: any, mm_per_px?: number | null): Promise<Drawing>;

/**
 * `initGpu()`: set up WebGPU for [`draw_batches_gpu`]. Resolves to the
 * adapter description, or `undefined` when WebGPU isn't available.
 */
export function initGpu(): Promise<string | undefined>;

/**
 * (id, blurb) for each style group — section headers in the web UI.
 */
export function style_groups(): any;

/**
 * (name, default, help, group) for every style parameter — drives the web UI.
 */
export function style_params(): any;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly memory: WebAssembly.Memory;
    readonly __wbg_drawing_free: (a: number, b: number) => void;
    readonly draw: (a: number, b: number, c: number, d: number, e: number, f: any, g: any) => [number, number, number];
    readonly draw_batches: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: any, i: number) => [number, number, number];
    readonly draw_batches_gpu: (a: number, b: number, c: number, d: number, e: number, f: number, g: number, h: any, i: number) => any;
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
    readonly drawing_width: (a: number) => number;
    readonly initGpu: () => any;
    readonly style_groups: () => [number, number, number];
    readonly style_params: () => [number, number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h27deb920fd659eb5: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h27deb920fd659eb5_22: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h27deb920fd659eb5_23: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen__convert__closures_____invoke__h468977ef09679dd1: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen__convert__closures_____invoke__hca337089dd2b96cb: (a: number, b: number, c: any) => [number, number];
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
