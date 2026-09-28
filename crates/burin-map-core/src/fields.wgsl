// Compute kernels for `gpu::build_fields_gpu`. Same maths as fields.rs /
// grid.rs, but in f32 throughout, so results differ in the last bits.
// Every kernel sees its own subset of these bindings (auto layout).

struct P {
    w: u32,
    h: u32,
    r: u32,
    mode: u32,
    f0: f32,
    f1: f32,
    f2: f32,
    f3: f32,
    f4: f32,
    f5: f32,
    f6: f32,
    f7: f32,
    f8: f32,
    f9: f32,
    f10: f32,
    f11: f32,
}

@group(0) @binding(0) var<uniform> p: P;
@group(0) @binding(1) var<storage, read_write> b1: array<f32>;
@group(0) @binding(2) var<storage, read_write> b2: array<f32>;
@group(0) @binding(3) var<storage, read_write> b3: array<f32>;
@group(0) @binding(4) var<storage, read_write> b4: array<f32>;
@group(0) @binding(5) var<storage, read_write> b5: array<f32>;
// [0] min bits, [1] max bits, [2] count, [3] percentile result bits
@group(0) @binding(6) var<storage, read_write> stats: array<atomic<u32>>;
@group(0) @binding(7) var<storage, read_write> hist: array<atomic<u32>>;

const BINS: u32 = 65536u;

fn reflect(i: i32, n: i32) -> i32 {
    let period = 2 * n;
    var m = i % period;
    if m < 0 {
        m += period;
    }
    if m >= n {
        return period - 1 - m;
    }
    return m;
}

// Separable convolution, kernel weights in b3 (length 2r+1). Neighbouring
// invocations read neighbouring pixels, so reads coalesce and the cache
// serves the overlap; only edge pixels need reflect.

@compute @workgroup_size(256, 1)
fn conv_h(@builtin(global_invocation_id) g: vec3<u32>) {
    if g.x >= p.w || g.y >= p.h { return; }
    let r = i32(p.r);
    let w = i32(p.w);
    let x = i32(g.x);
    let row = g.y * p.w;
    var acc = 0.0;
    if x >= r && x + r < w {
        let i = row + u32(x - r);
        for (var j = 0u; j <= 2u * p.r; j++) {
            acc += b3[j] * b1[i + j];
        }
    } else {
        for (var j = 0; j <= 2 * r; j++) {
            acc += b3[j] * b1[row + u32(reflect(x + j - r, w))];
        }
    }
    b2[row + g.x] = acc;
}

@compute @workgroup_size(64, 4)
fn conv_v(@builtin(global_invocation_id) g: vec3<u32>) {
    if g.x >= p.w || g.y >= p.h { return; }
    let r = i32(p.r);
    let h = i32(p.h);
    let y = i32(g.y);
    var acc = 0.0;
    if y >= r && y + r < h {
        var i = u32(y - r) * p.w + g.x;
        for (var j = 0; j <= 2 * r; j++) {
            acc += b3[j] * b1[i];
            i += p.w;
        }
    } else {
        for (var j = 0; j <= 2 * r; j++) {
            acc += b3[j] * b1[u32(reflect(y + j - r, h)) * p.w + g.x];
        }
    }
    b2[g.y * p.w + g.x] = acc;
}

// Box filter of radius r, BOX_RUN outputs per invocation from a running
// sum (short, so f32 drift stays negligible).
const BOX_RUN: u32 = 16u;

@compute @workgroup_size(64, 4)
fn box_h(@builtin(global_invocation_id) g: vec3<u32>) {
    let x0 = g.x * BOX_RUN;
    if x0 >= p.w || g.y >= p.h { return; }
    let r = i32(p.r);
    let w = i32(p.w);
    let row = g.y * p.w;
    let norm = 1.0 / f32(2 * r + 1);
    var acc = 0.0;
    for (var j = -r; j <= r; j++) {
        acc += b1[row + u32(reflect(i32(x0) + j, w))];
    }
    let x1 = min(x0 + BOX_RUN, p.w);
    for (var x = x0; x < x1; x++) {
        b2[row + x] = acc * norm;
        acc += b1[row + u32(reflect(i32(x) + r + 1, w))] - b1[row + u32(reflect(i32(x) - r, w))];
    }
}

