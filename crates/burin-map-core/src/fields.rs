//! Raster fields derived from the elevation: line direction, input, space.

use rayon::prelude::*;

use crate::grid::{blur, gradient, percentile, percentile_rank};
use crate::rng::Rng;
use crate::style::Style;

pub struct Fields {
    pub w: usize,
    pub h: usize,
    /// Unit vector along the contours of the smoothed surface.
    pub tx: Vec<f32>,
    pub ty: Vec<f32>,
    /// 0 = draw nothing … 1 = solid line. Feeds space_* and dash_*.
    pub input: Vec<f32>,
    /// Where lines may go (land with a defined direction).
    pub walkable: Vec<bool>,
    /// Local distance between lines, px.
    pub space: Vec<f32>,
}

/// Lambert shading relative to flat ground: 0 flat, +1 deep shadow, -1 fully
/// lit. Scaled by the 98th percentile over land so it doesn't depend on how
/// rugged the area is. `elev_px` is height in pixel units, so slopes are true.
fn relative_shade(
    elev_px: &[f32],
    w: usize,
    h: usize,
    sigma: f32,
    land: &[bool],
    st: &Style,
) -> Vec<f32> {
    let surf = blur(elev_px, w, h, sigma);
    let (sy, sx) = gradient(&surf, w, h, 1.0);
    let (alt, az) = (st.sun_altitude.to_radians(), st.sun_azimuth.to_radians());
    // Image y points south, so north is -y.
    let (lx, ly, lz) = (alt.cos() * az.sin(), -alt.cos() * az.cos(), alt.sin());
    let rel: Vec<f32> = sx
        .par_iter()
        .zip(&sy)
        .map(|(&gx, &gy)| {
            let (nx, ny) = (-gx * st.sun_exag, -gy * st.sun_exag);
            lz - (nx * lx + ny * ly + lz) / (nx * nx + ny * ny + 1.0).sqrt()
        })
        .collect();
    let on_land: Vec<f32> = rel
        .iter()
        .zip(land)
        .filter(|(_, &l)| l)
        .map(|(r, _)| r.abs())
        .collect();
    let scale = percentile(on_land, 98.0).max(1e-9);
    rel.par_iter()
        .map(|r| (r / scale).clamp(-1.0, 1.0))
        .collect()
}

pub fn build_fields(elev: &[f32], w: usize, h: usize, px_m: f32, st: &Style) -> Fields {
    build_fields_with(elev, w, h, px_m, st, None)
}

/// Land: less than half water where the coverage is known, and above sea level
/// where it is not. Height alone misses lakes and floods land below the sea.
pub fn land_mask(elev: &[f32], water: Option<&[f32]>) -> Vec<bool> {
    match water {
        Some(c) => c.par_iter().map(|&c| c < 0.5).collect(),
        None => elev.par_iter().map(|&e| e > 0.5).collect(),
    }
}

