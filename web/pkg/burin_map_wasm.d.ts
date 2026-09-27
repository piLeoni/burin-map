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
    readonly ends: Uint32Array;
    readonly height: number;
    readonly lines: number;
    readonly ms: number;
    readonly paths: Float32Array;
    readonly svg: string;
    readonly width: number;
}

/**
 * Draw an elevation grid: metres, row 0 = north, square pixels of `px_m`
 * metres. `elev` is a Float32Array (or any TypedArray view) of length
 * `width × height`. Values ≤ 0.5 m are sea. `style` is an object of
 * parameter overrides (missing keys keep the defaults).
 */
export function draw(elev: Float32Array, width: number, height: number, px_m: number, style: any): Drawing;

/**
 * Draw with multiple batch passes — accepts a JSON string of `Vec<Batch>`.
 */
export function draw_batches(elev: Float32Array, width: number, height: number, px_m: number, json: string): Drawing;

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
    readonly draw: (a: number, b: number, c: number, d: number, e: number, f: any) => [number, number, number];
    readonly draw_batches: (a: number, b: number, c: number, d: number, e: number, f: number, g: number) => [number, number, number];
    readonly drawing_dashes: (a: number) => number;
    readonly drawing_ends: (a: number) => any;
    readonly drawing_height: (a: number) => number;
    readonly drawing_lines: (a: number) => number;
    readonly drawing_ms: (a: number) => number;
    readonly drawing_paths: (a: number) => any;
    readonly drawing_svg: (a: number) => [number, number];
    readonly drawing_width: (a: number) => number;
    readonly style_groups: () => [number, number, number];
    readonly style_params: () => [number, number, number];
    readonly __wbindgen_exn_store: (a: number) => void;
    readonly __externref_table_alloc: () => number;
    readonly __wbindgen_externrefs: WebAssembly.Table;
    readonly __wbindgen_malloc: (a: number, b: number) => number;
    readonly __externref_table_dealloc: (a: number) => void;
    readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
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
