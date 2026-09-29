//! Waves along a line, as a hand draws them: each line swings `mod_wave` px
//! to either side, about once every `mod_wave_size` px, and both drift along
//! the way, so no two lines wave alike. With `mod_wave_zone` neighbouring
//! lines wave together instead, so parallel lines keep their spacing; with
//! `mod_wave_even` they wave evenly and in step along the flow, each line an
//! offset copy of the next, as in an engraved pattern.

use rayon::prelude::*;

use crate::rng::Rng;
use crate::style::Style;
use crate::Line;

/// `lines`, each waving along its own path. Unchanged with `mod_wave` 0.
/// `mod_wave_even` needs the flow's `phase`; without it, it does nothing.
pub fn undulate(lines: Vec<Line>, st: &Style, seed: u64, phase: Option<&Phase>) -> Vec<Line> {
    if st.mod_wave <= 0.0 {
        return lines;
    }
    lines
        .into_par_iter()
        .enumerate()
        .map(|(i, l)| {
            let mut rng =
                Rng::new(seed.wrapping_add(0x3A5E ^ (i as u64).wrapping_mul(0x9E37_79B9)));
            wave(&l, st, seed, phase, &mut rng)
        })
        .collect()
}

fn wave(l: &Line, st: &Style, seed: u64, phase: Option<&Phase>, rng: &mut Rng) -> Line {
    let (amp, size) = (st.mod_wave, st.mod_wave_size.max(2.0));
    let together = st.mod_wave_zone.clamp(0.0, 1.0);
    let even = if phase.is_some() {
        st.mod_wave_even.clamp(0.0, 1.0)
    } else {
        0.0
    };
    if l.len() < 2 {
        return l.clone();
    }
    let p = crate::water::resample(l, (size / 10.0).clamp(1.0, 4.0));
    let n = p.len();
    let mut s = vec![0.0f32];
    for q in p.windows(2) {
        s.push(s.last().unwrap() + (q[1][0] - q[0][0]).hypot(q[1][1] - q[0][1]));
    }
    let total = *s.last().unwrap();
    if total < size * 0.25 {
        return l.clone();
    }
    // How strongly and how long the waves are drifts slowly along the line.
    let knots = ((total / (size * 1.5)).ceil() as usize).max(1) + 1;
    let strong: Vec<f32> = (0..=knots).map(|_| rng.uniform(0.1, 1.3) as f32).collect();
    let long: Vec<f32> = (0..=knots).map(|_| rng.uniform(0.75, 1.3) as f32).collect();
    let drift = |v: &[f32], s: f32| {
        let x = s / (size * 1.5);
        let j = (x as usize).min(knots - 1);
        let f = (x - j as f32).clamp(0.0, 1.0);
        let f = f * f * (3.0 - 2.0 * f);
        v[j] + (v[j + 1] - v[j]) * f
    };
    // Shared swings push every line the same way: to its left when it runs
    // with the flow (or, without one, eastwards), to its right against it.
    let way = match phase {
        Some(ph) if ph.at(p[n - 1]) < ph.at(p[0]) => -1.0,
        Some(_) => 1.0,
        None if (p[n - 1][0] - p[0][0], p[n - 1][1] - p[0][1]) < (0.0, 0.0) => -1.0,
        None => 1.0,
    };
    let shift = (seed % 6283) as f32 / 1000.0;
    let mut turn = rng.uniform(0.0, std::f64::consts::TAU) as f32;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        if i > 0 {
            turn += std::f32::consts::TAU * (s[i] - s[i - 1]) / (size * drift(&long, s[i]));
        }
        let (a, b) = (p[i.saturating_sub(1)], p[(i + 1).min(n - 1)]);
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let m = dx.hypot(dy);
        if m < 1e-6 {
            out.push(p[i]);
            continue;
        }
        // Ends ease in, so a line starts and stops where it was traced.
        let ease = (s[i].min(total - s[i]) / (size * 0.5)).clamp(0.0, 1.0);
        let own = drift(&strong, s[i]) * ease * turn.sin();
        let shared = zone(p[i], size, seed) * way;
        let hand = own * (1.0 - together) + shared * together;
        let step = phase.map_or(0.0, |ph| {
            (std::f32::consts::TAU * ph.at(p[i]) / size + shift).sin() * way
        });
        let d = amp * (hand * (1.0 - even) + step * even);
        out.push([p[i][0] - dy / m * d, p[i][1] + dx / m * d]);
    }
    out
}

