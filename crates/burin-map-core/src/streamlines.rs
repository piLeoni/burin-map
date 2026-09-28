//! Evenly spaced streamlines (Jobard & Lefer, 1997) with variable spacing
//! and mid-gap seeding. Mirrors `src/burin/streamlines.py`; see there for
//! the full explanation.

use std::collections::{HashMap, VecDeque};
use std::hash::{BuildHasherDefault, Hasher};

use crate::fields::Fields;
use crate::rng::Rng;
use crate::style::Style;

pub type Line = Vec<[f32; 2]>;

/// Fast hasher for small integer keys (spatial hash cells).
#[derive(Default)]
struct Fx(u64);

impl Hasher for Fx {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.write_u64(b as u64);
        }
    }
    fn write_u64(&mut self, i: u64) {
        self.0 = (self.0.rotate_left(5) ^ i).wrapping_mul(0x517c_c1b7_2722_0a95);
    }
}

type FxMap<K, V> = HashMap<K, V, BuildHasherDefault<Fx>>;
/// Points of the line being traced: cell → (signed arc index, x, y).
type Own = FxMap<u64, Vec<(i32, f32, f32)>>;

#[inline]
fn cell_key(cx: i32, cy: i32) -> u64 {
    ((cx as u32 as u64) << 32) | cy as u32 as u64
}

#[inline]
fn bilinear(g: &[f32], w: usize, x: f32, y: f32) -> f32 {
    let (x0, y0) = (x as usize, y as usize);
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let r0 = y0 * w + x0;
    let r1 = r0 + w;
    (g[r0] * (1.0 - fx) + g[r0 + 1] * fx) * (1.0 - fy) + (g[r1] * (1.0 - fx) + g[r1 + 1] * fx) * fy
}

/// Pixel offsets (dy, dx) within radius r.
fn disk(r: f32) -> Vec<(isize, isize)> {
    let ri = r.ceil() as isize;
    let mut v = Vec::new();
    for dy in -ri..=ri {
        for dx in -ri..=ri {
            if ((dx * dx + dy * dy) as f32) <= r * r {
                v.push((dy, dx));
            }
        }
    }
    v
}

struct Tracer<'a> {
    f: &'a Fields,
    st: &'a Style,
    w: usize,
    h: usize,
    occ_test: Vec<bool>,
    occ_seed: Vec<bool>,
    line_id: Vec<i32>,
    disks: FxMap<u64, Vec<(isize, isize)>>,
    cell: f32,
    lag: i32,
    max_steps: usize,
}

#[derive(Clone, Copy)]
enum Occ {
    Test,
    Seed,
}

impl<'a> Tracer<'a> {
    fn inside(&self, x: f32, y: f32) -> bool {
        x >= 1.0 && x < (self.w - 2) as f32 && y >= 1.0 && y < (self.h - 2) as f32
    }

    fn free(&self, x: f32, y: f32, which: Occ) -> bool {
        if !self.inside(x, y) {
            return false;
        }
        let i = (y + 0.5) as usize * self.w + (x + 0.5) as usize;
        let occ = match which {
            Occ::Test => &self.occ_test,
            Occ::Seed => &self.occ_seed,
        };
        self.f.walkable[i] && !occ[i]
    }

    fn sep_at(&self, x: f32, y: f32) -> f32 {
        let xi = (x.max(0.0) as usize).min(self.w - 1);
        let yi = (y.max(0.0) as usize).min(self.h - 1);
        self.f.space[yi * self.w + xi]
    }

    /// Unit field direction at (x, y), flipped to agree with (pu, pv).
    fn direction(&self, x: f32, y: f32, pu: f32, pv: f32) -> Option<(f32, f32)> {
        let u = bilinear(&self.f.tx, self.w, x, y);
        let v = bilinear(&self.f.ty, self.w, x, y);
        let n = u.hypot(v);
        if n < 1e-6 {
            return None;
        }
        let (u, v) = (u / n, v / n);
        Some(if u * pu + v * pv < 0.0 {
            (-u, -v)
        } else {
            (u, v)
        })
    }

