//! burin-map drawing engine: pen-and-ink terrain lines from an elevation grid.
//!
//! Pure computation, no I/O: give it heights in metres on a grid of square
//! pixels, get back dashes as polylines in pixel coordinates. The same code
//! runs natively, in the browser (WASM) and behind the Python package.
//!
//! Pipeline: fields (direction, input, spacing) → evenly spaced streamlines
//! → plotting order → input-driven dashes. Several batch passes (angles)
//! run in parallel and are merged.

mod dashes;
#[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
pub mod engine;
mod fields;
pub mod geojson;
#[cfg(feature = "gpu")]
pub mod gpu;
mod grid;
mod order;
mod rng;
mod streamlines;
mod style;
pub mod water;

pub use dashes::dash_lines;
pub use fields::{batch_fields, build_fields, build_fields_with, land_mask, Fields};
pub use order::{pen_up_travel, plot_order};
pub use streamlines::{trace_streamlines, trace_stripes, Line};
pub use style::{
    from_json_batches, from_json_style, to_json_batches, to_json_style, Batch, Param, Style, GROUPS,
};
pub use water::{Follow, WaterBatch, WaterLayer};

/// Stripe width used by the drawing engine, in px. See `streamlines::trace_stripes`.
pub const STRIPE: i32 = 512;

/// Maps pixel coordinates back to geographic lon/lat. Pixel (0,0) is the
/// north-west corner of the elevation grid; y grows southward.
#[derive(Clone, Debug)]
pub struct GeoTransform {
    /// NW corner longitude.
    pub nw_lon: f64,
    /// NW corner latitude.
    pub nw_lat: f64,
    /// Ground distance per pixel in the east direction, degrees.
    pub deg_per_px_lon: f64,
    /// Ground distance per pixel in the south direction, degrees.
    pub deg_per_px_lat: f64,
}

impl GeoTransform {
    /// Pixel `(x, y)` → `(lon, lat)`.
    pub fn to_lonlat(&self, x: f64, y: f64) -> (f64, f64) {
        (
            self.nw_lon + x * self.deg_per_px_lon,
            self.nw_lat - y * self.deg_per_px_lat,
        )
    }
}

pub struct Drawing {
    pub width: usize,
    pub height: usize,
    /// Dashes in drawing order.
    pub paths: Vec<Line>,
    /// Streamlines before dashing, in drawing order.
    pub lines: Vec<Line>,
}

/// Draw with a single contour-parallel batch (the usual look).
pub fn draw(elev: &[f32], width: usize, height: usize, px_m: f32, style: &Style) -> Drawing {
    draw_batches(elev, width, height, px_m, &[Batch::along(style.clone())])
}

/// Draw one or more batch passes. `batches` must be non-empty. Terrain
/// fields (direction, input) come from the first batch's style; each pass
/// rotates that direction by its `angle` and uses its own space / dashes.
/// Passes run in parallel.
pub fn draw_batches(
    elev: &[f32],
    width: usize,
    height: usize,
    px_m: f32,
    batches: &[Batch],
) -> Drawing {
    assert_eq!(elev.len(), width * height, "elevation grid size mismatch");
    assert!(!batches.is_empty(), "need at least one batch");

    let f = build_fields(elev, width, height, px_m, &batches[0].style);
    draw_fields(&f, batches)
}

/// [`draw_batches`] on whichever device [`engine`] picked: the GPU when the
/// machine has one, the CPU otherwise. This is what the CLI, the bindings and
/// the browser call — the drawing is the product, so they should not care.
///
/// On `wasm32` the GPU is driven from JavaScript (see `burin-map-wasm`), so
/// there is nothing asynchronous to do here: it falls back to the CPU.
pub fn draw_batches_auto(
    elev: &[f32],
    width: usize,
    height: usize,
    px_m: f32,
    batches: &[Batch],
) -> Drawing {
    #[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
    if let Some(g) = engine::device() {
        assert!(!batches.is_empty(), "need at least one batch");
        let f = g.build_fields_blocking(elev, width, height, px_m, &batches[0].style);
        return draw_fields(&f, batches);
    }
    draw_batches(elev, width, height, px_m, batches)
}

/// [`draw_batches_auto`] with water: its coverage decides the land, and its
/// batches draw on it.
pub fn draw_batches_auto_with(
    elev: &[f32],
    width: usize,
    height: usize,
    px_m: f32,
    batches: &[Batch],
    water: Option<&WaterLayer>,
) -> Drawing {
    assert_eq!(elev.len(), width * height, "elevation grid size mismatch");
    assert!(!batches.is_empty(), "need at least one batch");
    let coverage = water.map(|w| w.coverage);
    #[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
    if let Some(g) = engine::device() {
        let f =
            g.build_fields_with_blocking(elev, width, height, px_m, &batches[0].style, coverage);
        return draw_fields_timed(&f, batches, water, || 0.0).0;
    }
    let f = build_fields_with(elev, width, height, px_m, &batches[0].style, coverage);
    draw_fields_timed(&f, batches, water, || 0.0).0
}