/// The sideways swing, -1…1 or a little over, that every line through `p`
/// shares: smooth noise with bumps about `size` px apart, and smaller ones
/// on top so the lines curl. Lines well under `size` / 4 px apart keep
/// their spacing.
fn zone(p: [f32; 2], size: f32, seed: u64) -> f32 {
    let coarse = value_noise(p[0] / (size * 0.5), p[1] / (size * 0.5), seed);
    let fine = value_noise(p[0] / (size * 0.3), p[1] / (size * 0.3), seed ^ 0xF1);
    (coarse * 0.75 + fine * 0.25 - 0.5) * 2.5
}

/// How far along the flow each point is, px: a field whose steps follow the
/// flow as closely as it can, so points level with each other across
/// neighbouring lines share a value. Where the flow curls no such field
/// exists, and it settles for the nearest.
pub struct Phase {
    w: usize,
    h: usize,
    cell: f32,
    v: Vec<f32>,
}

/// Pixels per cell of the finest grid: the flow bends over tens of px.
const PHASE_CELL: usize = 4;

impl Phase {
    /// From a flow of unit vectors, `w` × `h` px (zero where there is none).
    pub fn from_flow(tx: &[f32], ty: &[f32], w: usize, h: usize) -> Phase {
        // A pyramid of the flow, averaged, from PHASE_CELL px cells upwards.
        let mut levels = vec![shrink(tx, ty, w, h, PHASE_CELL, PHASE_CELL as f32)];
        while levels.last().unwrap().w.min(levels.last().unwrap().h) > 8 {
            let l = levels.last().unwrap();
            levels.push(shrink(&l.tx, &l.ty, l.w, l.h, 2, l.cell * 2.0));
        }
        // Solved coarse to fine, each level starting from the one below.
        let mut v: Vec<f32> = Vec::new();
        let mut from: Option<&Level> = None;
        for l in levels.iter().rev() {
            v = match from {
                None => vec![0.0; l.w * l.h],
                Some(c) => (0..l.w * l.h)
                    .map(|i| {
                        let (x, y) = ((i % l.w) as f32, (i / l.w) as f32);
                        sample(&v, c.w, c.h, (x + 0.5) / 2.0 - 0.5, (y + 0.5) / 2.0 - 0.5)
                    })
                    .collect(),
            };
            relax(&mut v, l, if from.is_none() { 200 } else { 40 });
            from = Some(l);
        }
        let l = &levels[0];
        Phase {
            w: l.w,
            h: l.h,
            cell: l.cell,
            v,
        }
    }

    /// The value at `p`, px.
    pub fn at(&self, p: [f32; 2]) -> f32 {
        sample(
            &self.v,
            self.w,
            self.h,
            p[0] / self.cell - 0.5,
            p[1] / self.cell - 0.5,
        )
    }
}

struct Level {
    w: usize,
    h: usize,
    cell: f32,
    tx: Vec<f32>,
    ty: Vec<f32>,
}

fn shrink(tx: &[f32], ty: &[f32], w: usize, h: usize, k: usize, cell: f32) -> Level {
    let (nw, nh) = (w.div_ceil(k), h.div_ceil(k));
    let mut out = Level {
        w: nw,
        h: nh,
        cell,
        tx: vec![0.0; nw * nh],
        ty: vec![0.0; nw * nh],
    };
    for y in 0..h {
        for x in 0..w {
            let j = (y / k) * nw + x / k;
            out.tx[j] += tx[y * w + x];
            out.ty[j] += ty[y * w + x];
        }
    }
    let n = (k * k) as f32;
    for j in 0..nw * nh {
        out.tx[j] /= n;
        out.ty[j] /= n;
    }
    out
}

/// Over-relaxed Gauss-Seidel on "each step between cells is the flow along
/// it, times the cell size".
fn relax(v: &mut [f32], l: &Level, sweeps: usize) {
    let (w, h, c) = (l.w, l.h, l.cell);
    for _ in 0..sweeps {
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let (mut sum, mut n) = (0.0, 0.0);
                if x + 1 < w {
                    sum += v[i + 1] - c * (l.tx[i] + l.tx[i + 1]) * 0.5;
                    n += 1.0;
                }
                if x > 0 {
                    sum += v[i - 1] + c * (l.tx[i] + l.tx[i - 1]) * 0.5;
                    n += 1.0;
                }
                if y + 1 < h {
                    sum += v[i + w] - c * (l.ty[i] + l.ty[i + w]) * 0.5;
                    n += 1.0;
                }
                if y > 0 {
                    sum += v[i - w] + c * (l.ty[i] + l.ty[i - w]) * 0.5;
                    n += 1.0;
                }
                if n > 0.0 {
                    v[i] += 1.8 * (sum / n - v[i]);
                }
            }
        }
    }
}