@compute @workgroup_size(64, 4)
fn box_v(@builtin(global_invocation_id) g: vec3<u32>) {
    let y0 = g.y * BOX_RUN;
    if g.x >= p.w || y0 >= p.h { return; }
    let r = i32(p.r);
    let h = i32(p.h);
    let norm = 1.0 / f32(2 * r + 1);
    var acc = 0.0;
    for (var j = -r; j <= r; j++) {
        acc += b1[u32(reflect(i32(y0) + j, h)) * p.w + g.x];
    }
    let y1 = min(y0 + BOX_RUN, p.h);
    for (var y = y0; y < y1; y++) {
        b2[y * p.w + g.x] = acc * norm;
        let add = u32(reflect(i32(y) + r + 1, h));
        let sub = u32(reflect(i32(y) - r, h));
        acc += b1[add * p.w + g.x] - b1[sub * p.w + g.x];
    }
}

// b2 = b1 / f0; mode 1: b2 = b1 > f1 (as 0/1)
@compute @workgroup_size(16, 16)
fn map1(@builtin(global_invocation_id) g: vec3<u32>) {
    if g.x >= p.w || g.y >= p.h { return; }
    let i = g.y * p.w + g.x;
    if p.mode == 1u {
        b2[i] = select(0.0, 1.0, b1[i] > p.f1);
    } else {
        b2[i] = b1[i] / p.f0;
    }
}

// numpy.gradient of b1 with spacing f0 → b2 = d/dy, b3 = d/dx
@compute @workgroup_size(16, 16)
fn gradient(@builtin(global_invocation_id) g: vec3<u32>) {
    if g.x >= p.w || g.y >= p.h { return; }
    let x = g.x;
    let y = g.y;
    let w = p.w;
    let h = p.h;
    let i = y * w + x;
    var gx = 0.0;
    if w >= 2u {
        if x == 0u {
            gx = b1[i + 1u] - b1[i];
        } else if x == w - 1u {
            gx = b1[i] - b1[i - 1u];
        } else {
            gx = (b1[i + 1u] - b1[i - 1u]) * 0.5;
        }
    }
    var gy = 0.0;
    if h >= 2u {
        if y == 0u {
            gy = b1[i + w] - b1[i];
        } else if y == h - 1u {
            gy = b1[i] - b1[i - w];
        } else {
            gy = (b1[i + w] - b1[i - w]) * 0.5;
        }
    }
    b2[i] = gy / p.f0;
    b3[i] = gx / p.f0;
}

// b1 = gx, b2 = gy → b3 = |g|, b4 = tx, b5 = ty
@compute @workgroup_size(16, 16)
fn direction(@builtin(global_invocation_id) g: vec3<u32>) {
    if g.x >= p.w || g.y >= p.h { return; }
    let i = g.y * p.w + g.x;
    let gx = b1[i];
    let gy = b2[i];
    let m = sqrt(gx * gx + gy * gy);
    b3[i] = m;
    if m > 1e-9 {
        b4[i] = -gy / m;
        b5[i] = gx / m;
    } else {
        b4[i] = 0.0;
        b5[i] = 0.0;
    }
}

// Rust's powf at 0 (WGSL pow is undefined there).
fn powf(x: f32, y: f32) -> f32 {
    if x <= 0.0 {
        return select(0.0, 1.0, y == 0.0);
    }
    return pow(x, y);
}

fn masked_value(i: u32) -> f32 {
    return select(b1[i], abs(b1[i]), p.mode == 1u);
}

// Min / max / count of b1 (or |b1| with mode 1) where b2 > 0.5. Values are
// non-negative, so their bit patterns order like the floats.
var<workgroup> wg_min: atomic<u32>;
var<workgroup> wg_max: atomic<u32>;
var<workgroup> wg_count: atomic<u32>;

@compute @workgroup_size(16, 16)
fn minmax(@builtin(global_invocation_id) g: vec3<u32>, @builtin(local_invocation_index) l: u32) {
    if l == 0u {
        atomicStore(&wg_min, 0xffffffffu);
        atomicStore(&wg_max, 0u);
        atomicStore(&wg_count, 0u);
    }
    workgroupBarrier();
    if g.x < p.w && g.y < p.h {
        let i = g.y * p.w + g.x;
        if b2[i] > 0.5 {
            let v = bitcast<u32>(max(masked_value(i), 0.0));
            atomicMin(&wg_min, v);
            atomicMax(&wg_max, v);
            atomicAdd(&wg_count, 1u);
        }
    }
    workgroupBarrier();
    if l == 0u {
        let c = atomicLoad(&wg_count);
        if c > 0u {
            atomicMin(&stats[0], atomicLoad(&wg_min));
            atomicMax(&stats[1], atomicLoad(&wg_max));
            atomicAdd(&stats[2], c);
        }
    }
}