/// `draw_batches` from fields already built (e.g. by `gpu::Gpu`) with the
/// first batch's style.
pub fn draw_fields(f: &Fields, batches: &[Batch]) -> Drawing {
    draw_fields_timed(f, batches, None, || 0.0).0
}

/// Milliseconds per phase of a draw, in the order they ran. Phases that run
/// once per batch carry the batch index in their label.
pub type Timings = Vec<(String, f64)>;

/// Streamlines and dashes of some batches, and the time each phase took.
type Traced = (Vec<Line>, Vec<Line>, Timings);

/// [`draw_fields`], timing each phase with `now` (milliseconds), with water
/// drawn alongside the land when there is some. The clock is passed in because
/// `std::time::Instant` panics on wasm32.
pub fn draw_fields_timed(
    f: &Fields,
    batches: &[Batch],
    water: Option<&WaterLayer>,
    now: fn() -> f64,
) -> (Drawing, Timings) {
    assert!(!batches.is_empty(), "need at least one batch");
    let (width, height) = (f.w, f.h);
    let ((mut lines, mut paths, mut timings), wet) = rayon::join(
        || trace_land(f, batches, now),
        || water.map(|wl| trace_water(wl, width, height, now)),
    );
    if let Some((l, p, t)) = wet {
        lines.extend(l);
        paths.extend(p);
        timings.extend(t);
    }
    if !paths.is_empty() {
        let t = now();
        paths = plot_order(paths);
        timings.push(("order dashes".into(), now() - t));
    }
    let d = Drawing {
        width,
        height,
        paths,
        lines,
    };
    (d, timings)
}

fn trace_land(f: &Fields, batches: &[Batch], now: fn() -> f64) -> Traced {
    use rayon::prelude::*;
    let base_space = batches[0].style.space;
    let parts: Vec<(Vec<Line>, Vec<Line>, [f64; 4])> = batches
        .par_iter()
        .enumerate()
        .map(|(i, b)| {
            let mut st = b.style.clone();
            st.batch_seed = st.batch_seed.wrapping_add(i as u64 * 0x9E37_79B9);
            let t0 = now();
            let bf = batch_fields(f, b, base_space);
            let t1 = now();
            let (lines, paths, t) = trace_batch(&bf, &st, now);
            (lines, paths, [t1 - t0, t[0], t[1], t[2]])
        })
        .collect();
    collect_parts(parts, "batch")
}

fn trace_water(wl: &WaterLayer, w: usize, h: usize, now: fn() -> f64) -> Traced {
    use rayon::prelude::*;
    let t0 = now();
    let (dist, shoreline) = water::shore(wl.coverage, w, h);
    let t1 = now();
    let parts: Vec<(Vec<Line>, Vec<Line>, [f64; 4])> = wl
        .batches
        .par_iter()
        .enumerate()
        .map(|(i, b)| {
            let mut st = b.style.clone();
            st.batch_seed = st.batch_seed.wrapping_add(0x5EA ^ (i as u64 * 0x9E37_79B9));
            let t0 = now();
            let f = water::water_fields(wl.coverage, &dist, w, h, b);
            let t1 = now();
            let (lines, paths, t) = trace_batch(&f, &st, now);
            (lines, paths, [t1 - t0, t[0], t[1], t[2]])
        })
        .collect();
    let (mut lines, mut paths, mut timings) = collect_parts(parts, "water");
    timings.insert(0, ("water shore distance".into(), t1 - t0));
    if wl.shore {
        lines.extend(shoreline.iter().cloned());
        paths.extend(shoreline);
    }
    (lines, paths, timings)
}

/// Trace, order and dash one batch on its own fields; ms for each of the three.
fn trace_batch(f: &Fields, st: &Style, now: fn() -> f64) -> (Vec<Line>, Vec<Line>, [f64; 3]) {
    let t0 = now();
    let traced = trace_stripes(f, st, crate::STRIPE);
    let t1 = now();
    let lines = plot_order(traced);
    let t2 = now();
    let paths = dash_lines(&lines, &f.input, f.w, f.h, st);
    let t3 = now();
    (lines, paths, [t1 - t0, t2 - t1, t3 - t2])
}

