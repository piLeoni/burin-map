//! Raster fields derived from the elevation: line direction, tone, spacing.
//! Mirrors `src/burin/fields.py`.

use crate::grid::{blur, gradient, percentile, percentile_rank};
use crate::rng::Rng;
use crate::style::Style;

pub struct Fields {
    pub w: usize,
    pub h: usize,
    /// Unit vector along the contours of the smoothed surface.
    pub tx: Vec<f32>,
    pub ty: Vec<f32>,
    /// 0 = draw nothing … 1 = solid line.
    pub tone: Vec<f32>,
    /// Where lines may go (land with a defined direction).
    pub walkable: Vec<bool>,
    /// Local line spacing, px.
    pub sep: Vec<f32>,
}

/// Lambert shading relative to flat ground: 0 flat, +1 deep shadow, -1 fully
/// lit. Scaled by the 98th percentile over land so it doesn't depend on how
/// rugged the area is. `elev_px` is height in pixel units, so slopes are true.
fn relative_shade(elev_px: &[f32], w: usize, h: usize, sigma: f32, land: &[bool], st: &Style) -> Vec<f32> {
    let surf = blur(elev_px, w, h, sigma);
    let (sy, sx) = gradient(&surf, w, h, 1.0);
    let (alt, az) = (st.altitude.to_radians(), st.azimuth.to_radians());
    // Image y points south, so north is -y.
    let (lx, ly, lz) = (alt.cos() * az.sin(), -alt.cos() * az.cos(), alt.sin());
    let rel: Vec<f32> = sx
        .iter()
        .zip(&sy)
        .map(|(&gx, &gy)| {
            let (nx, ny) = (-gx * st.z_exag, -gy * st.z_exag);
            lz - (nx * lx + ny * ly + lz) / (nx * nx + ny * ny + 1.0).sqrt()
        })
        .collect();
    let on_land: Vec<f32> = rel.iter().zip(land).filter(|(_, &l)| l).map(|(r, _)| r.abs()).collect();
    let scale = percentile(on_land, 98.0).max(1e-9);
    rel.iter().map(|r| (r / scale).clamp(-1.0, 1.0)).collect()
}