    fn hits_itself(&self, own: &Own, x: f32, y: f32, k: i32) -> bool {
        let r = self.sep_at(x, y) * self.st.engine_stop;
        let r2 = r * r;
        let (cx, cy) = ((x / self.cell) as i32, (y / self.cell) as i32);
        for gx in cx - 1..=cx + 1 {
            for gy in cy - 1..=cy + 1 {
                if let Some(pts) = own.get(&cell_key(gx, gy)) {
                    for &(j, px, py) in pts {
                        if (k - j).abs() > self.lag && (px - x).powi(2) + (py - y).powi(2) < r2 {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }

    /// Follow the field from a seed in one direction (RK2).
    /// Returns (points, closed) where closed means it came back to the seed.
    fn trace(&self, x0: f32, y0: f32, sign: f32, own: &mut Own) -> (Line, bool) {
        let step = self.st.engine_step;
        let Some(d) = self.direction(x0, y0, 0.0, 0.0) else {
            return (Vec::new(), false);
        };
        let (mut pu, mut pv) = (d.0 * sign, d.1 * sign);
        let (mut x, mut y) = (x0, y0);
        let r_close = (self.sep_at(x0, y0) * self.st.engine_stop).max(1.5 * step);
        let mut pts = Vec::new();
        for i in 0..self.max_steps {
            let Some(d1) = self.direction(x, y, pu, pv) else {
                break;
            };
            let (xm, ym) = (x + 0.5 * step * d1.0, y + 0.5 * step * d1.1);
            if !self.inside(xm, ym) {
                break;
            }
            let Some(d2) = self.direction(xm, ym, d1.0, d1.1) else {
                break;
            };
            x += step * d2.0;
            y += step * d2.1;
            (pu, pv) = d2;
            if !self.free(x, y, Occ::Test) {
                break;
            }
            if i as i32 > self.lag && (x - x0).powi(2) + (y - y0).powi(2) < r_close * r_close {
                return (pts, true);
            }
            let k = sign as i32 * (i as i32 + 1);
            if self.hits_itself(own, x, y, k) {
                break;
            }
            own.entry(cell_key((x / self.cell) as i32, (y / self.cell) as i32))
                .or_default()
                .push((k, x, y));
            pts.push([x, y]);
        }
        (pts, false)
    }

    fn stamp(&mut self, line: &Line, lid: i32) {
        let (w, h) = (self.w as isize, self.h as isize);
        let clamp = |v: isize, n: isize| v.clamp(0, n - 1) as usize;
        for p in line {
            let (xi, yi) = (p[0].round() as isize, p[1].round() as isize);
            // 4-connected so a probe stepping across the line can't slip through
            for (dx, dy) in [(0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)] {
                self.line_id[clamp(yi + dy, h) * self.w + clamp(xi + dx, w)] = lid;
            }
            let q = (self.f.space[clamp(yi, h) * self.w + clamp(xi, w)] * 20.0).round();
            for (which, ratio) in [(0u64, self.st.engine_stop), (1u64, 0.95)] {
                let key = (q as u64) << 1 | which;
                let offsets = self
                    .disks
                    .entry(key)
                    .or_insert_with(|| disk(q / 20.0 * ratio));
                let occ = if which == 0 {
                    &mut self.occ_test
                } else {
                    &mut self.occ_seed
                };
                for &(dy, dx) in offsets.iter() {
                    occ[clamp(yi + dy, h) * self.w + clamp(xi + dx, w)] = true;
                }
            }
        }
    }

    /// Seed beside line `lid` at (bx, by) along normal (nx, ny), or None.
    /// Wide gaps get a seed at space, narrower ones (≥ 2 × fill_ratio × space) one
    /// in the middle, narrower still none.
    fn gap_seed(&self, bx: f32, by: f32, nx: f32, ny: f32, lid: i32) -> Option<(f32, f32)> {
        let d = self.sep_at(bx, by);
        let mut s = 1.0f32;
        while s < 2.0 * d {
            let (xf, yf) = (bx + nx * s + 0.5, by + ny * s + 0.5);
            if xf < 0.0 || yf < 0.0 || xf >= self.w as f32 || yf >= self.h as f32 {
                break;
            }
            let i = yf as usize * self.w + xf as usize;
            if !self.f.walkable[i] {
                break; // coast or edge: not a neighbour, fill up to it
            }
            let j = self.line_id[i];
            if j >= 0 && j != lid {
                let gap = s + 0.75; // stamp is ~1 px thick
                if gap < 2.0 * self.st.engine_fill * d {
                    return None;
                }
                let (sx, sy) = (bx + nx * gap / 2.0, by + ny * gap / 2.0);
                return self.free(sx, sy, Occ::Test).then_some((sx, sy));
            }
            s += 0.5;
        }
        let (sx, sy) = (bx + nx * d, by + ny * d);
        self.free(sx, sy, Occ::Seed).then_some((sx, sy))
    }
}

/// Polylines in px. Closed loops repeat their first point.
pub fn trace_streamlines(f: &Fields, st: &Style) -> Vec<Line> {
    let (w, h) = (f.w, f.h);
    if w < 5 || h < 5 {
        return Vec::new();
    }
    let space_max = f.space.iter().copied().fold(0.0f32, f32::max);
    let space_min = f.space.iter().copied().fold(f32::INFINITY, f32::min);
    let mut t = Tracer {
        f,
        st,
        w,
        h,
        occ_test: vec![false; w * h],
        occ_seed: vec![false; w * h],
        line_id: vec![-1; w * h],
        disks: FxMap::default(),
        cell: space_max.max(1.0),
        lag: (std::f32::consts::TAU * space_max * st.engine_stop / st.engine_step).ceil() as i32
            + 2,
        max_steps: (4.0 * (w + h) as f32 / st.engine_step) as usize,
    };

    let mut rng = Rng::new(st.batch_seed);
    let g = (space_min as usize).max(2);
    let mut grid: Vec<(f32, f32)> = (2..h - 2)
        .step_by(g)
        .flat_map(|y| {
            (2..w - 2)
                .step_by(g)
                .map(move |x| (x as f32 + 0.5, y as f32 + 0.5))
        })
        .collect();
    rng.shuffle(&mut grid);
    let mut grid_i = 0;
    let mut queue: VecDeque<(f32, f32, f32, f32, i32)> = VecDeque::new();
    let mut lines: Vec<Line> = Vec::new();
    let seed_every = ((st.space / st.engine_step).round() as usize).max(1);

    loop {
        let (sx, sy) = if let Some((bx, by, nx, ny, lid)) = queue.pop_front() {
            match t.gap_seed(bx, by, nx, ny, lid) {
                Some(p) => p,
                None => continue,
            }
        } else {
            while grid_i < grid.len() && !t.free(grid[grid_i].0, grid[grid_i].1, Occ::Seed) {
                grid_i += 1;
            }
            if grid_i >= grid.len() {
                break;
            }
            grid_i += 1;
            grid[grid_i - 1]
        };

        let mut own = Own::default();
        own.insert(
            cell_key((sx / t.cell) as i32, (sy / t.cell) as i32),
            vec![(0, sx, sy)],
        );
        let (fwd, closed) = t.trace(sx, sy, 1.0, &mut own);
        let bwd = if closed {
            Vec::new()
        } else {
            t.trace(sx, sy, -1.0, &mut own).0
        };
        let mut pts: Line = bwd.into_iter().rev().collect();
        pts.push([sx, sy]);
        pts.extend(fwd);
        if closed {
            pts.push(pts[0]);
        }
        if (pts.len() as f32) * st.engine_step < st.engine_min {
            continue;
        }
        let lid = lines.len() as i32;
        t.stamp(&pts, lid);

        // Candidate seeds on both sides; placed by gap_seed when popped.
        for k in (0..pts.len() - 1).step_by(seed_every) {
            let (dx, dy) = (pts[k + 1][0] - pts[k][0], pts[k + 1][1] - pts[k][1]);
            let n = dx.hypot(dy).max(1e-9);
            let (nx, ny) = (-dy / n, dx / n);
            queue.push_back((pts[k][0], pts[k][1], nx, ny, lid));
            queue.push_back((pts[k][0], pts[k][1], -nx, -ny, lid));
        }
        lines.push(pts);
    }
    lines
}