fn collect_parts(parts: Vec<(Vec<Line>, Vec<Line>, [f64; 4])>, kind: &str) -> Traced {
    let mut timings = Timings::new();
    let mut lines = Vec::new();
    let mut paths = Vec::new();
    for (i, (l, p, t)) in parts.into_iter().enumerate() {
        for (name, ms) in ["fields", "trace", "order lines", "dashes"].iter().zip(t) {
            timings.push((format!("{kind} {i} {name}"), ms));
        }
        lines.extend(l);
        paths.extend(p);
    }
    (lines, paths, timings)
}

impl Drawing {
    /// SVG document. With `mm_per_px` it has a physical size in millimetres
    /// (coordinates stay in px; the viewBox scales them).
    pub fn to_svg(&self, stroke: f32, mm_per_px: Option<f32>) -> String {
        use std::fmt::Write;
        let (w, h) = (self.width, self.height);
        let size = match mm_per_px {
            Some(k) => format!(
                r#"width="{:.2}mm" height="{:.2}mm""#,
                w as f32 * k,
                h as f32 * k
            ),
            None => format!(r#"width="{w}" height="{h}""#),
        };
        let mut s = format!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" {size} viewBox="0 0 {w} {h}"><g fill="none" stroke="#000" stroke-width="{stroke}" stroke-linecap="round" stroke-linejoin="round">"##
        );
        s.push('\n');
        for p in &self.paths {
            s.push_str("<path d=\"M");
            for (i, q) in p.iter().enumerate() {
                if i > 0 {
                    s.push_str(" L");
                }
                let _ = write!(s, "{:.2} {:.2}", q[0], q[1]);
            }
            s.push_str("\"/>\n");
        }
        s.push_str("</g></svg>\n");
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn island(w: usize, h: usize) -> Vec<f32> {
        let mut e = vec![0.0f32; w * h];
        for y in 0..h {
            for x in 0..w {
                let bump = |cx: f32, cy: f32, r: f32, hgt: f32| {
                    hgt * (-((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)) / (2.0 * r * r))
                        .exp()
                };
                e[y * w + x] = (bump(150.0, 150.0, 60.0, 1500.0) + bump(270.0, 170.0, 40.0, 900.0)
                    - 150.0)
                    .max(0.0);
            }
        }
        e
    }

    #[test]
    fn draws_an_island() {
        let (w, h) = (400, 300);
        let e = island(w, h);
        let d = draw(&e, w, h, 200.0, &Style::default());
        assert!(d.lines.len() > 20, "{} lines", d.lines.len());
        assert!(d.paths.len() > d.lines.len());
        let on_land = d
            .paths
            .iter()
            .flatten()
            .filter(|p| e[(p[1] as usize) * w + p[0] as usize] > 0.0)
            .count();
        let total: usize = d.paths.iter().map(|p| p.len()).sum();
        assert!(on_land as f32 / total as f32 > 0.99);
    }

    #[test]
    fn shadow_darker_than_light() {
        let (w, h) = (400, 300);
        let f = build_fields(&island(w, h), w, h, 200.0, &Style::default());
        assert!(f.input[190 * w + 190] > f.input[110 * w + 110]);
    }

    #[test]
    fn flat_ground_sits_at_input_flat() {
        let (w, h) = (80, 80);
        let mut e = vec![100.0f32; w * h];
        e[40 * w + 40] = 101.0;
        let f = build_fields(&e, w, h, 200.0, &Style::default());
        let mut mid: Vec<f32> = (20..60)
            .flat_map(|y| (20..60).map(move |x| (x, y)))
            .map(|(x, y)| f.input[y * w + x])
            .collect();
        mid.sort_by(|a, b| a.total_cmp(b));
        assert!((mid[mid.len() / 2] - Style::default().input_flat).abs() < 0.02);
    }

    /// The GPU path lands on the same terrain: every field within a small
    /// tolerance, and a drawing of the same shape (the tracer turns last-bit
    /// differences into different dashes, so only statistics can be compared).
    #[cfg(feature = "gpu")]
    #[test]
    fn gpu_matches_cpu_terrain() {
        let g = match gpu::Gpu::new_blocking() {
            Some(g) => g,
            None => return,
        };
        let (w, h) = (400, 300);
        let e = island(w, h);
        let st = Style::default();
        let cpu = build_fields(&e, w, h, 200.0, &st);
        let gpu = g.build_fields_blocking(&e, w, h, 200.0, &st);
        for (name, a, b, tol) in [
            ("tx", &cpu.tx, &gpu.tx, 5e-3),
            ("ty", &cpu.ty, &gpu.ty, 5e-3),
            ("input", &cpu.input, &gpu.input, 1e-3),
            ("space", &cpu.space, &gpu.space, 1e-3),
        ] {
            // Direction is only defined on walkable ground; on sea both paths
            // may leave any unit vector behind.
            let worst = (0..a.len())
                .filter(|&i| name == "input" || name == "space" || cpu.walkable[i])
                .map(|i| (a[i] - b[i]).abs())
                .fold(0.0, f32::max);
            assert!(worst < tol, "{name}: worst difference {worst}");
        }
        assert_eq!(cpu.walkable, gpu.walkable, "walkable");

        let dc = draw_fields(&cpu, &[Batch::along(st.clone())]);
        let dg = draw_fields(&gpu, &[Batch::along(st)]);
        let ratio = dg.lines.len() as f32 / dc.lines.len() as f32;
        assert!(
            (0.8..1.25).contains(&ratio),
            "lines {} vs {}",
            dc.lines.len(),
            dg.lines.len()
        );
    }

    /// A polder below sea level on the left, a lake up a hill on the right:
    /// height alone gets both wrong, the water coverage gets both right.
    #[test]
    fn water_coverage_decides_the_land() {
        let (w, h) = (200, 100);
        let mut e = vec![-4.0f32; w * h];
        let mut c = vec![0.0f32; w * h];
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                if x >= 100 {
                    e[i] = 50.0 + ((x - 100) as f32).powf(1.3) + (y as f32 * 0.2).sin() * 5.0;
                }
                if x >= 150 {
                    c[i] = 1.0;
                }
                if x < 100 {
                    e[i] += (x as f32 * 0.3).sin() + (y as f32 * 0.25).cos();
                }
            }
        }
        let by_height = land_mask(&e, None);
        let by_water = land_mask(&e, Some(&c));
        assert!(!by_height[50 * w + 50] && by_water[50 * w + 50], "polder");
        assert!(by_height[50 * w + 170] && !by_water[50 * w + 170], "lake");