pub fn build_fields(elev: &[f32], w: usize, h: usize, px_m: f32, st: &Style) -> Fields {
    let n = w * h;
    let land: Vec<bool> = elev.iter().map(|&e| e > 0.5).collect();

    // Direction: gradient of a heavily smoothed surface, turned 90°.
    let surf = blur(elev, w, h, st.flow_blur);
    let (gy, gx) = gradient(&surf, w, h, px_m);
    let mag: Vec<f32> = gx.iter().zip(&gy).map(|(a, b)| a.hypot(*b)).collect();
    let mut tx = vec![0.0f32; n];
    let mut ty = vec![0.0f32; n];
    for i in 0..n {
        if mag[i] > 1e-9 {
            tx[i] = -gy[i] / mag[i];
            ty[i] = gx[i] / mag[i];
        }
    }
    let steep = percentile_rank(&mag, &land);

    // On near-flat ground a smooth random rotation, fading to none on slopes,
    // keeps lines from coming out combed-parallel.
    if st.wobble > 0.0 {
        let mut rng = Rng::new(st.seed);
        let noise: Vec<f32> = (0..n).map(|_| rng.normal() as f32).collect();
        let noise = blur(&noise, w, h, st.wobble_size);
        let mean = noise.iter().map(|&v| v as f64).sum::<f64>() / n as f64;
        let var = noise.iter().map(|&v| (v as f64 - mean).powi(2)).sum::<f64>() / n as f64;
        let inv_std = 1.0 / (var.sqrt() as f32).max(1e-12);
        let amp = st.wobble.to_radians();
        for i in 0..n {
            let a = amp * noise[i] * inv_std * (1.0 - steep[i]).powi(2);
            let (sa, ca) = a.sin_cos();
            let (u, v) = (tx[i], ty[i]);
            tx[i] = u * ca - v * sa;
            ty[i] = u * sa + v * ca;
        }
    }

    // Shading at two scales: detail keeps ridges readable up close, the
    // massif scale gives whole mountain sides one tone from afar.
    let elev_px: Vec<f32> = elev.iter().map(|e| e / px_m).collect();
    let mut s = relative_shade(&elev_px, w, h, st.shade_blur, &land, st);
    if st.broad_weight > 0.0 {
        let b = relative_shade(&elev_px, w, h, st.broad_blur, &land, st);
        for i in 0..n {
            s[i] = (1.0 - st.broad_weight) * s[i] + st.broad_weight * b[i];
        }
    }

    // Flat ground at flat_level; shadow raises it towards solid, light lowers
    // it towards lit_floor.
    let coast = blur(&land.iter().map(|&l| l as u8 as f32).collect::<Vec<_>>(), w, h, 2.0);
    let mut tone = vec![0.0f32; n];
    let mut loosen = vec![0.0f32; n];
    for i in 0..n {
        let si = (s[i] * st.shade_gain).clamp(-1.0, 1.0);
        let shadow = si.clamp(0.0, 1.0).powf(st.dark_gamma);
        let light = (-si * st.lit_gain).clamp(0.0, 1.0);
        let t = if si >= 0.0 {
            st.flat_level + (1.0 - st.flat_level) * shadow
        } else {
            st.lit_floor + (st.flat_level - st.lit_floor) * (1.0 - light)
        };
        tone[i] = t * (coast[i] * 1.4 - 0.2).clamp(0.0, 1.0);
        // Spacing: steepness always; light/shadow scaled by shade_gap
        // (shade_dash is the parallel channel — breaks lines instead).
        let g = st.shade_gap.clamp(0.0, 1.0);
        let steep_k = 1.0 - (1.0 - st.spacing_steep) * steep[i].powf(st.spacing_gamma);
        let dark_k = 1.0 - g * (1.0 - st.spacing_dark) * shadow;
        let lit_w = g * light;
        let lit_k = (1.0 - lit_w) + st.spacing_lit * lit_w;
        let keep = (steep_k * dark_k * lit_k).clamp(0.05, 4.0);
        loosen[i] = keep; // (misnamed) holds the local spacing multiplier
    }
    let sep: Vec<f32> = blur(&loosen, w, h, 3.0)
        .iter()
        .map(|k| st.spacing * k.clamp(0.05, 4.0))
        .collect();
    let walkable: Vec<bool> = (0..n).map(|i| land[i] && mag[i] > 1e-7).collect();

    Fields { w, h, tx, ty, tone, walkable, sep }
}

/// Fields for one hatch: rotate the contour direction by `hatch.angle` degrees,
/// rescale pitch to `hatch.style.spacing`, and optionally gate on `hatch_floor`.
pub fn hatch_fields(base: &Fields, hatch: &crate::style::Hatch, base_spacing: f32) -> Fields {
    let n = base.w * base.h;
    let a = hatch.angle.to_radians();
    let (sa, ca) = a.sin_cos();
    let pitch = hatch.style.spacing / base_spacing.max(1e-6);
    let floor = hatch.style.hatch_floor;
    let mut tx = vec![0.0f32; n];
    let mut ty = vec![0.0f32; n];
    let mut walkable = vec![false; n];
    let mut sep = vec![0.0f32; n];
    for i in 0..n {
        let (u, v) = (base.tx[i], base.ty[i]);
        tx[i] = u * ca - v * sa;
        ty[i] = u * sa + v * ca;
        let ok = base.walkable[i] && (floor <= 0.0 || base.tone[i] >= floor);
        walkable[i] = ok;
        sep[i] = base.sep[i] * pitch;
    }
    Fields {
        w: base.w,
        h: base.h,
        tx,
        ty,
        tone: base.tone.clone(),
        walkable,
        sep,
    }
}

