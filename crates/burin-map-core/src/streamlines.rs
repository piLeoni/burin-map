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
        let Some(d) = self.direction(x0, y0, 0.0, 0.0) else {
            return (Vec::new(), false);
        };
        self.walk(x0, y0, (d.0 * sign, d.1 * sign), sign as i32, 0, true, own)
    }

    /// Follow the field from (x0, y0), heading (pu, pv) at first. Points go
    /// into `own` numbered `sign * (k0 + i + 1)`; `close` ends the line when
    /// it comes back to (x0, y0).
    #[allow(clippy::too_many_arguments)]
    fn walk(
        &self,
        x0: f32,
        y0: f32,
        (mut pu, mut pv): (f32, f32),
        sign: i32,
        k0: i32,
        close: bool,
        own: &mut Own,
    ) -> (Line, bool) {
        let step = self.st.engine_step;
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
            if close
                && i as i32 > self.lag
                && (x - x0).powi(2) + (y - y0).powi(2) < r_close * r_close
            {
                return (pts, true);
            }
            let k = sign * (k0 + i as i32 + 1);
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

/// Candidate seeds on both sides of a line; placed by gap_seed when popped.
fn seeds_beside(queue: &mut Seeds, pts: &Line, lid: i32, every: usize) {
    if pts.len() < 2 {
        return;
    }
    for k in (0..pts.len() - 1).step_by(every) {
        let (dx, dy) = (pts[k + 1][0] - pts[k][0], pts[k + 1][1] - pts[k][1]);
        let n = dx.hypot(dy).max(1e-9);
        let (nx, ny) = (-dy / n, dx / n);
        queue.push_back((pts[k][0], pts[k][1], nx, ny, lid));
        queue.push_back((pts[k][0], pts[k][1], -nx, -ny, lid));
    }
}

type Seeds = VecDeque<(f32, f32, f32, f32, i32)>;

/// Ids of lines drawn elsewhere start here, clear of the ones traced now.
const OTHERS: i32 = 1 << 24;

/// Polylines in px. Closed loops repeat their first point.
pub fn trace_streamlines(f: &Fields, st: &Style) -> Vec<Line> {
    trace_beside(f, st, &[], &[]).0
}

/// `trace_streamlines` next to lines drawn before: the new lines keep their
/// distance from `fixed` and grow out from them, as if all had been traced
/// together. Each of `open` ends where this field starts to be walkable, its
/// last point on the edge, and is carried on. Returns the new lines and, for
/// each open line, the points added to its end.
fn trace_beside(f: &Fields, st: &Style, fixed: &[Line], open: &[Line]) -> (Vec<Line>, Vec<Line>) {
    let (w, h) = (f.w, f.h);
    if w < 5 || h < 5 {
        return (Vec::new(), vec![Vec::new(); open.len()]);
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
    let mut queue = Seeds::new();
    let mut lines: Vec<Line> = Vec::new();
    let seed_every = ((st.space_base() / st.engine_step).round() as usize).max(1);

    for (i, l) in fixed.iter().enumerate() {
        let lid = OTHERS + i as i32;
        t.stamp(l, lid);
        seeds_beside(&mut queue, l, lid, seed_every);
    }
    let mut added = Vec::with_capacity(open.len());
    for (i, l) in open.iter().enumerate() {
        let lid = OTHERS + (fixed.len() + i) as i32;
        let n = l.len();
        let more = if n >= 2 {
            let (p, q) = (l[n - 1], l[n - 2]);
            let m = (p[0] - q[0]).hypot(p[1] - q[1]).max(1e-6);
            let mut own = Own::default();
            for (j, p) in l.iter().enumerate() {
                own.entry(cell_key((p[0] / t.cell) as i32, (p[1] / t.cell) as i32))
                    .or_default()
                    .push((j as i32, p[0], p[1]));
            }
            let head = ((p[0] - q[0]) / m, (p[1] - q[1]) / m);
            t.walk(p[0], p[1], head, 1, n as i32 - 1, false, &mut own).0
        } else {
            Vec::new()
        };
        t.stamp(&more, lid);
        added.push(more);
    }
    // Stamped only now: an arc with both ends at the seam would otherwise
    // block its second end with its own body.
    for (i, (l, more)) in open.iter().zip(&added).enumerate() {
        let lid = OTHERS + (fixed.len() + i) as i32;
        let whole: Line = l.iter().chain(more).copied().collect();
        t.stamp(l, lid);
        seeds_beside(&mut queue, &whole, lid, seed_every);
    }

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
        seeds_beside(&mut queue, &pts, lid, seed_every);
        lines.push(pts);
    }
    (lines, added)
}

/* ------------------------------------------------------------------ */
/*  Striped tracing                                                    */
/* ------------------------------------------------------------------ */

/// How much of the neighbouring stripe a tracer may read while deciding where
/// its own lines stop. It only mirrors `engine_stop`'s reach, so a line can
/// come as close to the boundary as the rule allows.
const STRIPE_MARGIN_PX: f32 = 16.0;

/// How far the stripes traced second may walk into their neighbours: a line
/// carried on from the seam at a slant needs a step or two to leave them.
const SEAM_OVERLAP_PX: f32 = 2.0;

/// Evenly spaced streamlines, traced in parallel stripes.
///
/// `trace_streamlines` already runs a whole line per thread, but it produces
/// one set of lines. Here the plane is cut into vertical stripes, each traced
/// on its own with the same rule over the same fields. A stripe may only
/// *walk* its own columns — the padding it reads has its walkable flag
/// cleared — so its strands stop at the edge.
///
/// Every other stripe is traced first, all at once: a stripe apart, they
/// never meet. The stripes between are traced next, each with its
/// neighbours' lines already in place: it keeps its distance from them, seeds
/// beside them, and carries on every strand that stopped at its edge, so a
/// line crosses the seam as one line. Two rounds, each as long as its
/// slowest stripe.
pub fn trace_stripes(f: &Fields, st: &Style, stripe: i32) -> Vec<Line> {
    // Stripes are cut across the flow, so few lines cross a seam: where
    // lines mostly run east-west, the grid is traced on its side.
    if runs_across(f) {
        let t = transpose(f);
        return trace_columns(&t, st, stripe)
            .into_iter()
            .map(|l| l.into_iter().map(|p| [p[1], p[0]]).collect())
            .collect();
    }
    trace_columns(f, st, stripe)
}

/// The stripe width, in px, that cuts `f` into `count` stripes the way
/// `trace_stripes` will, but none narrower than `MIN_STRIPE_PX`.
pub fn stripe_width(f: &Fields, count: u32) -> i32 {
    let across = if runs_across(f) { f.h } else { f.w } as i32;
    let count = count.max(1) as i32;
    ((across + count - 1) / count).max(MIN_STRIPE_PX)
}

/// Narrower stripes would reach past their neighbours into the next ones.
const MIN_STRIPE_PX: i32 = 64;

/// Whether the walkable field mostly points east-west.
fn runs_across(f: &Fields) -> bool {
    let (mut x, mut y) = (0.0f64, 0.0f64);
    for i in 0..f.w * f.h {
        if f.walkable[i] {
            x += f.tx[i].abs() as f64;
            y += f.ty[i].abs() as f64;
        }
    }
    x > y
}

/// `f` with rows and columns swapped.
fn transpose(f: &Fields) -> Fields {
    let (w, h) = (f.w, f.h);
    let swap = |v: &[f32]| -> Vec<f32> { (0..w * h).map(|k| v[(k % h) * w + k / h]).collect() };
    Fields {
        w: h,
        h: w,
        tx: swap(&f.ty),
        ty: swap(&f.tx),
        input: swap(&f.input),
        walkable: (0..w * h)
            .map(|k| f.walkable[(k % h) * w + k / h])
            .collect(),
        space: swap(&f.space),
    }
}

/// What a stripe added to a neighbour's line: which line, at its tail or its
/// head, and the points, going away from it.
type Grown = (usize, bool, Line);

fn trace_columns(f: &Fields, st: &Style, stripe: i32) -> Vec<Line> {
    use rayon::prelude::*;
    let (w, h) = (f.w, f.h);
    if w < 5 || h < 5 || stripe < 8 {
        return Vec::new();
    }
    let n = (w as i32 + stripe - 1) / stripe;
    let cols = |s: i32| (s * stripe, ((s + 1) * stripe).min(w as i32));
    // Each stripe shuffles its seeds with its own seed, so neighbours do not
    // repeat the same pattern side by side.
    let seeded = |s: i32| {
        let mut st = st.clone();
        st.batch_seed = st.batch_seed.wrapping_add(0x9E37_79B9 * (s as u64 + 1));
        st
    };
    let shift = |ls: Vec<Line>, dx: f32| -> Vec<Line> {
        ls.into_iter()
            .map(|l| l.into_iter().map(|p| [p[0] + dx, p[1]]).collect())
            .collect()
    };

    // Every other stripe first: they are a stripe apart, so none sees another.
    let even: Vec<i32> = (0..n).step_by(2).collect();
    let mut lines: Vec<Line> = even
        .par_iter()
        .flat_map_iter(|&s| {
            let (x0, x1) = cols(s);
            let sub = slice_columns(f, x0, x1, STRIPE_MARGIN_PX, 0.0);
            let dx = (x0 as f32 - STRIPE_MARGIN_PX).max(0.0);
            shift(trace_streamlines(&sub, &seeded(s)), dx)
        })
        .collect();

    // Then the ones between, next to what their neighbours drew: they keep
    // the spacing across the seam and carry on the lines that reach it.
    let space_max = f.space.iter().copied().fold(0.0f32, f32::max);
    let reach = STRIPE_MARGIN_PX.max(2.0 * space_max + 2.0);
    let spans: Vec<(f32, f32)> = lines
        .iter()
        .map(|l| {
            l.iter()
                .fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p[0]), b.max(p[0])))
        })
        .collect();
    let odd: Vec<i32> = (1..n).step_by(2).collect();
    let done: Vec<(Vec<Line>, Vec<Grown>)> = odd
        .par_iter()
        .map(|&s| {
            let (x0, x1) = cols(s);
            let sub = slice_columns(f, x0, x1, reach, SEAM_OVERLAP_PX);
            let lo = (x0 as f32 - reach).max(0.0);
            let hi = lo + sub.w as f32;
            let seam = |p: [f32; 2], q: [f32; 2]| {
                let out = p[0] - q[0];
                (out > 0.0 && (p[0] - x0 as f32).abs() <= 2.0)
                    || (out < 0.0 && (p[0] - x1 as f32).abs() <= 2.0)
            };
            let inside = |p: &[f32; 2]| p[0] >= lo && p[0] < hi - 1.0;
            let local = |p: &[f32; 2]| [p[0] - lo, p[1]];
            let mut fixed = Vec::new();
            let mut open = Vec::new();
            let mut ends = Vec::new();
            for (i, l) in lines.iter().enumerate() {
                if l.len() < 2 || spans[i].1 < lo || spans[i].0 >= hi {
                    continue;
                }
                let m = l.len();
                // From each end at the seam, the run of points in the window.
                let mut tail = vec![false; m];
                for at_tail in [false, true] {
                    let (p, q) = if at_tail {
                        (l[m - 1], l[m - 2])
                    } else {
                        (l[0], l[1])
                    };
                    if !seam(p, q) {
                        continue;
                    }
                    let run: Vec<usize> = if at_tail {
                        (0..m).rev().take_while(|&k| inside(&l[k])).collect()
                    } else {
                        (0..m).take_while(|&k| inside(&l[k])).collect()
                    };
                    for &k in &run {
                        tail[k] = true;
                    }
                    open.push(run.iter().rev().map(|&k| local(&l[k])).collect::<Line>());
                    ends.push((i, at_tail));
                }
                let mut piece = Vec::new();
                for (k, p) in l.iter().enumerate() {
                    if inside(p) && !tail[k] {
                        piece.push(local(p));
                    } else if !piece.is_empty() {
                        fixed.push(std::mem::take(&mut piece));
                    }
                }
                if !piece.is_empty() {
                    fixed.push(piece);
                }
            }
            let (new, added) = trace_beside(&sub, &seeded(s), &fixed, &open);
            let grown = ends
                .into_iter()
                .zip(shift(added, lo))
                .map(|((i, at_tail), more)| (i, at_tail, more))
                .collect();
            (shift(new, lo), grown)
        })
        .collect();
    for (new, grown) in done {
        for (i, at_tail, more) in grown {
            if more.is_empty() {
                continue;
            }
            if at_tail {
                lines[i].extend(more);
            } else {
                lines[i].splice(0..0, more.into_iter().rev());
            }
        }
        lines.extend(new);
    }

    let seams: Vec<f32> = (1..n).map(|s| (s * stripe) as f32).collect();
    let reach = |p: [f32; 2]| {
        let (x, y) = (
            p[0].clamp(0.0, w as f32 - 1.0),
            p[1].clamp(0.0, h as f32 - 1.0),
        );
        f.space[y as usize * w + x as usize].max(3.0)
    };
    join_at_seams(lines, &seams, reach)
}

