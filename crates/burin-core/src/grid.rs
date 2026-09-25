//! Raster helpers: the few numpy/scipy operations the pipeline needs.
//! Grids are row-major `Vec<f32>` of `w × h`, row 0 at the top (north).

/// scipy.ndimage "reflect" boundary: d c b a | a b c d | d c b a
#[inline]
fn reflect(i: isize, n: isize) -> usize {
    let period = 2 * n;
    let m = i.rem_euclid(period);
    (if m >= n { period - 1 - m } else { m }) as usize
}

/// Gaussian blur, reflect boundary. Small sigmas use an exact kernel
/// (truncated at 4σ like scipy); larger ones three box passes, whose cost
/// doesn't grow with sigma and whose result is visually the same.
pub fn blur(src: &[f32], w: usize, h: usize, sigma: f32) -> Vec<f32> {
    if sigma <= 0.0 {
        return src.to_vec();
    }
    let mut a = src.to_vec();
    let mut b = vec![0.0f32; src.len()];
    if sigma < 3.0 {
        let k = gaussian_kernel(sigma);
        for y in 0..h {
            convolve(&a[y * w..(y + 1) * w], &mut b[y * w..(y + 1) * w], &k);
        }
        convolve_rows(&b, &mut a, w, h, &k);
    } else {
        for r in box_radii(sigma, 3) {
            for y in 0..h {
                box_line(&a[y * w..(y + 1) * w], &mut b[y * w..(y + 1) * w], r);
            }
            box_rows(&b, &mut a, w, h, r);
        }
    }
    a
}

/// Vertical convolution, row by row so memory is read in order.
fn convolve_rows(src: &[f32], dst: &mut [f32], w: usize, h: usize, k: &[f32]) {
    let r = (k.len() / 2) as isize;
    for y in 0..h {
        let out = &mut dst[y * w..(y + 1) * w];
        out.iter_mut().for_each(|v| *v = 0.0);
        for (j, &kv) in k.iter().enumerate() {
            let sy = reflect(y as isize + j as isize - r, h as isize);
            for (o, &s) in out.iter_mut().zip(&src[sy * w..(sy + 1) * w]) {
                *o += kv * s;
            }
        }
    }
}

/// Vertical box filter with a running row sum.
fn box_rows(src: &[f32], dst: &mut [f32], w: usize, h: usize, r: usize) {
    let (n, r) = (h as isize, r as isize);
    let norm = 1.0 / (2 * r + 1) as f64;
    let row = |y: isize| &src[reflect(y, n) * w..(reflect(y, n) + 1) * w];
    let mut acc = vec![0.0f64; w];
    for y in -r..=r {
        for (a, &s) in acc.iter_mut().zip(row(y)) {
            *a += s as f64;
        }
    }
    for y in 0..n {
        for (o, &a) in dst[y as usize * w..(y as usize + 1) * w].iter_mut().zip(&acc) {
            *o = (a * norm) as f32;
        }
        let (add, sub) = (row(y + r + 1), row(y - r));
        for ((a, &p), &m) in acc.iter_mut().zip(add).zip(sub) {
            *a += p as f64 - m as f64;
        }
    }
}

fn gaussian_kernel(sigma: f32) -> Vec<f32> {
    let r = (4.0 * sigma + 0.5) as isize;
    let mut k: Vec<f32> = (-r..=r)
        .map(|i| (-(i * i) as f32 / (2.0 * sigma * sigma)).exp())
        .collect();
    let s: f32 = k.iter().sum();
    k.iter_mut().for_each(|v| *v /= s);
    k
}

/// Radii of `n` box filters whose succession approximates a gaussian of `sigma`.
fn box_radii(sigma: f32, n: usize) -> Vec<usize> {
    let s2 = (sigma * sigma) as f64;
    let nf = n as f64;
    let w_ideal = (12.0 * s2 / nf + 1.0).sqrt();
    let mut wl = w_ideal.floor() as i64;
    if wl % 2 == 0 {
        wl -= 1;
    }
    let wu = wl + 2;
    let m_ideal = (12.0 * s2 - nf * (wl * wl) as f64 - 4.0 * nf * wl as f64 - 3.0 * nf)
        / (-4.0 * wl as f64 - 4.0);
    let m = m_ideal.round() as usize;
    (0..n).map(|i| (((if i < m { wl } else { wu }) - 1) / 2) as usize).collect()
}