fn bin_of(x: f32, lo: f32, scale: f32) -> u32 {
    return min(u32(max((x - lo) * scale, 0.0)), BINS - 1u);
}

@compute @workgroup_size(16, 16)
fn histogram(@builtin(global_invocation_id) g: vec3<u32>) {
    if g.x >= p.w || g.y >= p.h { return; }
    let i = g.y * p.w + g.x;
    let lo = bitcast<f32>(atomicLoad(&stats[0]));
    let hi = bitcast<f32>(atomicLoad(&stats[1]));
    if b2[i] > 0.5 && hi > lo {
        atomicAdd(&hist[bin_of(masked_value(i), lo, f32(BINS - 1u) / (hi - lo))], 1u);
    }
}

var<workgroup> scan: array<u32, 256>;

// One workgroup, 256 bins per invocation. Rank table into b3 (mode 0) or
// the f0-th percentile into stats[3] (mode 1).
@compute @workgroup_size(256)
fn hist_scan(@builtin(local_invocation_index) l: u32) {
    let lo = bitcast<f32>(atomicLoad(&stats[0]));
    let hi = bitcast<f32>(atomicLoad(&stats[1]));
    let count = atomicLoad(&stats[2]);
    let b0 = l * 256u;
    var sum = 0u;
    for (var b = b0; b < b0 + 256u; b++) {
        sum += atomicLoad(&hist[b]);
    }
    scan[l] = sum;
    workgroupBarrier();
    if l == 0u {
        var acc = 0u;
        for (var k = 0u; k < 256u; k++) {
            let s = scan[k];
            scan[k] = acc;
            acc += s;
        }
    }
    workgroupBarrier();
    var acc = scan[l];
    if p.mode == 0u {
        for (var b = b0; b < b0 + 256u; b++) {
            let c = atomicLoad(&hist[b]);
            b3[b] = f32(acc) + max(f32(c) - 1.0, 0.0) * 0.5;
            acc += c;
        }
        return;
    }
    if count == 0u || hi <= lo {
        if l == 0u {
            atomicStore(&stats[3], bitcast<u32>(hi));
        }
        return;
    }
    let pos = p.f0 / 100.0 * f32(count - 1u);
    for (var b = b0; b < b0 + 256u; b++) {
        let c = atomicLoad(&hist[b]);
        if c > 0u && f32(acc) <= pos && f32(acc + c) > pos {
            let t = (f32(b) + (pos - f32(acc) + 0.5) / f32(c)) / f32(BINS - 1u);
            atomicStore(&stats[3], bitcast<u32>(lo + (hi - lo) * t));
        }
        acc += c;
    }
}

// Rank of b1 among b2 > 0.5 pixels, from the table in b3 → b4.
@compute @workgroup_size(16, 16)
fn rank(@builtin(global_invocation_id) g: vec3<u32>) {
    if g.x >= p.w || g.y >= p.h { return; }
    let i = g.y * p.w + g.x;
    let lo = bitcast<f32>(atomicLoad(&stats[0]));
    let hi = bitcast<f32>(atomicLoad(&stats[1]));
    let count = atomicLoad(&stats[2]);
    if b2[i] > 0.5 && count >= 2u && hi > lo {
        b4[i] = b3[bin_of(b1[i], lo, f32(BINS - 1u) / (hi - lo))] / f32(count - 1u);
    } else {
        b4[i] = 0.0;
    }
}

var<workgroup> part: array<f32, 256>;

// Per-workgroup sums of b1 (mode 0) or of (b1 - b3[0])² (mode 1) → b2.
@compute @workgroup_size(256)
fn sum_partial(@builtin(global_invocation_id) g: vec3<u32>, @builtin(local_invocation_index) l: u32,
               @builtin(workgroup_id) wg: vec3<u32>, @builtin(num_workgroups) nw: vec3<u32>) {
    let n = p.w * p.h;
    let i = (wg.y * nw.x + wg.x) * 256u + l;
    var v = 0.0;
    if i < n {
        v = b1[i];
        if p.mode == 1u {
            v = (v - b3[0]) * (v - b3[0]);
        }
    }
    part[l] = v;
    workgroupBarrier();
    for (var s = 128u; s > 0u; s >>= 1u) {
        if l < s {
            part[l] += part[l + s];
        }
        workgroupBarrier();
    }
    if l == 0u {
        b2[wg.y * nw.x + wg.x] = part[0];
    }
}