fn sample(v: &[f32], w: usize, h: usize, x: f32, y: f32) -> f32 {
    let x = x.clamp(0.0, (w - 1) as f32);
    let y = y.clamp(0.0, (h - 1) as f32);
    let (i, j) = (x as usize, y as usize);
    let (i1, j1) = ((i + 1).min(w - 1), (j + 1).min(h - 1));
    let (fx, fy) = (x - i as f32, y - j as f32);
    let top = v[j * w + i] + (v[j * w + i1] - v[j * w + i]) * fx;
    let bottom = v[j1 * w + i] + (v[j1 * w + i1] - v[j1 * w + i]) * fx;
    top + (bottom - top) * fy
}

/// Smooth noise, 0…1, on a unit lattice that goes on forever.
fn value_noise(x: f32, y: f32, seed: u64) -> f32 {
    let (i, j) = (x.floor(), y.floor());
    let e = |t: f32| t * t * (3.0 - 2.0 * t);
    let (fx, fy) = (e(x - i), e(y - j));
    let v = |a: f32, b: f32| lattice(a as i64, b as i64, seed);
    let top = v(i, j) + (v(i + 1.0, j) - v(i, j)) * fx;
    let bottom = v(i, j + 1.0) + (v(i + 1.0, j + 1.0) - v(i, j + 1.0)) * fx;
    top + (bottom - top) * fy
}

fn lattice(i: i64, j: i64, seed: u64) -> f32 {
    let mut z = seed
        ^ (i as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15)
        ^ (j as u64).wrapping_mul(0xC2B2_AE3D_27D4_EB4F);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^= z >> 31;
    (z >> 40) as f32 / (1u64 << 24) as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_wave_sideways_but_stay_near() {
        let straight: Line = (0..=200).map(|x| [x as f32, 50.0]).collect();
        let st = Style {
            mod_wave: 3.0,
            mod_wave_size: 20.0,
            ..Style::default()
        };
        let out = undulate(vec![straight.clone(), straight.clone()], &st, 1, None);
        let off = |l: &Line| l.iter().map(|p| (p[1] - 50.0).abs()).fold(0.0f32, f32::max);
        assert!(off(&out[0]) > 1.5, "it waves");
        assert!(off(&out[0]) <= 3.0 * 1.3 + 1e-3, "but not far");
        assert_ne!(out[0], out[1], "no two lines alike");
        let still = undulate(vec![straight.clone()], &Style::default(), 1, None);
        assert_eq!(still[0], straight, "off by default");
    }

    #[test]
    fn lines_in_a_zone_wave_together() {
        let row = |y: f32| -> Line { (0..=300).map(|x| [x as f32, y]).collect() };
        let st = Style {
            mod_wave: 3.0,
            mod_wave_size: 60.0,
            mod_wave_zone: 1.0,
            ..Style::default()
        };
        let out = undulate(vec![row(50.0), row(54.0)], &st, 7, None);
        let gaps: Vec<f32> = out[0]
            .iter()
            .zip(&out[1])
            .map(|(a, b)| b[1] - a[1])
            .collect();
        let swing = out[0]
            .iter()
            .map(|p| (p[1] - 50.0).abs())
            .fold(0.0f32, f32::max);
        assert!(swing > 2.0, "they wave");
        assert!(
            gaps.iter().all(|g| (g - 4.0).abs() < 1.5),
            "and keep their spacing"
        );
    }

    #[test]
    fn even_waves_keep_step_along_the_flow() {
        // A flow running east: rows traced either way still wave as one.
        let (w, h) = (300, 100);
        let phase = Phase::from_flow(&vec![1.0; w * h], &vec![0.0; w * h], w, h);
        let row = |y: f32| -> Line { (20..=280).map(|x| [x as f32, y]).collect() };
        let mut back = row(55.0);
        back.reverse();
        let st = Style {
            mod_wave: 3.0,
            mod_wave_size: 40.0,
            mod_wave_even: 1.0,
            ..Style::default()
        };
        let out = undulate(vec![row(50.0), back], &st, 3, Some(&phase));
        // Height where a line crosses x, between the points either side.
        let at = |l: &Line, x: f32| {
            l.windows(2)
                .find(|q| (q[0][0] - x) * (q[1][0] - x) <= 0.0 && q[0][0] != q[1][0])
                .map(|q| q[0][1] + (q[1][1] - q[0][1]) * (x - q[0][0]) / (q[1][0] - q[0][0]))
                .unwrap()
        };
        let swing = (40..260)
            .map(|x| (at(&out[0], x as f32) - 50.0).abs())
            .fold(0.0f32, f32::max);
        assert!(swing > 2.5, "they wave: {swing}");
        for x in (40..260).step_by(7) {
            let gap = at(&out[1], x as f32) - at(&out[0], x as f32);
            assert!((gap - 5.0).abs() < 0.8, "in step at {x}: gap {gap}");
        }
    }
}
