/* @ts-self-types="./burin_map_wasm.d.ts" */
import { startWorkers } from './snippets/wasm-bindgen-rayon-38edf6e439f6d70d/src/workerHelpers.no-bundler.js';


/**
 * Result of a draw: canvas-ready paths plus an SVG string.
 */
export class Drawing {
    static __wrap(ptr) {
        const obj = Object.create(Drawing.prototype);
        obj.__wbg_ptr = ptr;
        DrawingFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        DrawingFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_drawing_free(ptr, 0);
    }
    /**
     * @returns {number}
     */
    get dashes() {
        const ret = wasm.drawing_dashes(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * Degrees of latitude per pixel (southward).
     * @returns {number}
     */
    get deg_per_px_lat() {
        const ret = wasm.drawing_deg_per_px_lat(this.__wbg_ptr);
        return ret;
    }
    /**
     * Degrees of longitude per pixel (eastward).
     * @returns {number}
     */
    get deg_per_px_lon() {
        const ret = wasm.drawing_deg_per_px_lon(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {Uint32Array}
     */
    get ends() {
        const ret = wasm.drawing_ends(this.__wbg_ptr);
        return ret;
    }
    /**
     * GeoJSON MultiLineString FeatureCollection of the dashes.
     * @returns {string}
     */
    get geojson() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.drawing_geojson(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * @returns {number}
     */
    get height() {
        const ret = wasm.drawing_height(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    get lines() {
        const ret = wasm.drawing_lines(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    get ms() {
        const ret = wasm.drawing_ms(this.__wbg_ptr);
        return ret;
    }
    /**
     * NW corner latitude.
     * @returns {number}
     */
    get nw_lat() {
        const ret = wasm.drawing_nw_lat(this.__wbg_ptr);
        return ret;
    }
    /**
     * NW corner longitude.
     * @returns {number}
     */
    get nw_lon() {
        const ret = wasm.drawing_nw_lon(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {Float32Array}
     */
    get paths() {
        const ret = wasm.drawing_paths(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {string}
     */
    get svg() {
        let deferred1_0;
        let deferred1_1;
        try {
            const ret = wasm.drawing_svg(this.__wbg_ptr);
            deferred1_0 = ret[0];
            deferred1_1 = ret[1];
            return getStringFromWasm0(ret[0], ret[1]);
        } finally {
            wasm.__wbindgen_free(deferred1_0, deferred1_1, 1);
        }
    }
    /**
     * `[label, ms]` for each phase of the draw, in the order they ran.
     * @returns {Array<any>}
     */
    get timings() {
        const ret = wasm.drawing_timings(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    get width() {
        const ret = wasm.drawing_width(this.__wbg_ptr);
        return ret >>> 0;
    }
}
if (Symbol.dispose) Drawing.prototype[Symbol.dispose] = Drawing.prototype.free;

/**
 * Draw an elevation grid: metres, row 0 = north, square pixels of `px_m`
 * metres. `elev` is a Float32Array (or any TypedArray view) of length
 * `width × height`. Values ≤ 0.5 m are sea. `style` is an object of
 * parameter overrides (missing keys keep the defaults).
 * `geo` is an optional `{ nw_lon, nw_lat, deg_per_px_lon, deg_per_px_lat }` —
 * when present, `.geojson` will contain geographic coordinates.
 * @param {Float32Array} elev
 * @param {number} width
 * @param {number} height
 * @param {number} px_m
 * @param {any} style
 * @param {any} geo
 * @returns {Drawing}
 */
export function draw(elev, width, height, px_m, style, geo) {
    const ptr0 = passArrayF32ToWasm0(elev, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.draw(ptr0, len0, width, height, px_m, style, geo);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Drawing.__wrap(ret[0]);
}

/**
 * Draw with multiple batch passes — accepts a JSON string of `Vec<Batch>`.
 * `geo` is an optional `{ nw_lon, nw_lat, deg_per_px_lon, deg_per_px_lat }`.
 * With `mm_per_px` the SVG is sized in millimetres, for paper. `water`: see
 * [`read_water`]; it decides the land, and its batches draw on it.
 * @param {Float32Array} elev
 * @param {number} width
 * @param {number} height
 * @param {number} px_m
 * @param {string} json
 * @param {any} geo
 * @param {number | null | undefined} mm_per_px
 * @param {any} water
 * @returns {Drawing}
 */
export function draw_batches(elev, width, height, px_m, json, geo, mm_per_px, water) {
    const ptr0 = passArrayF32ToWasm0(elev, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.draw_batches(ptr0, len0, width, height, px_m, ptr1, len1, geo, isLikeNone(mm_per_px) ? Number.MAX_SAFE_INTEGER : Math.fround(mm_per_px), water);
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return Drawing.__wrap(ret[0]);
}

/**
 * [`draw_batches`] with the terrain fields built on the GPU (call
 * [`init_gpu`] first). Lines come out slightly different from the CPU path.
 * @param {Float32Array} elev
 * @param {number} width
 * @param {number} height
 * @param {number} px_m
 * @param {string} json
 * @param {any} geo
 * @param {number | null | undefined} mm_per_px
 * @param {any} water
 * @returns {Promise<Drawing>}
 */
export function draw_batches_gpu(elev, width, height, px_m, json, geo, mm_per_px, water) {
    const ptr0 = passArrayF32ToWasm0(elev, wasm.__wbindgen_malloc);
    const len0 = WASM_VECTOR_LEN;
    const ptr1 = passStringToWasm0(json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len1 = WASM_VECTOR_LEN;
    const ret = wasm.draw_batches_gpu(ptr0, len0, width, height, px_m, ptr1, len1, geo, isLikeNone(mm_per_px) ? Number.MAX_SAFE_INTEGER : Math.fround(mm_per_px), water);
    return ret;
}

/**
 * `initGpu()`: set up WebGPU for [`draw_batches_gpu`]. Resolves to the
 * adapter description, or `undefined` when WebGPU isn't available.
 * @returns {Promise<string | undefined>}
 */
export function initGpu() {
    const ret = wasm.initGpu();
    return ret;
}

/**
 * @param {number} num_threads
 * @returns {Promise<any>}
 */
export function initThreadPool(num_threads) {
    const ret = wasm.initThreadPool(num_threads);
    return ret;
}

/**
 * True when any water batch or shoreline colouring needs a depth grid.
 * @param {string} spec_json
 * @returns {boolean}
 */
export function needsWaterDepth(spec_json) {
    const ptr0 = passStringToWasm0(spec_json, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
    const len0 = WASM_VECTOR_LEN;
    const ret = wasm.needsWaterDepth(ptr0, len0);
    return ret !== 0;
}

/**
 * (id, blurb) for each style group — section headers in the web UI.
 * @returns {any}
 */
export function style_groups() {
    const ret = wasm.style_groups();
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * (name, default, water_default, ripples_default, dots_default,
 * shoreline_default, help, group) for every style parameter — drives the web
 * UI. `water_default` is the value in water batches that draw lines,
 * `ripples_default` and `dots_default` in those that draw ripples or dots,
 * `shoreline_default` on the shoreline.
 * @returns {any}
 */
export function style_params() {
    const ret = wasm.style_params();
    if (ret[2]) {
        throw takeFromExternrefTable0(ret[1]);
    }
    return takeFromExternrefTable0(ret[0]);
}

/**
 * Water tiles to fetch for a north-up Web Mercator grid `width` px across the
 * box, flat: z, x, y, z, x, y, … Across the 180° meridian x runs past
 * 2^z - 1; wrap it for the URL, but hand it back as it is.
 * @param {number} west
 * @param {number} south
 * @param {number} east
 * @param {number} north
 * @param {number} width
 * @param {number} max_zoom
 * @param {number} max_tiles
 * @returns {Uint32Array}
 */
export function water_tiles(west, south, east, north, width, max_zoom, max_tiles) {
    const ret = wasm.water_tiles(west, south, east, north, width, max_zoom, max_tiles);
    var v1 = getArrayU32FromWasm0(ret[0], ret[1]).slice();
    wasm.__wbindgen_free(ret[0], ret[1] * 4, 4);
    return v1;
}

export class wbg_rayon_PoolBuilder {
    static __wrap(ptr) {
        const obj = Object.create(wbg_rayon_PoolBuilder.prototype);
        obj.__wbg_ptr = ptr;
        wbg_rayon_PoolBuilderFinalization.register(obj, obj.__wbg_ptr, obj);
        return obj;
    }
    __destroy_into_raw() {
        const ptr = this.__wbg_ptr;
        this.__wbg_ptr = 0;
        wbg_rayon_PoolBuilderFinalization.unregister(this);
        return ptr;
    }
    free() {
        const ptr = this.__destroy_into_raw();
        wasm.__wbg_wbg_rayon_poolbuilder_free(ptr, 0);
    }
    build() {
        wasm.wbg_rayon_poolbuilder_build(this.__wbg_ptr);
    }
    /**
     * @returns {string}
     */
    mainJS() {
        const ret = wasm.wbg_rayon_poolbuilder_mainJS(this.__wbg_ptr);
        return ret;
    }
    /**
     * @returns {number}
     */
    numThreads() {
        const ret = wasm.wbg_rayon_poolbuilder_numThreads(this.__wbg_ptr);
        return ret >>> 0;
    }
    /**
     * @returns {number}
     */
    receiver() {
        const ret = wasm.wbg_rayon_poolbuilder_receiver(this.__wbg_ptr);
        return ret >>> 0;
    }
}
if (Symbol.dispose) wbg_rayon_PoolBuilder.prototype[Symbol.dispose] = wbg_rayon_PoolBuilder.prototype.free;

/**
 * @param {number} receiver
 */
export function wbg_rayon_start_worker(receiver) {
    wasm.wbg_rayon_start_worker(receiver);
}
function __wbg_get_imports(memory) {
    const import0 = {
        __proto__: null,
        __wbg_Window_a2a6c4d665047b14: function(arg0) {
            const ret = arg0.Window;
            return ret;
        },
        __wbg_WorkerGlobalScope_2664448a7c667d67: function(arg0) {
            const ret = arg0.WorkerGlobalScope;
            return ret;
        },
        __wbg___wbindgen_debug_string_0e68cf47c9cbd9b0: function(arg0, arg1) {
            const ret = debugString(arg1);
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_is_function_fcda5e3902d732fe: function(arg0) {
            const ret = typeof(arg0) === 'function';
            return ret;
        },
        __wbg___wbindgen_is_null_5160b3e381865372: function(arg0) {
            const ret = arg0 === null;
            return ret;
        },
        __wbg___wbindgen_is_string_c4f7cb494a2a21f1: function(arg0) {
            const ret = typeof(arg0) === 'string';
            return ret;
        },
        __wbg___wbindgen_is_undefined_8c687d0b90d5b524: function(arg0) {
            const ret = arg0 === undefined;
            return ret;
        },
        __wbg___wbindgen_memory_3f8442e22540244f: function() {
            const ret = wasm.memory;
            return ret;
        },
        __wbg___wbindgen_module_c2aeeb3b14bad631: function() {
            const ret = wasmModule;
            return ret;
        },
        __wbg___wbindgen_number_get_1dc732b810cb937c: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'number' ? obj : undefined;
            getDataViewMemory0().setFloat64(arg0 + 8 * 1, isLikeNone(ret) ? 0 : ret, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, !isLikeNone(ret), true);
        },
        __wbg___wbindgen_rethrow_dba7bb2caa14ba21: function(arg0) {
            throw arg0;
        },
        __wbg___wbindgen_string_get_92ab86bb19cbc12f: function(arg0, arg1) {
            const obj = arg1;
            const ret = typeof(obj) === 'string' ? obj : undefined;
            var ptr1 = isLikeNone(ret) ? 0 : passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            var len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg___wbindgen_throw_5d9e815e6fdf150f: function(arg0, arg1) {
            throw new Error(getStringFromWasm0(arg0, arg1));
        },
        __wbg__wbg_cb_unref_997e73d32238e655: function(arg0) {
            arg0._wbg_cb_unref();
        },
        __wbg_async_50fcc3f740d37ed6: function(arg0) {
            const ret = arg0.async;
            return ret;
        },
        __wbg_beginComputePass_b9a325184985e6ff: function(arg0, arg1) {
            const ret = arg0.beginComputePass(arg1);
            return ret;
        },
        __wbg_buffer_60eec204070b2802: function(arg0) {
            const ret = arg0.buffer;
            return ret;
        },
        __wbg_call_269c5566fbede3eb: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.call(arg1);
            return ret;
        }, arguments); },
        __wbg_call_6bcf8d3e20937e46: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.call(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_configure_1e2c1c9edad07d26: function() { return handleError(function (arg0, arg1) {
            arg0.configure(arg1);
        }, arguments); },
        __wbg_copyBufferToBuffer_01766818654a9868: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4) {
            arg0.copyBufferToBuffer(arg1, arg2, arg3, arg4);
        }, arguments); },
        __wbg_copyBufferToBuffer_9c174b96fb08d551: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5) {
            arg0.copyBufferToBuffer(arg1, arg2, arg3, arg4, arg5);
        }, arguments); },
        __wbg_createBindGroup_f539b26ca341308f: function(arg0, arg1) {
            const ret = arg0.createBindGroup(arg1);
            return ret;
        },
        __wbg_createBuffer_d800e9b1d41b2ee5: function() { return handleError(function (arg0, arg1) {
            const ret = arg0.createBuffer(arg1);
            return ret;
        }, arguments); },
        __wbg_createCommandEncoder_3352d1ffc36c6fc0: function(arg0, arg1) {
            const ret = arg0.createCommandEncoder(arg1);
            return ret;
        },
        __wbg_createComputePipeline_224fa2618d9948a0: function(arg0, arg1) {
            const ret = arg0.createComputePipeline(arg1);
            return ret;
        },
        __wbg_createShaderModule_cefa51336cb288ae: function(arg0, arg1) {
            const ret = arg0.createShaderModule(arg1);
            return ret;
        },
        __wbg_data_d607e3ca86acb68c: function(arg0) {
            const ret = arg0.data;
            return ret;
        },
        __wbg_description_83b8a393160021b9: function(arg0, arg1) {
            const ret = arg1.description;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_dispatchWorkgroups_56b943172790add0: function(arg0, arg1, arg2, arg3) {
            arg0.dispatchWorkgroups(arg1 >>> 0, arg2 >>> 0, arg3 >>> 0);
        },
        __wbg_drawing_new: function(arg0) {
            const ret = Drawing.__wrap(arg0);
            return ret;
        },
        __wbg_end_82fd0c12ba185009: function(arg0) {
            arg0.end();
        },
        __wbg_finish_09ec094c10f41e7b: function(arg0) {
            const ret = arg0.finish();
            return ret;
        },
        __wbg_finish_ec1c191f66a895b1: function(arg0, arg1) {
            const ret = arg0.finish(arg1);
            return ret;
        },
        __wbg_from_a39669ce566077da: function(arg0) {
            const ret = Array.from(arg0);
            return ret;
        },
        __wbg_getBindGroupLayout_349f9101ad6d4d76: function(arg0, arg1) {
            const ret = arg0.getBindGroupLayout(arg1 >>> 0);
            return ret;
        },
        __wbg_getContext_5ff6bd600503b094: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getContext(getStringFromWasm0(arg1, arg2));
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        }, arguments); },
        __wbg_getMappedRange_fb54c6327b2d8d20: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = arg0.getMappedRange(arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_get_989d0a1309644f2b: function() { return handleError(function (arg0, arg1) {
            const ret = Reflect.get(arg0, arg1);
            return ret;
        }, arguments); },
        __wbg_get_b1f0ab13c737f856: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_get_unchecked_363572bdd397d473: function(arg0, arg1) {
            const ret = arg0[arg1 >>> 0];
            return ret;
        },
        __wbg_gpu_afdd4387c7afe5f9: function(arg0) {
            const ret = arg0.gpu;
            return ret;
        },
        __wbg_info_971d8b9db3dae69f: function(arg0) {
            const ret = arg0.info;
            return ret;
        },
        __wbg_instanceof_Window_a3b8566f0a9c5d1a: function(arg0) {
            let result;
            try {
                result = arg0 instanceof Window;
            } catch (_) {
                result = false;
            }
            const ret = result;
            return ret;
        },
        __wbg_isFallbackAdapter_4c8cc3b18677460a: function(arg0) {
            const ret = arg0.isFallbackAdapter;
            return ret;
        },
        __wbg_label_7add8cb37a6ef98f: function(arg0, arg1) {
            const ret = arg1.label;
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        __wbg_length_2ea75db9a3294a06: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_31bdaf014f5fbde2: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_4e1adc0d42e23620: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_length_8768c6f6941913e3: function(arg0) {
            const ret = arg0.length;
            return ret;
        },
        __wbg_limits_601ad2e086ef8141: function(arg0) {
            const ret = arg0.limits;
            return ret;
        },
        __wbg_mapAsync_b0597127f5037286: function(arg0, arg1, arg2, arg3) {
            const ret = arg0.mapAsync(arg1 >>> 0, arg2, arg3);
            return ret;
        },
        __wbg_maxBindGroupsPlusVertexBuffers_52369f089736ef9d: function(arg0) {
            const ret = arg0.maxBindGroupsPlusVertexBuffers;
            return ret;
        },
        __wbg_maxBindGroups_4e424afe6ce86ca2: function(arg0) {
            const ret = arg0.maxBindGroups;
            return ret;
        },
        __wbg_maxBindingsPerBindGroup_7d035da36821c44f: function(arg0) {
            const ret = arg0.maxBindingsPerBindGroup;
            return ret;
        },
        __wbg_maxBufferSize_423f4a084e32a195: function(arg0) {
            const ret = arg0.maxBufferSize;
            return ret;
        },
        __wbg_maxColorAttachmentBytesPerSample_c4cd9126f6d287c6: function(arg0) {
            const ret = arg0.maxColorAttachmentBytesPerSample;
            return ret;
        },
        __wbg_maxColorAttachments_d924670762b9e250: function(arg0) {
            const ret = arg0.maxColorAttachments;
            return ret;
        },
        __wbg_maxComputeInvocationsPerWorkgroup_707a3868f7cebb59: function(arg0) {
            const ret = arg0.maxComputeInvocationsPerWorkgroup;
            return ret;
        },
        __wbg_maxComputeWorkgroupSizeX_0a4d99463cbd6e5e: function(arg0) {
            const ret = arg0.maxComputeWorkgroupSizeX;
            return ret;
        },
        __wbg_maxComputeWorkgroupSizeY_85123ea0587f7558: function(arg0) {
            const ret = arg0.maxComputeWorkgroupSizeY;
            return ret;
        },
        __wbg_maxComputeWorkgroupSizeZ_a3186b4c5267d44f: function(arg0) {
            const ret = arg0.maxComputeWorkgroupSizeZ;
            return ret;
        },
        __wbg_maxComputeWorkgroupStorageSize_57b297355cfb6204: function(arg0) {
            const ret = arg0.maxComputeWorkgroupStorageSize;
            return ret;
        },
        __wbg_maxComputeWorkgroupsPerDimension_4158f95e673d54c4: function(arg0) {
            const ret = arg0.maxComputeWorkgroupsPerDimension;
            return ret;
        },
        __wbg_maxDynamicStorageBuffersPerPipelineLayout_226b0b70910aa16c: function(arg0) {
            const ret = arg0.maxDynamicStorageBuffersPerPipelineLayout;
            return ret;
        },
        __wbg_maxDynamicUniformBuffersPerPipelineLayout_0e835fda711fc7e6: function(arg0) {
            const ret = arg0.maxDynamicUniformBuffersPerPipelineLayout;
            return ret;
        },
        __wbg_maxInterStageShaderVariables_8c4a1d727e2aa35a: function(arg0) {
            const ret = arg0.maxInterStageShaderVariables;
            return ret;
        },
        __wbg_maxSampledTexturesPerShaderStage_6675f5e91d9a728a: function(arg0) {
            const ret = arg0.maxSampledTexturesPerShaderStage;
            return ret;
        },
        __wbg_maxSamplersPerShaderStage_1910fa38a6ed1e1f: function(arg0) {
            const ret = arg0.maxSamplersPerShaderStage;
            return ret;
        },
        __wbg_maxStorageBufferBindingSize_2e244bded070b18d: function(arg0) {
            const ret = arg0.maxStorageBufferBindingSize;
            return ret;
        },
        __wbg_maxStorageBuffersPerShaderStage_a285f3ebca51ca0d: function(arg0) {
            const ret = arg0.maxStorageBuffersPerShaderStage;
            return ret;
        },
        __wbg_maxStorageTexturesPerShaderStage_7aa946f0fc322a2b: function(arg0) {
            const ret = arg0.maxStorageTexturesPerShaderStage;
            return ret;
        },
        __wbg_maxTextureArrayLayers_0e699147ad00502d: function(arg0) {
            const ret = arg0.maxTextureArrayLayers;
            return ret;
        },
        __wbg_maxTextureDimension1D_aabf6add54decfe2: function(arg0) {
            const ret = arg0.maxTextureDimension1D;
            return ret;
        },
        __wbg_maxTextureDimension2D_dd598b27e9c0c1c4: function(arg0) {
            const ret = arg0.maxTextureDimension2D;
            return ret;
        },
        __wbg_maxTextureDimension3D_f944266c65dfd1a9: function(arg0) {
            const ret = arg0.maxTextureDimension3D;
            return ret;
        },
        __wbg_maxUniformBufferBindingSize_59fa6be7cfbeeb53: function(arg0) {
            const ret = arg0.maxUniformBufferBindingSize;
            return ret;
        },
        __wbg_maxUniformBuffersPerShaderStage_bee5f00a4d706c7f: function(arg0) {
            const ret = arg0.maxUniformBuffersPerShaderStage;
            return ret;
        },
        __wbg_maxVertexAttributes_5cf6392c4e9033fe: function(arg0) {
            const ret = arg0.maxVertexAttributes;
            return ret;
        },
        __wbg_maxVertexBufferArrayStride_548baa887375d865: function(arg0) {
            const ret = arg0.maxVertexBufferArrayStride;
            return ret;
        },
        __wbg_maxVertexBuffers_75d881156591f5da: function(arg0) {
            const ret = arg0.maxVertexBuffers;
            return ret;
        },
        __wbg_minStorageBufferOffsetAlignment_5ba9b77792bdadb3: function(arg0) {
            const ret = arg0.minStorageBufferOffsetAlignment;
            return ret;
        },
        __wbg_minUniformBufferOffsetAlignment_ab7d52a5293b22bd: function(arg0) {
            const ret = arg0.minUniformBufferOffsetAlignment;
            return ret;
        },
        __wbg_navigator_d217ca64c4bbff48: function(arg0) {
            const ret = arg0.navigator;
            return ret;
        },
        __wbg_navigator_d25c0f071226f233: function(arg0) {
            const ret = arg0.navigator;
            return ret;
        },
        __wbg_new_0700aaf827082758: function(arg0) {
            const ret = new Int32Array(arg0);
            return ret;
        },
        __wbg_new_12e5d807044fbfe3: function() { return handleError(function (arg0, arg1) {
            const ret = new OffscreenCanvas(arg0 >>> 0, arg1 >>> 0);
            return ret;
        }, arguments); },
        __wbg_new_1da3429bc3c4541c: function(arg0) {
            const ret = new Uint8Array(arg0);
            return ret;
        },
        __wbg_new_56479471e551e4fc: function(arg0) {
            const ret = new Float32Array(arg0);
            return ret;
        },
        __wbg_new_8f72ed7c652bf5a2: function(arg0, arg1) {
            try {
                var state0 = {a: arg0, b: arg1};
                var cb0 = (arg0, arg1) => {
                    const a = state0.a;
                    state0.a = 0;
                    try {
                        return wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___js_sys_3eb0eb5c2e5854c9___Function_fn_wasm_bindgen_6b83e68b973024d4___JsValue_____wasm_bindgen_6b83e68b973024d4___sys__Undefined___js_sys_3eb0eb5c2e5854c9___Function_fn_wasm_bindgen_6b83e68b973024d4___JsValue_____wasm_bindgen_6b83e68b973024d4___sys__Undefined_______true_(a, state0.b, arg0, arg1);
                    } finally {
                        state0.a = a;
                    }
                };
                const ret = new Promise(cb0);
                return ret;
            } finally {
                state0.a = 0;
            }
        },
        __wbg_new_bebc3f4757acf305: function() {
            const ret = new Object();
            return ret;
        },
        __wbg_new_ffa92086ea89f79c: function() {
            const ret = new Array();
            return ret;
        },
        __wbg_new_from_slice_2221cabb71753908: function(arg0, arg1) {
            const ret = new Float32Array(getArrayF32FromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_new_from_slice_a500ec81601be48f: function(arg0, arg1) {
            const ret = new Uint32Array(getArrayU32FromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_new_typed_6f8b0d724fe26c07: function(arg0, arg1) {
            try {
                var state0 = {a: arg0, b: arg1};
                var cb0 = (arg0, arg1) => {
                    const a = state0.a;
                    state0.a = 0;
                    try {
                        return wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___js_sys_3eb0eb5c2e5854c9___Function_fn_wasm_bindgen_6b83e68b973024d4___JsValue_____wasm_bindgen_6b83e68b973024d4___sys__Undefined___js_sys_3eb0eb5c2e5854c9___Function_fn_wasm_bindgen_6b83e68b973024d4___JsValue_____wasm_bindgen_6b83e68b973024d4___sys__Undefined_______true_(a, state0.b, arg0, arg1);
                    } finally {
                        state0.a = a;
                    }
                };
                const ret = new Promise(cb0);
                return ret;
            } finally {
                state0.a = 0;
            }
        },
        __wbg_new_typed_7d4574ab4b8446c8: function() {
            const ret = new Object();
            return ret;
        },
        __wbg_new_with_byte_offset_and_length_492c969e8b5da8a4: function(arg0, arg1, arg2) {
            const ret = new Uint8Array(arg0, arg1 >>> 0, arg2 >>> 0);
            return ret;
        },
        __wbg_new_worker_81c08e70aadb3771: function(arg0, arg1) {
            const ret = new Worker(getStringFromWasm0(arg0, arg1));
            return ret;
        },
        __wbg_now_d1fb6650485d7f3e: function() {
            const ret = Date.now();
            return ret;
        },
        __wbg_of_1c173c2ad96e2f7e: function(arg0, arg1) {
            const ret = Array.of(arg0, arg1);
            return ret;
        },
        __wbg_of_d9a05aa72bb023a2: function(arg0, arg1, arg2) {
            const ret = Array.of(arg0, arg1, arg2);
            return ret;
        },
        __wbg_onSubmittedWorkDone_1190213cee1ecf7e: function(arg0) {
            const ret = arg0.onSubmittedWorkDone();
            return ret;
        },
        __wbg_postMessage_8105a302ed002474: function() { return handleError(function (arg0, arg1) {
            arg0.postMessage(arg1);
        }, arguments); },
        __wbg_prototypesetcall_1629b854cfbe3b87: function(arg0, arg1, arg2) {
            Uint32Array.prototype.set.call(getArrayU32FromWasm0(arg0, arg1), arg2);
        },
        __wbg_prototypesetcall_804a1eb1b047ccb9: function(arg0, arg1, arg2) {
            Float32Array.prototype.set.call(getArrayF32FromWasm0(arg0, arg1), arg2);
        },
        __wbg_prototypesetcall_ae9f5e7459250748: function(arg0, arg1, arg2) {
            Uint8Array.prototype.set.call(getArrayU8FromWasm0(arg0, arg1), arg2);
        },
        __wbg_push_bfdf956ba476f65b: function(arg0, arg1) {
            const ret = arg0.push(arg1);
            return ret;
        },
        __wbg_queueMicrotask_85c90f6987555d65: function(arg0) {
            const ret = arg0.queueMicrotask;
            return ret;
        },
        __wbg_queueMicrotask_f6a1fa10b81d1fc0: function(arg0) {
            queueMicrotask(arg0);
        },
        __wbg_queue_7b62c28143d44293: function(arg0) {
            const ret = arg0.queue;
            return ret;
        },
        __wbg_requestAdapter_a539af006419f2e9: function(arg0, arg1) {
            const ret = arg0.requestAdapter(arg1);
            return ret;
        },
        __wbg_requestDevice_5cb8a582e55d08cb: function(arg0, arg1) {
            const ret = arg0.requestDevice(arg1);
            return ret;
        },
        __wbg_resolve_35ec7e0c6af4c82c: function(arg0) {
            const ret = Promise.resolve(arg0);
            return ret;
        },
        __wbg_setBindGroup_0b7c2a055ef1f314: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            arg0.setBindGroup(arg1 >>> 0, arg2, getArrayU32FromWasm0(arg3, arg4), arg5, arg6 >>> 0);
        }, arguments); },
        __wbg_setBindGroup_c83391351ce68826: function(arg0, arg1, arg2) {
            arg0.setBindGroup(arg1 >>> 0, arg2);
        },
        __wbg_setPipeline_5146f7b8d2b4af5c: function(arg0, arg1) {
            arg0.setPipeline(arg1);
        },
        __wbg_set_9cfc0f17d60ff0af: function(arg0, arg1, arg2) {
            arg0.set(arg1, arg2 >>> 0);
        },
        __wbg_set_a377297433dfea63: function() { return handleError(function (arg0, arg1, arg2) {
            const ret = Reflect.set(arg0, arg1, arg2);
            return ret;
        }, arguments); },
        __wbg_set_beginning_of_pass_write_index_fa9ae10d5d2804ce: function(arg0, arg1) {
            arg0.beginningOfPassWriteIndex = arg1 >>> 0;
        },
        __wbg_set_binding_b6cee57f35ac5190: function(arg0, arg1) {
            arg0.binding = arg1 >>> 0;
        },
        __wbg_set_buffer_1548ae88a9188037: function(arg0, arg1) {
            arg0.buffer = arg1;
        },
        __wbg_set_code_5d5b0b9e2fd0dca7: function(arg0, arg1, arg2) {
            arg0.code = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_compute_bc754d9e37f6e4c9: function(arg0, arg1) {
            arg0.compute = arg1;
        },
        __wbg_set_device_210484a77b675c9c: function(arg0, arg1) {
            arg0.device = arg1;
        },
        __wbg_set_end_of_pass_write_index_b54a8e3802b9ae0d: function(arg0, arg1) {
            arg0.endOfPassWriteIndex = arg1 >>> 0;
        },
        __wbg_set_entries_f26b77ab9548e906: function(arg0, arg1, arg2) {
            arg0.entries = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_entry_point_ac8db535971761fe: function(arg0, arg1, arg2) {
            arg0.entryPoint = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_format_f009e603f7d4c28e: function(arg0, arg1) {
            arg0.format = __wbindgen_enum_GpuTextureFormat[arg1];
        },
        __wbg_set_label_09e2da5f8522dbe8: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_17b4858e1eef23dd: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_2fefb39c0e0dbbe8: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_70a09ee68d6b1b26: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_92cd3811e96b487c: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_c7987704d29f284b: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_cfe64bca8945ee30: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_label_ee172cd5f6a96961: function(arg0, arg1, arg2) {
            arg0.label = getStringFromWasm0(arg1, arg2);
        },
        __wbg_set_layout_454e3a091b390cd4: function(arg0, arg1) {
            arg0.layout = arg1;
        },
        __wbg_set_layout_4b5d5b12fbb72a76: function(arg0, arg1) {
            arg0.layout = arg1;
        },
        __wbg_set_layout_gpu_auto_layout_mode_ee432e515c357fa8: function(arg0, arg1) {
            arg0.layout = __wbindgen_enum_GpuAutoLayoutMode[arg1];
        },
        __wbg_set_mapped_at_creation_3f320fef6761b02c: function(arg0, arg1) {
            arg0.mappedAtCreation = arg1 !== 0;
        },
        __wbg_set_module_c9946af218d23b53: function(arg0, arg1) {
            arg0.module = arg1;
        },
        __wbg_set_offset_f64_127e8a0aa5c5485a: function(arg0, arg1) {
            arg0.offset = arg1;
        },
        __wbg_set_onmessage_2244c2525c4e0c38: function(arg0, arg1) {
            arg0.onmessage = arg1;
        },
        __wbg_set_power_preference_b42d00a8facfbade: function(arg0, arg1) {
            arg0.powerPreference = __wbindgen_enum_GpuPowerPreference[arg1];
        },
        __wbg_set_query_set_f9a94586851fdba2: function(arg0, arg1) {
            arg0.querySet = arg1;
        },
        __wbg_set_required_features_bbab71414c45e621: function(arg0, arg1, arg2) {
            arg0.requiredFeatures = getArrayJsValueViewFromWasm0(arg1, arg2);
        },
        __wbg_set_required_limits_837f62d865e7cfac: function(arg0, arg1) {
            arg0.requiredLimits = arg1;
        },
        __wbg_set_resource_8fd8658b30d86ecf: function(arg0, arg1) {
            arg0.resource = arg1;
        },
        __wbg_set_resource_gpu_buffer_binding_33099b25da65b610: function(arg0, arg1) {
            arg0.resource = arg1;
        },
        __wbg_set_resource_gpu_texture_view_4cffe7bc7c8e5cbe: function(arg0, arg1) {
            arg0.resource = arg1;
        },
        __wbg_set_size_f64_2f591b0654540477: function(arg0, arg1) {
            arg0.size = arg1;
        },
        __wbg_set_size_f64_e844c985b8f95261: function(arg0, arg1) {
            arg0.size = arg1;
        },
        __wbg_set_timestamp_writes_5f240bbaad97fcaf: function(arg0, arg1) {
            arg0.timestampWrites = arg1;
        },
        __wbg_set_usage_f3e34822998d2147: function(arg0, arg1) {
            arg0.usage = arg1 >>> 0;
        },
        __wbg_startWorkers_622cedd0d351664e: function(arg0, arg1, arg2) {
            const ret = startWorkers(arg0, arg1, wbg_rayon_PoolBuilder.__wrap(arg2));
            return ret;
        },
        __wbg_static_accessor_GLOBAL_8eb4cd83130a11a0: function() {
            const ret = typeof global === 'undefined' ? null : global;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_GLOBAL_THIS_1e7044f654e934db: function() {
            const ret = typeof globalThis === 'undefined' ? null : globalThis;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_SELF_d8b50611246a6d92: function() {
            const ret = typeof self === 'undefined' ? null : self;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_static_accessor_URL_fac851ed3060f4eb: function() {
            const ret = import.meta.url;
            return ret;
        },
        __wbg_static_accessor_WINDOW_fd0bc376bf0f8b42: function() {
            const ret = typeof window === 'undefined' ? null : window;
            return isLikeNone(ret) ? 0 : addToExternrefTable0(ret);
        },
        __wbg_subgroupMaxSize_b43be0aa16182403: function(arg0) {
            const ret = arg0.subgroupMaxSize;
            return ret;
        },
        __wbg_subgroupMinSize_03feb6ee0cda6775: function(arg0) {
            const ret = arg0.subgroupMinSize;
            return ret;
        },
        __wbg_submit_077c85cc28e36892: function(arg0, arg1, arg2) {
            arg0.submit(getArrayJsValueViewFromWasm0(arg1, arg2));
        },
        __wbg_then_7a850dae4493f353: function(arg0, arg1, arg2) {
            const ret = arg0.then(arg1, arg2);
            return ret;
        },
        __wbg_then_8ebf7f4d2b402f98: function(arg0, arg1) {
            const ret = arg0.then(arg1);
            return ret;
        },
        __wbg_then_b830475380919203: function(arg0, arg1) {
            const ret = arg0.then(arg1);
            return ret;
        },
        __wbg_unconfigure_835307f58dc68d80: function(arg0) {
            arg0.unconfigure();
        },
        __wbg_unmap_6a96b14c9ef5f7f5: function(arg0) {
            arg0.unmap();
        },
        __wbg_value_a06131499b246ab0: function(arg0) {
            const ret = arg0.value;
            return ret;
        },
        __wbg_waitAsync_80b2e954334c3fdf: function(arg0, arg1, arg2) {
            const ret = Atomics.waitAsync(arg0, arg1 >>> 0, arg2);
            return ret;
        },
        __wbg_waitAsync_ac27a6bec1171c41: function() {
            const ret = Atomics.waitAsync;
            return ret;
        },
        __wbg_writeBuffer_f4bb3f54adfe1330: function() { return handleError(function (arg0, arg1, arg2, arg3, arg4, arg5, arg6) {
            arg0.writeBuffer(arg1, arg2, getArrayU8FromWasm0(arg3, arg4), arg5, arg6);
        }, arguments); },
        __wbindgen_generic_0000000000000001: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 268, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___wasm_bindgen_6b83e68b973024d4___JsValue______true_);
            return ret;
        },
        __wbindgen_generic_0000000000000002: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 333, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___wasm_bindgen_6b83e68b973024d4___JsValue__core_9a364347cfd915f1___result__Result_____wasm_bindgen_6b83e68b973024d4___JsError___true_);
            return ret;
        },
        __wbindgen_generic_0000000000000003: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [Externref], shim_idx: 335, ret: Unit, inner_ret: Some(Unit) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___js_sys_3eb0eb5c2e5854c9___futures__task__wait_async_polyfill__MessageEvent______true_);
            return ret;
        },
        __wbindgen_generic_0000000000000004: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("GPUDevice")], shim_idx: 266, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___wasm_bindgen_6b83e68b973024d4___sys__JsNullable_wgpu_e195cbd6f832a32a___backend__webgpu__webgpu_sys__gen_GpuError__GpuError___core_9a364347cfd915f1___result__Result_____wasm_bindgen_6b83e68b973024d4___JsError___true_);
            return ret;
        },
        __wbindgen_generic_0000000000000005: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("any")], shim_idx: 266, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___wasm_bindgen_6b83e68b973024d4___sys__JsNullable_wgpu_e195cbd6f832a32a___backend__webgpu__webgpu_sys__gen_GpuError__GpuError___core_9a364347cfd915f1___result__Result_____wasm_bindgen_6b83e68b973024d4___JsError___true__33);
            return ret;
        },
        __wbindgen_generic_0000000000000006: function(arg0, arg1) {
            // Cast intrinsic for `Closure(Closure { owned: true, function: Function { arguments: [NamedExternref("undefined")], shim_idx: 266, ret: Result(Unit), inner_ret: Some(Result(Unit)) }, mutable: true }) -> Externref`.
            const ret = makeMutClosure(arg0, arg1, wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___wasm_bindgen_6b83e68b973024d4___sys__JsNullable_wgpu_e195cbd6f832a32a___backend__webgpu__webgpu_sys__gen_GpuError__GpuError___core_9a364347cfd915f1___result__Result_____wasm_bindgen_6b83e68b973024d4___JsError___true__34);
            return ret;
        },
        __wbindgen_generic_0000000000000007: function(arg0) {
            // Cast intrinsic for `F64 -> Externref`.
            const ret = arg0;
            return ret;
        },
        __wbindgen_generic_0000000000000008: function(arg0, arg1) {
            // Cast intrinsic for `Ref(Slice(U8)) -> NamedExternref("Uint8Array")`.
            const ret = getArrayU8FromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_generic_0000000000000009: function(arg0, arg1) {
            // Cast intrinsic for `Ref(String) -> Externref`.
            const ret = getStringFromWasm0(arg0, arg1);
            return ret;
        },
        __wbindgen_init_externref_table: function() {
            const table = wasm.__wbindgen_externrefs;
            const offset = table.grow(4);
            table.set(0, undefined);
            table.set(offset + 0, undefined);
            table.set(offset + 1, null);
            table.set(offset + 2, true);
            table.set(offset + 3, false);
        },
        __wbindgen_link_e7dbf70342428633: function(arg0) {
            const val = `onmessage = function (ev) {
                let [ia, index, value] = ev.data;
                ia = new Int32Array(ia.buffer);
                let result = Atomics.wait(ia, index, value);
                postMessage(result);
            };
            `;
            const ret = typeof URL.createObjectURL === 'undefined' ? "data:application/javascript," + encodeURIComponent(val) : URL.createObjectURL(new Blob([val], { type: "text/javascript" }));
            const ptr1 = passStringToWasm0(ret, wasm.__wbindgen_malloc, wasm.__wbindgen_realloc);
            const len1 = WASM_VECTOR_LEN;
            getDataViewMemory0().setInt32(arg0 + 4 * 1, len1, true);
            getDataViewMemory0().setInt32(arg0 + 4 * 0, ptr1, true);
        },
        memory: memory || new WebAssembly.Memory({initial:19,maximum:65536,shared:true}),
    };
    return {
        __proto__: null,
        "./burin_map_wasm_bg.js": import0,
    };
}

function wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___wasm_bindgen_6b83e68b973024d4___JsValue______true_(arg0, arg1, arg2) {
    wasm.wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___wasm_bindgen_6b83e68b973024d4___JsValue______true_(arg0, arg1, arg2);
}

function wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___js_sys_3eb0eb5c2e5854c9___futures__task__wait_async_polyfill__MessageEvent______true_(arg0, arg1, arg2) {
    wasm.wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___js_sys_3eb0eb5c2e5854c9___futures__task__wait_async_polyfill__MessageEvent______true_(arg0, arg1, arg2);
}

function wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___wasm_bindgen_6b83e68b973024d4___JsValue__core_9a364347cfd915f1___result__Result_____wasm_bindgen_6b83e68b973024d4___JsError___true_(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___wasm_bindgen_6b83e68b973024d4___JsValue__core_9a364347cfd915f1___result__Result_____wasm_bindgen_6b83e68b973024d4___JsError___true_(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

function wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___wasm_bindgen_6b83e68b973024d4___sys__JsNullable_wgpu_e195cbd6f832a32a___backend__webgpu__webgpu_sys__gen_GpuError__GpuError___core_9a364347cfd915f1___result__Result_____wasm_bindgen_6b83e68b973024d4___JsError___true_(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___wasm_bindgen_6b83e68b973024d4___sys__JsNullable_wgpu_e195cbd6f832a32a___backend__webgpu__webgpu_sys__gen_GpuError__GpuError___core_9a364347cfd915f1___result__Result_____wasm_bindgen_6b83e68b973024d4___JsError___true_(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

function wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___wasm_bindgen_6b83e68b973024d4___sys__JsNullable_wgpu_e195cbd6f832a32a___backend__webgpu__webgpu_sys__gen_GpuError__GpuError___core_9a364347cfd915f1___result__Result_____wasm_bindgen_6b83e68b973024d4___JsError___true__33(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___wasm_bindgen_6b83e68b973024d4___sys__JsNullable_wgpu_e195cbd6f832a32a___backend__webgpu__webgpu_sys__gen_GpuError__GpuError___core_9a364347cfd915f1___result__Result_____wasm_bindgen_6b83e68b973024d4___JsError___true__33(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

function wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___wasm_bindgen_6b83e68b973024d4___sys__JsNullable_wgpu_e195cbd6f832a32a___backend__webgpu__webgpu_sys__gen_GpuError__GpuError___core_9a364347cfd915f1___result__Result_____wasm_bindgen_6b83e68b973024d4___JsError___true__34(arg0, arg1, arg2) {
    const ret = wasm.wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___wasm_bindgen_6b83e68b973024d4___sys__JsNullable_wgpu_e195cbd6f832a32a___backend__webgpu__webgpu_sys__gen_GpuError__GpuError___core_9a364347cfd915f1___result__Result_____wasm_bindgen_6b83e68b973024d4___JsError___true__34(arg0, arg1, arg2);
    if (ret[1]) {
        throw takeFromExternrefTable0(ret[0]);
    }
}

function wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___js_sys_3eb0eb5c2e5854c9___Function_fn_wasm_bindgen_6b83e68b973024d4___JsValue_____wasm_bindgen_6b83e68b973024d4___sys__Undefined___js_sys_3eb0eb5c2e5854c9___Function_fn_wasm_bindgen_6b83e68b973024d4___JsValue_____wasm_bindgen_6b83e68b973024d4___sys__Undefined_______true_(arg0, arg1, arg2, arg3) {
    wasm.wasm_bindgen_6b83e68b973024d4___convert__closures_____invoke___js_sys_3eb0eb5c2e5854c9___Function_fn_wasm_bindgen_6b83e68b973024d4___JsValue_____wasm_bindgen_6b83e68b973024d4___sys__Undefined___js_sys_3eb0eb5c2e5854c9___Function_fn_wasm_bindgen_6b83e68b973024d4___JsValue_____wasm_bindgen_6b83e68b973024d4___sys__Undefined_______true_(arg0, arg1, arg2, arg3);
}


const __wbindgen_enum_GpuAutoLayoutMode = ["auto"];


const __wbindgen_enum_GpuPowerPreference = ["low-power", "high-performance"];


const __wbindgen_enum_GpuTextureFormat = ["r8unorm", "r8snorm", "r8uint", "r8sint", "r16unorm", "r16snorm", "r16uint", "r16sint", "r16float", "rg8unorm", "rg8snorm", "rg8uint", "rg8sint", "r32uint", "r32sint", "r32float", "rg16unorm", "rg16snorm", "rg16uint", "rg16sint", "rg16float", "rgba8unorm", "rgba8unorm-srgb", "rgba8snorm", "rgba8uint", "rgba8sint", "bgra8unorm", "bgra8unorm-srgb", "rgb9e5ufloat", "rgb10a2uint", "rgb10a2unorm", "rg11b10ufloat", "rg32uint", "rg32sint", "rg32float", "rgba16unorm", "rgba16snorm", "rgba16uint", "rgba16sint", "rgba16float", "rgba32uint", "rgba32sint", "rgba32float", "stencil8", "depth16unorm", "depth24plus", "depth24plus-stencil8", "depth32float", "depth32float-stencil8", "bc1-rgba-unorm", "bc1-rgba-unorm-srgb", "bc2-rgba-unorm", "bc2-rgba-unorm-srgb", "bc3-rgba-unorm", "bc3-rgba-unorm-srgb", "bc4-r-unorm", "bc4-r-snorm", "bc5-rg-unorm", "bc5-rg-snorm", "bc6h-rgb-ufloat", "bc6h-rgb-float", "bc7-rgba-unorm", "bc7-rgba-unorm-srgb", "etc2-rgb8unorm", "etc2-rgb8unorm-srgb", "etc2-rgb8a1unorm", "etc2-rgb8a1unorm-srgb", "etc2-rgba8unorm", "etc2-rgba8unorm-srgb", "eac-r11unorm", "eac-r11snorm", "eac-rg11unorm", "eac-rg11snorm", "astc-4x4-unorm", "astc-4x4-unorm-srgb", "astc-5x4-unorm", "astc-5x4-unorm-srgb", "astc-5x5-unorm", "astc-5x5-unorm-srgb", "astc-6x5-unorm", "astc-6x5-unorm-srgb", "astc-6x6-unorm", "astc-6x6-unorm-srgb", "astc-8x5-unorm", "astc-8x5-unorm-srgb", "astc-8x6-unorm", "astc-8x6-unorm-srgb", "astc-8x8-unorm", "astc-8x8-unorm-srgb", "astc-10x5-unorm", "astc-10x5-unorm-srgb", "astc-10x6-unorm", "astc-10x6-unorm-srgb", "astc-10x8-unorm", "astc-10x8-unorm-srgb", "astc-10x10-unorm", "astc-10x10-unorm-srgb", "astc-12x10-unorm", "astc-12x10-unorm-srgb", "astc-12x12-unorm", "astc-12x12-unorm-srgb"];
const DrawingFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_drawing_free(ptr, 1));
const wbg_rayon_PoolBuilderFinalization = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(ptr => wasm.__wbg_wbg_rayon_poolbuilder_free(ptr, 1));

function addToExternrefTable0(obj) {
    const idx = wasm.__externref_table_alloc();
    wasm.__wbindgen_externrefs.set(idx, obj);
    return idx;
}

const CLOSURE_DTORS = (typeof FinalizationRegistry === 'undefined')
    ? { register: () => {}, unregister: () => {} }
    : new FinalizationRegistry(state => wasm.__wbindgen_destroy_closure(state.a, state.b));

function debugString(val) {
    // primitive types
    const type = typeof val;
    if (type == 'number' || type == 'boolean' || val == null) {
        return  `${val}`;
    }
    if (type == 'string') {
        return `"${val}"`;
    }
    if (type == 'symbol') {
        const description = val.description;
        if (description == null) {
            return 'Symbol';
        } else {
            return `Symbol(${description})`;
        }
    }
    if (type == 'function') {
        const name = val.name;
        if (typeof name == 'string' && name.length > 0) {
            return `Function(${name})`;
        } else {
            return 'Function';
        }
    }
    // objects
    if (Array.isArray(val)) {
        const length = val.length;
        let debug = '[';
        if (length > 0) {
            debug += debugString(val[0]);
        }
        for(let i = 1; i < length; i++) {
            debug += ', ' + debugString(val[i]);
        }
        debug += ']';
        return debug;
    }
    // Test for built-in
    const builtInMatches = /\[object ([^\]]+)\]/.exec(toString.call(val));
    let className;
    if (builtInMatches && builtInMatches.length > 1) {
        className = builtInMatches[1];
    } else {
        // Failed to match the standard '[object ClassName]'
        return toString.call(val);
    }
    if (className == 'Object') {
        // we're a user defined class or Object
        // JSON.stringify avoids problems with cycles, and is generally much
        // easier than looping through ownProperties of `val`.
        try {
            return 'Object(' + JSON.stringify(val) + ')';
        } catch (_) {
            return 'Object';
        }
    }
    // errors
    if (val instanceof Error) {
        return `${val.name}: ${val.message}\n${val.stack}`;
    }
    // TODO we could test for more things here, like `Set`s and `Map`s.
    return className;
}

function getArrayF32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getFloat32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayJsValueViewFromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    const mem = getDataViewMemory0();
    const result = [];
    for (let i = ptr; i < ptr + 4 * len; i += 4) {
        result.push(wasm.__wbindgen_externrefs.get(mem.getUint32(i, true)));
    }
    return result;
}

function getArrayU32FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint32ArrayMemory0().subarray(ptr / 4, ptr / 4 + len);
}

function getArrayU8FromWasm0(ptr, len) {
    ptr = ptr >>> 0;
    return getUint8ArrayMemory0().subarray(ptr / 1, ptr / 1 + len);
}

let cachedDataViewMemory0 = null;
function getDataViewMemory0() {
    if (cachedDataViewMemory0 === null || cachedDataViewMemory0.buffer !== wasm.memory.buffer) {
        cachedDataViewMemory0 = new DataView(wasm.memory.buffer);
    }
    return cachedDataViewMemory0;
}

let cachedFloat32ArrayMemory0 = null;
function getFloat32ArrayMemory0() {
    if (cachedFloat32ArrayMemory0 === null || cachedFloat32ArrayMemory0.buffer !== wasm.memory.buffer) {
        cachedFloat32ArrayMemory0 = new Float32Array(wasm.memory.buffer);
    }
    return cachedFloat32ArrayMemory0;
}

function getStringFromWasm0(ptr, len) {
    return decodeText(ptr >>> 0, len);
}

let cachedUint32ArrayMemory0 = null;
function getUint32ArrayMemory0() {
    if (cachedUint32ArrayMemory0 === null || cachedUint32ArrayMemory0.buffer !== wasm.memory.buffer) {
        cachedUint32ArrayMemory0 = new Uint32Array(wasm.memory.buffer);
    }
    return cachedUint32ArrayMemory0;
}

let cachedUint8ArrayMemory0 = null;
function getUint8ArrayMemory0() {
    if (cachedUint8ArrayMemory0 === null || cachedUint8ArrayMemory0.buffer !== wasm.memory.buffer) {
        cachedUint8ArrayMemory0 = new Uint8Array(wasm.memory.buffer);
    }
    return cachedUint8ArrayMemory0;
}

function handleError(f, args) {
    try {
        return f.apply(this, args);
    } catch (e) {
        const idx = addToExternrefTable0(e);
        wasm.__wbindgen_exn_store(idx);
    }
}

function isLikeNone(x) {
    return x === undefined || x === null;
}

function makeMutClosure(arg0, arg1, f) {
    const state = { a: arg0, b: arg1, cnt: 1 };
    const real = (...args) => {

        // First up with a closure we increment the internal reference
        // count. This ensures that the Rust closure environment won't
        // be deallocated while we're invoking it.
        state.cnt++;
        const a = state.a;
        state.a = 0;
        try {
            return f(a, state.b, ...args);
        } finally {
            state.a = a;
            real._wbg_cb_unref();
        }
    };
    real._wbg_cb_unref = () => {
        if (--state.cnt === 0) {
            wasm.__wbindgen_destroy_closure(state.a, state.b);
            state.a = 0;
            CLOSURE_DTORS.unregister(state);
        }
    };
    CLOSURE_DTORS.register(real, state, state);
    return real;
}

function passArrayF32ToWasm0(arg, malloc) {
    const ptr = malloc(arg.length * 4, 4) >>> 0;
    getFloat32ArrayMemory0().set(arg, ptr / 4);
    WASM_VECTOR_LEN = arg.length;
    return ptr;
}

function passStringToWasm0(arg, malloc, realloc) {
    if (realloc === undefined) {
        const buf = cachedTextEncoder.encode(arg);
        const ptr = malloc(buf.length, 1) >>> 0;
        getUint8ArrayMemory0().subarray(ptr, ptr + buf.length).set(buf);
        WASM_VECTOR_LEN = buf.length;
        return ptr;
    }

    let len = arg.length;
    let ptr = malloc(len, 1) >>> 0;

    const mem = getUint8ArrayMemory0();

    let offset = 0;

    for (; offset < len; offset++) {
        const code = arg.charCodeAt(offset);
        if (code > 0x7F) break;
        mem[ptr + offset] = code;
    }
    if (offset !== len) {
        if (offset !== 0) {
            arg = arg.slice(offset);
        }
        ptr = realloc(ptr, len, len = offset + arg.length * 3, 1) >>> 0;
        const view = getUint8ArrayMemory0().subarray(ptr + offset, ptr + len);
        const ret = cachedTextEncoder.encodeInto(arg, view);

        offset += ret.written;
        ptr = realloc(ptr, len, offset, 1) >>> 0;
    }

    WASM_VECTOR_LEN = offset;
    return ptr;
}

function takeFromExternrefTable0(idx) {
    const value = wasm.__wbindgen_externrefs.get(idx);
    wasm.__externref_table_dealloc(idx);
    return value;
}

let cachedTextDecoder = (typeof TextDecoder !== 'undefined' ? new TextDecoder('utf-8', { ignoreBOM: true, fatal: true }) : undefined);
if (cachedTextDecoder) cachedTextDecoder.decode();

const MAX_SAFARI_DECODE_BYTES = 2146435072;
let numBytesDecoded = 0;
function decodeText(ptr, len) {
    numBytesDecoded += len;
    if (numBytesDecoded >= MAX_SAFARI_DECODE_BYTES) {
        cachedTextDecoder = new TextDecoder('utf-8', { ignoreBOM: true, fatal: true });
        cachedTextDecoder.decode();
        numBytesDecoded = len;
    }
    return cachedTextDecoder.decode(getUint8ArrayMemory0().slice(ptr, ptr + len));
}

const cachedTextEncoder = (typeof TextEncoder !== 'undefined' ? new TextEncoder() : undefined);

if (cachedTextEncoder) {
    cachedTextEncoder.encodeInto = function (arg, view) {
        const buf = cachedTextEncoder.encode(arg);
        view.set(buf);
        return {
            read: arg.length,
            written: buf.length
        };
    };
}

let WASM_VECTOR_LEN = 0;

let wasmModule, wasmInstance, wasm;
function __wbg_finalize_init(instance, module, thread_stack_size) {
    wasmInstance = instance;
    wasm = instance.exports;
    wasmModule = module;
    cachedDataViewMemory0 = null;
    cachedFloat32ArrayMemory0 = null;
    cachedUint32ArrayMemory0 = null;
    cachedUint8ArrayMemory0 = null;
    if (typeof thread_stack_size !== 'undefined' && (typeof thread_stack_size !== 'number' || thread_stack_size === 0 || thread_stack_size % 65536 !== 0)) {
        throw new Error('invalid stack size');
    }

    wasm.__wbindgen_start(thread_stack_size);
    return wasm;
}

async function __wbg_load(module, imports) {
    if (typeof Response === 'function' && module instanceof Response) {
        if (!module.ok) {
            throw new Error(`failed to fetch Wasm: ${module.status} ${module.statusText} fetching '${module.url}'`);
        }

        if (typeof WebAssembly.instantiateStreaming === 'function') {
            try {
                return await WebAssembly.instantiateStreaming(module, imports);
            } catch (e) {
                const validResponse = expectedResponseType(module.type);

                if (validResponse && module.headers.get('Content-Type') !== 'application/wasm') {
                    console.warn("`WebAssembly.instantiateStreaming` failed because your server does not serve Wasm with `application/wasm` MIME type. Falling back to `WebAssembly.instantiate` which is slower. Original error:\n", e);

                } else { throw e; }
            }
        }

        const bytes = await module.arrayBuffer();
        return await WebAssembly.instantiate(bytes, imports);
    } else {
        const instance = await WebAssembly.instantiate(module, imports);

        if (instance instanceof WebAssembly.Instance) {
            return { instance, module };
        } else {
            return instance;
        }
    }

    function expectedResponseType(type) {
        switch (type) {
            case 'basic': case 'cors': case 'default': return true;
        }
        return false;
    }
}

function initSync(module, memory) {
    if (wasm !== undefined) return wasm;

    let thread_stack_size
    if (module !== undefined) {
        if (Object.getPrototypeOf(module) === Object.prototype) {
            ({module, memory, thread_stack_size} = module)
        } else {
            console.warn('using deprecated parameters for `initSync()`; pass a single object instead')
        }
    }

    const imports = __wbg_get_imports(memory);
    if (!(module instanceof WebAssembly.Module)) {
        module = new WebAssembly.Module(module);
    }
    const instance = new WebAssembly.Instance(module, imports);
    return __wbg_finalize_init(instance, module, thread_stack_size);
}

async function __wbg_init(module_or_path, memory) {
    if (wasm !== undefined) return wasm;

    let thread_stack_size
    if (module_or_path !== undefined) {
        if (Object.getPrototypeOf(module_or_path) === Object.prototype) {
            ({module_or_path, memory, thread_stack_size} = module_or_path)
        } else {
            console.warn('using deprecated parameters for the initialization function; pass a single object instead')
        }
    }

    if (module_or_path === undefined) {
        module_or_path = new URL('burin_map_wasm_bg.wasm', import.meta.url);
    }
    const imports = __wbg_get_imports(memory);

    if (typeof module_or_path === 'string' || (typeof Request === 'function' && module_or_path instanceof Request) || (typeof URL === 'function' && module_or_path instanceof URL)) {
        module_or_path = fetch(module_or_path);
    }

    const { instance, module } = await __wbg_load(await module_or_path, imports);

    return __wbg_finalize_init(instance, module, thread_stack_size);
}

export { initSync, __wbg_init as default };
