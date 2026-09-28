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
mod fields;
pub mod geojson;
#[cfg(feature = "gpu")]
pub mod gpu;
mod grid;
mod order;
mod rng;
mod streamlines;
mod style;

pub use dashes::dash_lines;
pub use fields::{batch_fields, build_fields, Fields};
pub use order::{pen_up_travel, plot_order};
pub use streamlines::{trace_streamlines, Line};
pub use style::{from_json_batches, from_json_style, to_json_batches, to_json_style, Batch, Param, Style, GROUPS};

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
        (self.nw_lon + x * self.deg_per_px_lon, self.nw_lat - y * self.deg_per_px_lat)
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
pub fn draw_batches(elev: &[f32], width: usize, height: usize, px_m: f32, batches: &[Batch]) -> Drawing {
    assert_eq!(elev.len(), width * height, "elevation grid size mismatch");
    assert!(!batches.is_empty(), "need at least one batch");

    let f = build_fields(elev, width, height, px_m, &batches[0].style);
    draw_fields(&f, batches)
}

/// `draw_batches` from fields already built (e.g. by `gpu::Gpu`) with the
/// first batch's style.
pub fn draw_fields(f: &Fields, batches: &[Batch]) -> Drawing {
    assert!(!batches.is_empty(), "need at least one batch");
    let (width, height) = (f.w, f.h);
    let base_space = batches[0].style.space;

    let parts: Vec<(Vec<Line>, Vec<Line>)> = {
        use rayon::prelude::*;
        batches
            .par_iter()
            .enumerate()
            .map(|(i, b)| {
                let mut st = b.style.clone();
                st.batch_seed = st.batch_seed.wrapping_add(i as u64 * 0x9E37_79B9);
                let bf = batch_fields(f, b, base_space);
                let lines = plot_order(trace_streamlines(&bf, &st));
                let paths = dash_lines(&lines, &f.input, width, height, &st);
                (lines, paths)
            })
            .collect()
    };

    let mut lines = Vec::new();
    let mut paths = Vec::new();
    for (l, p) in parts {
        lines.extend(l);
        paths.extend(p);
    }
    if !paths.is_empty() {
        paths = plot_order(paths);
    }
    Drawing { width, height, paths, lines }
}

impl Drawing {
    /// SVG document. With `mm_per_px` it has a physical size in millimetres
    /// (coordinates stay in px; the viewBox scales them).
    pub fn to_svg(&self, stroke: f32, mm_per_px: Option<f32>) -> String {
        use std::fmt::Write;
        let (w, h) = (self.width, self.height);
        let size = match mm_per_px {
            Some(k) => format!(r#"width="{:.2}mm" height="{:.2}mm""#, w as f32 * k, h as f32 * k),
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
                    hgt * (-((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)) / (2.0 * r * r)).exp()
                };
                e[y * w + x] =
                    (bump(150.0, 150.0, 60.0, 1500.0) + bump(270.0, 170.0, 40.0, 900.0) - 150.0).max(0.0);
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
        assert!((0.8..1.25).contains(&ratio), "lines {} vs {}", dc.lines.len(), dg.lines.len());
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
            &[Batch::along(st.clone()), Batch { name: None, angle: 90.0, style: st90 }],
        );
        assert!(d1.lines.len() > d0.lines.len(), "off={} on={}", d0.lines.len(), d1.lines.len());
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
        let mut st = Style::default();
        st.dash_light = 1.0;
        st.dash_dark = 1.0;
        st.dash_drop = 0.0;
        st.space_dark = 0.5;
        st.space_light = 1.6;
        let d = draw(&e, w, h, 30.0, &st);
        assert_eq!(d.paths.len(), d.lines.len(), "each streamline stays one stroke");
    }
}