fn convolve(line: &[f32], out: &mut [f32], k: &[f32]) {
    let n = line.len() as isize;
    let r = (k.len() / 2) as isize;
    for i in 0..n {
        let mut acc = 0.0f32;
        for (j, kv) in k.iter().enumerate() {
            acc += kv * line[reflect(i + j as isize - r, n)];
        }
        out[i as usize] = acc;
    }
}

fn box_line(line: &[f32], out: &mut [f32], r: usize) {
    let n = line.len() as isize;
    let r = r as isize;
    let norm = 1.0 / (2 * r + 1) as f64;
    let mut acc: f64 = (-r..=r).map(|i| line[reflect(i, n)] as f64).sum();
    for i in 0..n {
        out[i as usize] = (acc * norm) as f32;
        acc += line[reflect(i + r + 1, n)] as f64 - line[reflect(i - r, n)] as f64;
    }
}

/// numpy.gradient: central differences inside, one-sided at the edges.
/// Returns (d/dy, d/dx).
pub fn gradient(f: &[f32], w: usize, h: usize, spacing: f32) -> (Vec<f32>, Vec<f32>) {
    let mut gy = vec![0.0f32; f.len()];
    let mut gx = vec![0.0f32; f.len()];
    for y in 0..h {
        for x in 0..w {
            let i = y * w + x;
            gx[i] = if w < 2 {
                0.0
            } else if x == 0 {
                f[i + 1] - f[i]
            } else if x == w - 1 {
                f[i] - f[i - 1]
            } else {
                (f[i + 1] - f[i - 1]) * 0.5
            } / spacing;
            gy[i] = if h < 2 {
                0.0
            } else if y == 0 {
                f[i + w] - f[i]
            } else if y == h - 1 {
                f[i] - f[i - w]
            } else {
                (f[i + w] - f[i - w]) * 0.5
            } / spacing;
        }
    }
    (gy, gx)
}

/// Rank of each value among `mask` pixels, in [0, 1]; 0 outside the mask.
/// From a fine histogram rather than a sort: linear time, and the error
/// (< 1/65536 of the value range) is far below anything visible.
pub fn percentile_rank(v: &[f32], mask: &[bool]) -> Vec<f32> {
    const BINS: usize = 65536;
    let (mut lo, mut hi) = (f32::INFINITY, f32::NEG_INFINITY);
    let mut count = 0usize;
    for (&x, &m) in v.iter().zip(mask) {
        if m {
            lo = lo.min(x);
            hi = hi.max(x);
            count += 1;
        }
    }
    let mut out = vec![0.0f32; v.len()];
    if count < 2 || hi <= lo {
        return out;
    }
    let scale = (BINS - 1) as f32 / (hi - lo);
    let bin = |x: f32| (((x - lo) * scale) as usize).min(BINS - 1);
    let mut hist = vec![0u32; BINS];
    for (&x, &m) in v.iter().zip(mask) {
        if m {
            hist[bin(x)] += 1;
        }
    }
    // Rank of a value = values in lower bins + half of its own bin.
    let mut below = vec![0.0f32; BINS];
    let mut acc = 0u64;
    for b in 0..BINS {
        below[b] = (acc as f64 + (hist[b] as f64 - 1.0).max(0.0) * 0.5) as f32;
        acc += hist[b] as u64;
    }
    let denom = (count - 1) as f32;
    for ((o, &x), &m) in out.iter_mut().zip(v).zip(mask) {
        if m {
            *o = below[bin(x)] / denom;
        }
    }
    out
}

/// q-th percentile (0–100) with linear interpolation, like numpy's default.
pub fn percentile(mut values: Vec<f32>, q: f32) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let pos = q / 100.0 * (values.len() - 1) as f32;
    let lo = pos.floor() as usize;
    let hi = (lo + 1).min(values.len() - 1);
    let (_, &mut a, rest) = values.select_nth_unstable_by(lo, |a, b| a.total_cmp(b));
    let b = if hi == lo { a } else { rest.iter().copied().fold(f32::INFINITY, f32::min) };
    a + (b - a) * (pos - lo as f32)
}