        let batches = [WaterBatch::default()];
        let layer = WaterLayer {
            coverage: &c,
            shore: true,
            batches: &batches,
        };
        let d = draw_batches_auto_with(
            &e,
            w,
            h,
            30.0,
            &[Batch::along(Style::default())],
            Some(&layer),
        );
        let (mut wet, mut dry) = (0, 0);
        for p in d.paths.iter().flatten() {
            let x = p[0] as usize;
            if x >= 152 {
                wet += 1;
            } else if x < 148 {
                dry += 1;
            }
        }
        assert!(wet > 50 && dry > 50, "lines on both: {wet} wet, {dry} dry");
    }

    #[test]
    fn order_cuts_travel() {
        let (w, h) = (400, 300);
        let d = draw(&island(w, h), w, h, 200.0, &Style::default());
        let mut shuffled = d.lines.clone();
        let mut r = rng::Rng::new(3);
        r.shuffle(&mut shuffled);
        assert!(pen_up_travel(&plot_order(shuffled.clone())) < 0.5 * pen_up_travel(&shuffled));
    }

    #[test]
    fn multi_angle_adds_lines() {
        let (w, h) = (200, 200);
        let e = island(w, h);
        let st = Style::default();
        let d0 = draw(&e, w, h, 200.0, &st);
        let mut st90 = st.clone();
        st90.batch_floor = 0.2;
        st90.mod_breaks = 0.8;
        let d1 = draw_batches(
            &e,
            w,
            h,
            200.0,
            &[
                Batch::along(st.clone()),
                Batch {
                    name: None,
                    angle: 90.0,
                    style: st90,
                },
            ],
        );
        assert!(
            d1.lines.len() > d0.lines.len(),
            "off={} on={}",
            d0.lines.len(),
            d1.lines.len()
        );
    }

    #[test]
    fn solid_dash_keeps_whole_lines() {
        let (w, h) = (120, 120);
        let mut e = vec![0.0f32; w * h];
        for y in 0..h {
            for x in 0..w {
                let dx = x as f32 - 60.0;
                let dy = y as f32 - 60.0;
                e[y * w + x] = (200.0 - (dx * dx + dy * dy).sqrt() * 2.0).max(0.0);
            }
        }
        let st = Style {
            dash_light: 1.0,
            dash_dark: 1.0,
            dash_drop: 0.0,
            space_dark: 0.5,
            space_light: 1.6,
            ..Style::default()
        };
        let d = draw(&e, w, h, 30.0, &st);
        assert_eq!(
            d.paths.len(),
            d.lines.len(),
            "each streamline stays one stroke"
        );
    }
}
