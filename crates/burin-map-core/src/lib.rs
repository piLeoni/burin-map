//! burin-map drawing engine: pen-and-ink terrain lines from an elevation grid.
//!
//! Pure computation, no I/O: give it heights in metres on a grid of square
//! pixels, get back dashes as polylines in pixel coordinates. The same code
//! runs natively, in the browser (WASM) and behind the Python package.
//!
//! Pipeline: fields (direction, tone, spacing) → evenly spaced streamlines
//! → plotting order → tone-driven dashes. Several hatching passes (angles)
//! run in parallel and are merged.

mod dashes;
mod fields;
mod grid;
mod order;
mod rng;
mod streamlines;
mod style;

pub use dashes::dash_lines;
pub use fields::{build_fields, hatch_fields, Fields};
pub use order::{pen_up_travel, plot_order};
pub use streamlines::{trace_streamlines, Line};
pub use style::{Hatch, Style};

pub struct Drawing {
    pub width: usize,
    pub height: usize,
    /// Dashes in drawing order.
    pub paths: Vec<Line>,
    /// Streamlines before dashing, in drawing order.
    pub lines: Vec<Line>,
}

/// Draw with a single contour-parallel hatch (the usual look).
pub fn draw(elev: &[f32], width: usize, height: usize, px_m: f32, style: &Style) -> Drawing {
    draw_hatches(elev, width, height, px_m, &[Hatch::along(style.clone())])
}

/// Draw one or more hatching passes. `hatches` must be non-empty. Terrain
/// fields (direction, tone) come from the first hatch's style; each pass
/// rotates that direction by its `angle` and uses its own spacing / dashes.
/// Passes run in parallel.
pub fn draw_hatches(elev: &[f32], width: usize, height: usize, px_m: f32, hatches: &[Hatch]) -> Drawing {
    assert_eq!(elev.len(), width * height, "elevation grid size mismatch");
    assert!(!hatches.is_empty(), "need at least one hatch");

    let base = &hatches[0].style;
    let f = build_fields(elev, width, height, px_m, base);
    let base_spacing = base.spacing;

    let parts: Vec<(Vec<Line>, Vec<Line>)> = {
        use rayon::prelude::*;
        hatches
            .par_iter()
            .enumerate()
            .map(|(i, h)| {
                let mut st = h.style.clone();
                st.seed = st.seed.wrapping_add(i as u64 * 0x9E37_79B9);
                let hf = hatch_fields(&f, h, base_spacing);
                let lines = plot_order(trace_streamlines(&hf, &st));
                let paths = dash_lines(&lines, &f.tone, width, height, &st);
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
        assert!(f.tone[190 * w + 190] > f.tone[110 * w + 110]);
    }

    #[test]
    fn flat_ground_sits_at_flat_level() {
        let (w, h) = (80, 80);
        let mut e = vec![100.0f32; w * h];
        e[40 * w + 40] = 101.0;
        let f = build_fields(&e, w, h, 200.0, &Style::default());
        let mut mid: Vec<f32> = (20..60)
            .flat_map(|y| (20..60).map(move |x| (x, y)))
            .map(|(x, y)| f.tone[y * w + x])
            .collect();
        mid.sort_by(|a, b| a.total_cmp(b));
        assert!((mid[mid.len() / 2] - Style::default().flat_level).abs() < 0.02);
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
        st90.hatch_floor = 0.2;
        st90.breaks = 0.8;
        let d1 = draw_hatches(
            &e,
            w,
            h,
            200.0,
            &[Hatch::along(st.clone()), Hatch { angle: 90.0, style: st90 }],
        );
        assert!(d1.lines.len() > d0.lines.len(), "off={} on={}", d0.lines.len(), d1.lines.len());
    }

    #[test]
    fn shade_dash_zero_keeps_whole_lines() {
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
        st.shade_dash = 0.0;
        st.shade_gap = 1.0;
        st.spacing_dark = 0.5;
        st.spacing_lit = 1.6;
        let d = draw(&e, w, h, 30.0, &st);
        assert_eq!(d.paths.len(), d.lines.len(), "each streamline stays one stroke");
    }
}