// Sum of the r partials in b2; mode 0 stores the mean in b3[0], mode 1 the
// inverse standard deviation in b3[1].
@compute @workgroup_size(256)
fn sum_final(@builtin(local_invocation_index) l: u32) {
    var v = 0.0;
    for (var i = l; i < p.r; i += 256u) {
        v += b2[i];
    }
    part[l] = v;
    workgroupBarrier();
    for (var s = 128u; s > 0u; s >>= 1u) {
        if l < s {
            part[l] += part[l + s];
        }
        workgroupBarrier();
    }
    if l == 0u {
        let m = part[0] / f32(p.w * p.h);
        if p.mode == 0u {
            b3[0] = m;
        } else {
            b3[1] = 1.0 / max(sqrt(m), 1e-12);
        }
    }
}

// Rotate (b1, b2) by f0 · noise(b3) · inv_std(b5[1]) · (1 - steep(b4))².
@compute @workgroup_size(16, 16)
fn wobble(@builtin(global_invocation_id) g: vec3<u32>) {
    if g.x >= p.w || g.y >= p.h { return; }
    let i = g.y * p.w + g.x;
    let k = 1.0 - b4[i];
    let a = p.f0 * b3[i] * b5[1] * k * k;
    let sa = sin(a);
    let ca = cos(a);
    let u = b1[i];
    let v = b2[i];
    b1[i] = u * ca - v * sa;
    b2[i] = u * sa + v * ca;
}

// Lambert shade from gy = b1, gx = b2 → b3. f0..f2 light, f3 exaggeration.
@compute @workgroup_size(16, 16)
fn shade(@builtin(global_invocation_id) g: vec3<u32>) {
    if g.x >= p.w || g.y >= p.h { return; }
    let i = g.y * p.w + g.x;
    let nx = -b2[i] * p.f3;
    let ny = -b1[i] * p.f3;
    b3[i] = p.f2 - (nx * p.f0 + ny * p.f1 + p.f2) / sqrt(nx * nx + ny * ny + 1.0);
}

// b1 = clamp(b1 / percentile, -1, 1); mode 1 then mixes into b2 by f0.
@compute @workgroup_size(16, 16)
fn shade_norm(@builtin(global_invocation_id) g: vec3<u32>) {
    if g.x >= p.w || g.y >= p.h { return; }
    let i = g.y * p.w + g.x;
    let scale = max(bitcast<f32>(atomicLoad(&stats[3])), 1e-9);
    let s = clamp(b1[i] / scale, -1.0, 1.0);
    if p.mode == 1u {
        b2[i] = (1.0 - p.f0) * b2[i] + p.f0 * s;
    } else {
        b1[i] = s;
    }
}

// shade b1, coast b2, steep b3 → input b4, loosen b5.
@compute @workgroup_size(16, 16)
fn input_space(@builtin(global_invocation_id) g: vec3<u32>) {
    if g.x >= p.w || g.y >= p.h { return; }
    let i = g.y * p.w + g.x;
    let gain = p.f0;
    let dark_gamma = p.f1;
    let light_gain = p.f2;
    let flat = p.f3;
    let light_floor = p.f4;
    let space_steep = p.f5;
    let steep_gamma = p.f6;
    let space_light = p.f7;
    let space_dark = p.f8;
    let si = clamp(b1[i] * gain, -1.0, 1.0);
    let shadow = powf(clamp(si, 0.0, 1.0), dark_gamma);
    let light = clamp(-si * light_gain, 0.0, 1.0);
    var t: f32;
    if si >= 0.0 {
        t = flat + (1.0 - flat) * shadow;
    } else {
        t = light_floor + (flat - light_floor) * (1.0 - light);
    }
    t *= clamp(b2[i] * 1.4 - 0.2, 0.0, 1.0);
    b4[i] = t;
    let steep_k = 1.0 - clamp(space_steep, 0.0, 1.0) * powf(b3[i], steep_gamma);
    let shade_k = space_light + (space_dark - space_light) * t;
    b5[i] = clamp(steep_k * shade_k, 0.05, 4.0);
}

// space b2 = f0 · clamp(b1); walkable b5 = land b3 && mag b4 > 1e-7.
@compute @workgroup_size(16, 16)
fn finish(@builtin(global_invocation_id) g: vec3<u32>) {
    if g.x >= p.w || g.y >= p.h { return; }
    let i = g.y * p.w + g.x;
    b2[i] = p.f0 * clamp(b1[i], 0.05, 4.0);
    b5[i] = select(0.0, 1.0, b3[i] > 0.5 && b4[i] > 1e-7);
}
