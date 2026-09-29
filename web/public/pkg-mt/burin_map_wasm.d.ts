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
    /**
     * `[label, ms]` for each phase of the draw, in the order they ran.
     */
    readonly timings: Array<any>;
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
 * `initGpu()`: set up WebGPU for [`draw_batches_gpu`]. Resolves to the
 * adapter description, or `undefined` when WebGPU isn't available.
 */
export function initGpu(): Promise<string | undefined>;

export function initThreadPool(num_threads: number): Promise<any>;

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
 * Water tiles to fetch for a north-up Web Mercator grid `width` px across the
 * box, flat: z, x, y, z, x, y, … Across the 180° meridian x runs past
 * 2^z - 1; wrap it for the URL, but hand it back as it is.
 */
export function water_tiles(west: number, south: number, east: number, north: number, width: number, max_zoom: number, max_tiles: number): Uint32Array;

export class wbg_rayon_PoolBuilder {
    private constructor();
    free(): void;
    [Symbol.dispose](): void;
    build(): void;
    mainJS(): string;
    numThreads(): number;
    receiver(): number;
}

export function wbg_rayon_start_worker(receiver: number): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
    readonly __wbg_drawing_free: (a: number, b: number) => void;
    readonly __wbg_wbg_rayon_poolbuilder_free: (a: number, b: number) => void;
    readonly draw: (a: number, b: number, c: number, d: number, e: number, f: any, g: any) => [number, number, number];
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
    readonly initGpu: () => any;
    readonly initThreadPool: (a: number) => any;
    readonly style_groups: () => [number, number, number];
    readonly style_params: () => [number, number, number];
    readonly water_tiles: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number];
    readonly wbg_rayon_poolbuilder_build: (a: number) => void;
    readonly wbg_rayon_poolbuilder_mainJS: (a: number) => any;
    readonly wbg_rayon_poolbuilder_numThreads: (a: number) => number;
    readonly wbg_rayon_poolbuilder_receiver: (a: number) => number;
    readonly wbg_rayon_start_worker: (a: number) => void;
    readonly wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___js_sys_3eb0eb5c2e5854c9___Function_fn_wasm_bindgen_6b83e68b973024d4___JsValue_____wasm_bindgen_6b83e68b973024d4___sys__Undefined___js_sys_3eb0eb5c2e5854c9___Function_fn_wasm_bindgen_6b83e68b973024d4___JsValue_____wasm_bindgen_6b83e68b973024d4___sys__Undefined_______true_: (a: number, b: number, c: any, d: any) => void;
    readonly wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___wasm_bindgen_6b83e68b973024d4___JsValue__core_9a364347cfd915f1___result__Result_____wasm_bindgen_6b83e68b973024d4___JsError___true_: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___wasm_bindgen_6b83e68b973024d4___sys__JsNullable_wgpu_e195cbd6f832a32a___backend__webgpu__webgpu_sys__gen_GpuError__GpuError___core_9a364347cfd915f1___result__Result_____wasm_bindgen_6b83e68b973024d4___JsError___true_: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___wasm_bindgen_6b83e68b973024d4___sys__JsNullable_wgpu_e195cbd6f832a32a___backend__webgpu__webgpu_sys__gen_GpuError__GpuError___core_9a364347cfd915f1___result__Result_____wasm_bindgen_6b83e68b973024d4___JsError___true__32: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___wasm_bindgen_6b83e68b973024d4___sys__JsNullable_wgpu_e195cbd6f832a32a___backend__webgpu__webgpu_sys__gen_GpuError__GpuError___core_9a364347cfd915f1___result__Result_____wasm_bindgen_6b83e68b973024d4___JsError___true__33: (a: number, b: number, c: any) => [number, number];
    readonly wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___js_sys_3eb0eb5c2e5854c9___futures__task__wait_async_polyfill__MessageEvent______true_: (a: number, b: number, c: any) => void;
    readonly wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___wasm_bindgen_6b83e68b973024d4___JsValue______true_: (a: number, b: number, c: any) => void;
    readonly memory: WebAssembly.Memory;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_destroy_closure: (a: number, b: number) => void;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_free: (a: number, b: number, c: number) => void;
    readonly __wbindgen_thread_destroy: (a?: number, b?: number, c?: number) => void;
    readonly __wbindgen_start: (a: number) => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;

/**
 * Instantiates the given `module`, which can either be bytes or
 * a precompiled `WebAssembly.Module`.
 *
 * @param {{ module: SyncInitInput, memory?: WebAssembly.Memory, thread_stack_size?: number }} module - Passing `SyncInitInput` directly is deprecated.
 * @param {WebAssembly.Memory} memory - Deprecated.
 *
 * @returns {InitOutput}
 */
export function initSync(module: { module: SyncInitInput, memory?: WebAssembly.Memory, thread_stack_size?: number } | SyncInitInput, memory?: WebAssembly.Memory): InitOutput;

/**
 * If `module_or_path` is {RequestInfo} or {URL}, makes a request and
 * for everything else, calls `WebAssembly.instantiate` directly.
 *
 * @param {{ module_or_path: InitInput | Promise<InitInput>, memory?: WebAssembly.Memory, thread_stack_size?: number }} module_or_path - Passing `InitInput` directly is deprecated.
 * @param {WebAssembly.Memory} memory - Deprecated.
 *
 * @returns {Promise<InitOutput>}
 */
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput>, memory?: WebAssembly.Memory, thread_stack_size?: number } | InitInput | Promise<InitInput>, memory?: WebAssembly.Memory): Promise<InitOutput>;