/// A line's end at a seam: which end, where, the way it faces, and whether
/// it is on the seam's left.
type SeamEnd = (usize, [f32; 2], [f32; 2], bool);

/// Strands cut by a stripe edge, joined up again: an end that runs into a
/// seam meets the nearest end within `reach` px (the spacing there) that
/// leaves the other side the same way. A line waved or dashed afterwards
/// then shows no seam.
fn join_at_seams(lines: Vec<Line>, seams: &[f32], reach: impl Fn([f32; 2]) -> f32) -> Vec<Line> {
    if seams.is_empty() {
        return lines;
    }
    // Ends: index 2 * line, +1 for the tail. Each at a seam, facing it.
    let mut at_seam: Vec<Vec<SeamEnd>> = vec![Vec::new(); seams.len()];
    for (i, l) in lines.iter().enumerate() {
        if l.len() < 2 {
            continue;
        }
        for tail in [false, true] {
            let (p, q) = if tail {
                (l[l.len() - 1], l[l.len() - 2])
            } else {
                (l[0], l[1])
            };
            let (dx, dy) = (p[0] - q[0], p[1] - q[1]);
            let m = dx.hypot(dy).max(1e-6);
            let out = [dx / m, dy / m];
            for (k, &x) in seams.iter().enumerate() {
                // Its side is the way it heads: a strand may stop a step
                // past the seam.
                let left = out[0] > 0.0;
                if (p[0] - x).abs() <= reach(p) && out[0].abs() > 0.2 {
                    at_seam[k].push((2 * i + tail as usize, p, out, left));
                }
            }
        }
    }
    let mut pairs = Vec::new();
    for ends in &at_seam {
        for a in ends.iter().filter(|e| e.3) {
            for b in ends.iter().filter(|e| !e.3) {
                let d = (a.1[0] - b.1[0]).hypot(a.1[1] - b.1[1]);
                if d <= reach(a.1).max(reach(b.1)) && a.2[0] * -b.2[0] + a.2[1] * -b.2[1] > 0.5 {
                    pairs.push((d, a.0, b.0));
                }
            }
        }
    }
    pairs.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut partner = vec![usize::MAX; lines.len() * 2];
    // A line may meet its own other end: carried on round, it closes.
    for (_, a, b) in pairs {
        if partner[a] == usize::MAX && partner[b] == usize::MAX {
            partner[a] = b;
            partner[b] = a;
        }
    }

    let mut used = vec![false; lines.len()];
    let mut out = Vec::with_capacity(lines.len());
    // Chains start at a free end; what is left are loops, cut anywhere.
    let starts = (0..lines.len() * 2)
        .filter(|&e| partner[e] == usize::MAX)
        .chain(0..lines.len() * 2);
    for start in starts {
        if used[start / 2] {
            continue;
        }
        let mut chain: Line = Vec::new();
        let mut enter = start;
        loop {
            let i = enter / 2;
            used[i] = true;
            let from_tail = enter % 2 == 1;
            if from_tail {
                chain.extend(lines[i].iter().rev());
            } else {
                chain.extend(lines[i].iter());
            }
            let exit = if from_tail { 2 * i } else { 2 * i + 1 };
            match partner[exit] {
                p if p != usize::MAX && !used[p / 2] => enter = p,
                p if p == start => {
                    chain.push(chain[0]);
                    break;
                }
                _ => break,
            }
        }
        out.push(chain);
    }
    out
}