/// [`build_fields`] with the water coverage (0…1 per pixel) deciding the land.
pub fn build_fields_with(
    elev: &[f32],
    w: usize,
    h: usize,
    px_m: f32,
    st: &Style,
    water: Option<&[f32]>,
) -> Fields {
    let n = w * h;
    let land = land_mask(elev, water);

    // Direction: gradient of a heavily smoothed surface, turned 90°.
    let surf = blur(elev, w, h, st.mod_smooth);
    let (gy, gx) = gradient(&surf, w, h, px_m);
    let mag: Vec<f32> = gx.par_iter().zip(&gy).map(|(a, b)| a.hypot(*b)).collect();
    let mut tx = vec![0.0f32; n];
    let mut ty = vec![0.0f32; n];
    tx.par_iter_mut()
        .zip(&mut ty)
        .enumerate()
        .for_each(|(i, (tx, ty))| {
            if mag[i] > 1e-9 {
                *tx = -gy[i] / mag[i];
                *ty = gx[i] / mag[i];
            }
        });
    let steep = percentile_rank(&mag, &land);

    // On near-flat ground a smooth random rotation, fading to none on slopes,
    // keeps lines from coming out combed-parallel.
    wobble(&mut tx, &mut ty, w, h, st, |i| (1.0 - steep[i]).powi(2));

    // Shading at two scales: detail keeps ridges readable up close, the
    // massif scale gives whole mountain sides one value from afar.
    let elev_px: Vec<f32> = elev.par_iter().map(|e| e / px_m).collect();
    let mut s = relative_shade(&elev_px, w, h, st.input_blur, &land, st);
    if st.input_broad > 0.0 {
        let b = relative_shade(&elev_px, w, h, st.input_broad_blur, &land, st);
        s.par_iter_mut().zip(&b).for_each(|(s, &b)| {
            *s = (1.0 - st.input_broad) * *s + st.input_broad * b;
        });
    }

    // Flat ground at input_flat; dark raises it towards solid, light lowers
    // it towards input_light_floor. Space lerps space_light → space_dark by input.
    let coast = blur(
        &land.par_iter().map(|&l| l as u8 as f32).collect::<Vec<_>>(),
        w,
        h,
        2.0,
    );
    let mut input = vec![0.0f32; n];
    let mut loosen = vec![0.0f32; n];
    input
        .par_iter_mut()
        .zip(&mut loosen)
        .enumerate()
        .for_each(|(i, (input, loosen))| {
            let si = (s[i] * st.input_gain).clamp(-1.0, 1.0);
            let shadow = si.clamp(0.0, 1.0).powf(st.input_dark_gamma);
            let light = (-si * st.input_light_gain).clamp(0.0, 1.0);
            let t = if si >= 0.0 {
                st.input_flat + (1.0 - st.input_flat) * shadow
            } else {
                st.input_light_floor + (st.input_flat - st.input_light_floor) * (1.0 - light)
            };
            let t = t * (coast[i] * 1.4 - 0.2).clamp(0.0, 1.0);
            *input = t;
            // space_steep: 0 = off, 1 = max densification on steep ground
            let steep_k =
                1.0 - st.space_steep.clamp(0.0, 1.0) * steep[i].powf(st.space_steep_gamma);
            let shade_k = st.space_light + (st.space_dark - st.space_light) * t;
            *loosen = (steep_k * shade_k).clamp(0.05, 4.0);
        });
    let space: Vec<f32> = blur(&loosen, w, h, 3.0)
        .par_iter()
        .map(|k| st.space * k.clamp(0.05, 4.0))
        .collect();
    let walkable: Vec<bool> = (0..n)
        .into_par_iter()
        .map(|i| land[i] && mag[i] > 1e-7)
        .collect();

    Fields {
        w,
        h,
        tx,
        ty,
        input,
        walkable,
        space,
    }
}

/// Turn the direction field by a smooth random angle of up to about
/// `mod_wobble` degrees, times `weight(i)` at each pixel.
pub(crate) fn wobble(
    tx: &mut [f32],
    ty: &mut [f32],
    w: usize,
    h: usize,
    st: &Style,
    weight: impl Fn(usize) -> f32 + Sync,
) {
    if st.mod_wobble <= 0.0 {
        return;
    }
    let n = w * h;
    let mut noise = vec![0.0f32; n];
    Rng::new(st.batch_seed).fill_normal(&mut noise);
    let noise = blur(&noise, w, h, st.mod_wobble_size);
    let mean = noise.iter().map(|&v| v as f64).sum::<f64>() / n as f64;
    let var = noise
        .iter()
        .map(|&v| (v as f64 - mean).powi(2))
        .sum::<f64>()
        / n as f64;
    let inv_std = 1.0 / (var.sqrt() as f32).max(1e-12);
    let amp = st.mod_wobble.to_radians();
    tx.par_iter_mut()
        .zip(ty)
        .enumerate()
        .for_each(|(i, (tx, ty))| {
            let a = amp * noise[i] * inv_std * weight(i);
            let (sa, ca) = a.sin_cos();
            let (u, v) = (*tx, *ty);
            *tx = u * ca - v * sa;
            *ty = u * sa + v * ca;
        });
}

/// Fields for one batch: rotate the contour direction by `batch.angle` degrees,
/// rescale pitch to `batch.style.space`, and optionally gate on `batch_floor`.
pub fn batch_fields(base: &Fields, batch: &crate::style::Batch, base_space: f32) -> Fields {
    let a = batch.angle.to_radians();
    let (sa, ca) = a.sin_cos();
    let pitch = batch.style.space / base_space.max(1e-6);
    let floor = batch.style.batch_floor;
    let (tx, ty): (Vec<f32>, Vec<f32>) = base
        .tx
        .par_iter()
        .zip(&base.ty)
        .map(|(&u, &v)| (u * ca - v * sa, u * sa + v * ca))
        .unzip();
    let walkable: Vec<bool> = base
        .walkable
        .par_iter()
        .zip(&base.input)
        .map(|(&w, &t)| w && (floor <= 0.0 || t >= floor))
        .collect();
    let space: Vec<f32> = base.space.par_iter().map(|&s| s * pitch).collect();
    Fields {
        w: base.w,
        h: base.h,
        tx,
        ty,
        input: base.input.clone(),
        walkable,
        space,
    }
}