/// `f` narrowed to columns `x0..x1` plus `pad` columns each side. Beyond
/// `over` columns each side `walkable` is cleared, so the tracer reads the
/// rest of the padding but stays out of it.
fn slice_columns(f: &Fields, x0: i32, x1: i32, pad: f32, over: f32) -> Fields {
    let (w, h) = (f.w, f.h);
    let lo = (x0 as f32 - pad).max(0.0) as usize;
    let hi = ((x1 as f32 + pad).round() as usize).min(w);
    let nw = hi - lo;
    let mut out = Fields {
        w: nw,
        h,
        tx: Vec::with_capacity(nw * h),
        ty: Vec::with_capacity(nw * h),
        input: Vec::with_capacity(nw * h),
        walkable: Vec::with_capacity(nw * h),
        space: Vec::with_capacity(nw * h),
    };
    for y in 0..h {
        let row = y * w;
        let (a, b) = (row + lo, row + hi);
        out.tx.extend_from_slice(&f.tx[a..b]);
        out.ty.extend_from_slice(&f.ty[a..b]);
        out.input.extend_from_slice(&f.input[a..b]);
        out.walkable.extend_from_slice(&f.walkable[a..b]);
        out.space.extend_from_slice(&f.space[a..b]);
    }
    // The padding belongs to the neighbour: read it, but do not draw in it.
    let over = over as i32;
    let inner = (x0 - over - lo as i32).max(0) as usize;
    let inner_end = (((x1 + over) as usize).saturating_sub(lo)).min(nw);
    for y in 0..h {
        let row = y * nw;
        for x in 0..inner.min(nw) {
            out.walkable[row + x] = false;
        }
        for x in inner_end..nw {
            out.walkable[row + x] = false;
        }
    }
    out
}
