//! burin-map drawing engine: pen-and-ink terrain lines from an elevation grid.
//!
//! Pure computation, no I/O: give it heights in metres on a grid of square
//! pixels, get back dashes as polylines in pixel coordinates. The same code
//! runs natively, in the browser (WASM) and behind the Python package.
//!
//! Pipeline: fields (direction, input, spacing) → evenly spaced streamlines
//! → plotting order → input-driven dashes. Several batch passes (angles)
//! run in parallel and are merged.

mod color_expr;
mod dashes;
mod depth;
#[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
pub mod engine;
mod fields;
pub mod geojson;
#[cfg(feature = "gpu")]
pub mod gpu;
mod grid;
mod order;
mod pen;
mod rng;
mod streamlines;
mod style;
pub mod water;
mod wave;

pub use dashes::{dash_lines, dash_lines_by};
pub use fields::{batch_fields, build_fields, build_fields_with, land_mask, Fields};
pub use order::{pen_up_travel, plot_order};
pub use pen::Pen;
pub use streamlines::{trace_streamlines, trace_stripes, Line};
pub use style::{
    batches_inheriting, from_json_batches, from_json_style, to_json_batches, to_json_style, Batch,
    Param, Style, GROUPS,
};
pub use depth::{
    bathymetry_source_needs_fetch, build_depth_grid, depth_allows, expr_uses_depth, needs_depth,
    BathymetryDoc, WaterWhere,
};
pub use water::{Draw, Follow, Shoreline, WaterBatch, WaterLayer};

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
    /// Pens as runs over `paths`: each `(pen, end)` covers the paths from the
    /// previous run's end up to `end`.
    pub runs: Vec<(Pen, usize)>,
}

/// Draw with a single contour-parallel batch (the usual look).
pub fn draw(elev: &[f32], width: usize, height: usize, px_m: f32, style: &Style) -> Drawing {
    draw_batches(elev, width, height, px_m, &[Batch::along(style.clone())])
}

/// Draw one or more batch passes. `batches` must be non-empty. Terrain
/// fields (direction, input) are built once for each distinct set of terrain
/// parameters (see [`Style::is_terrain`]); each pass rotates its fields'
/// direction by its `angle` and uses its own space / dashes. Passes run in
/// parallel.
pub fn draw_batches(
    elev: &[f32],
    width: usize,
    height: usize,
    px_m: f32,
    batches: &[Batch],
) -> Drawing {
    assert_eq!(elev.len(), width * height, "elevation grid size mismatch");
    assert!(!batches.is_empty(), "need at least one batch");

    let fields: Vec<Fields> = terrain_groups(batches)
        .0
        .iter()
        .map(|&i| build_fields(elev, width, height, px_m, &batches[i].style))
        .collect();
    draw_fields(&fields, batches, elev)
}

/// Batches that share terrain fields: the first batch of each distinct set of
/// terrain parameters, and for every batch the index of its set in that list.
pub fn terrain_groups(batches: &[Batch]) -> (Vec<usize>, Vec<usize>) {
    let mut firsts: Vec<usize> = Vec::new();
    let of = batches
        .iter()
        .enumerate()
        .map(|(i, b)| {
            match firsts
                .iter()
                .position(|&j| batches[j].style.same_terrain(&b.style))
            {
                Some(g) => g,
                None => {
                    firsts.push(i);
                    firsts.len() - 1
                }
            }
        })
        .collect();
    (firsts, of)
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
        let fields: Vec<Fields> = terrain_groups(batches)
            .0
            .iter()
            .map(|&i| g.build_fields_blocking(elev, width, height, px_m, &batches[i].style))
            .collect();
        return draw_fields(&fields, batches, elev);
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
    let firsts = terrain_groups(batches).0;
    #[cfg(all(feature = "gpu", not(target_arch = "wasm32")))]
    if let Some(g) = engine::device() {
        let fields: Vec<Fields> = firsts
            .iter()
            .map(|&i| {
                g.build_fields_with_blocking(elev, width, height, px_m, &batches[i].style, coverage)
            })
            .collect();
        return draw_fields_timed(&fields, batches, water, elev, || 0.0).0;
    }
    let fields: Vec<Fields> = firsts
        .iter()
        .map(|&i| build_fields_with(elev, width, height, px_m, &batches[i].style, coverage))
        .collect();
    draw_fields_timed(&fields, batches, water, elev, || 0.0).0
}

/// `draw_batches` from fields already built (e.g. by `gpu::Gpu`): one per
/// entry of [`terrain_groups`], from that batch's style.
pub fn draw_fields(fields: &[Fields], batches: &[Batch], elev: &[f32]) -> Drawing {
    draw_fields_timed(fields, batches, None, elev, || 0.0).0
}

/// Milliseconds per phase of a draw, in the order they ran. Phases that run
/// once per batch carry the batch index in their label.
pub type Timings = Vec<(String, f64)>;

/// Streamlines of some batches, their dashes by line width, and the time
/// each phase took.
type Traced = (Vec<Line>, Vec<(Pen, Vec<Line>)>, Timings);

/// [`draw_fields`], timing each phase with `now` (milliseconds), with water
/// drawn alongside the land when there is some. The clock is passed in because
/// `std::time::Instant` panics on wasm32.
pub fn draw_fields_timed(
    fields: &[Fields],
    batches: &[Batch],
    water: Option<&WaterLayer>,
    elev: &[f32],
    now: fn() -> f64,
) -> (Drawing, Timings) {
    assert!(!batches.is_empty(), "need at least one batch");
    let (width, height) = (fields[0].w, fields[0].h);
    assert_eq!(elev.len(), width * height, "elevation grid size mismatch");
    let depth_store = water
        .filter(|wl| needs_depth(wl.batches, wl.shoreline))
        .map(|wl| build_depth_grid(elev, wl.coverage, wl.depth_override));
    let depth = depth_store.as_deref();
    let ((mut lines, dry, mut timings), wet) = rayon::join(
        || trace_land(fields, batches, elev, now),
        || water.map(|wl| trace_water(wl, width, height, depth, now)),
    );
    let mut pens: Vec<(Pen, Vec<Line>)> = Vec::new();
    let mut add = |groups: Vec<(Pen, Vec<Line>)>| {
        for (pen, p) in groups {
            match pens.iter_mut().find(|(q, _)| *q == pen) {
                Some((_, all)) => all.extend(p),
                None => pens.push((pen, p)),
            }
        }
    };
    add(dry);
    if let Some((l, p, t)) = wet {
        lines.extend(l);
        add(p);
        timings.extend(t);
    }
    let mut paths = Vec::new();
    let mut runs = Vec::new();
    if pens.iter().any(|(_, p)| !p.is_empty()) {
        let t = now();
        for (pen, p) in pens.into_iter().filter(|(_, p)| !p.is_empty()) {
            paths.extend(plot_order(p));
            runs.push((pen, paths.len()));
        }
        timings.push(("order dashes".into(), now() - t));
    }
    let d = Drawing {
        width,
        height,
        paths,
        lines,
        runs,
    };
    (d, timings)
}

fn trace_land(fields: &[Fields], batches: &[Batch], elev: &[f32], now: fn() -> f64) -> Traced {
    use rayon::prelude::*;
    let (firsts, of) = terrain_groups(batches);
    assert_eq!(
        fields.len(),
        firsts.len(),
        "one set of fields per terrain group"
    );
    let parts: Vec<Part> = batches
        .par_iter()
        .enumerate()
        .map(|(i, b)| {
            let mut st = b.style.clone();
            st.batch_seed = st.batch_seed.wrapping_add(i as u64 * 0x9E37_79B9);
            let t0 = now();
            let g = of[i];
            let bf = batch_fields(&fields[g], b, batches[firsts[g]].style.space);
            let t1 = now();
            let (lines, paths, t) = trace_batch(&bf, &st, false, now);
            let ctx = color_expr::ColorCtx {
                w: bf.w,
                h: bf.h,
                elev: Some(elev),
                tone: Some(&bf.input),
                dist: None,
                depth: None,
            };
            let ink = color_expr::colorize_paths(
                paths,
                st.batch_stroke,
                b.layer.clone(),
                b.color.as_deref(),
                b.color_expr.as_deref(),
                &ctx,
            );
            (lines, ink, [t1 - t0, t[0], t[1], t[2]])
        })
        .collect();
    collect_parts(parts, "batch")
}

fn trace_water(
    wl: &WaterLayer,
    w: usize,
    h: usize,
    depth: Option<&[f32]>,
    now: fn() -> f64,
) -> Traced {
    use rayon::prelude::*;
    let t0 = now();
    let (dist, outlines) = water::shore(wl.coverage, w, h);
    let t1 = now();
    let seeded = |i: usize, b: &WaterBatch| {
        let mut b = b.clone();
        b.style.batch_seed = b
            .style
            .batch_seed
            .wrapping_add(0x5EA ^ (i as u64 * 0x9E37_79B9));
        b
    };
    let parts: Vec<Part> = wl
        .batches
        .par_iter()
        .enumerate()
        .map(|(i, b)| {
            let st = seeded(i, b).style;
            let t0 = now();
            if b.draw != water::Draw::Lines {
                let b = seeded(i, b);
                let (marks, ms) = match b.draw {
                    water::Draw::Ripples => {
                        (
                            water::ripples(wl.coverage, &dist, depth, w, h, &b),
                            now() - t0,
                        )
                    }
                    _ => (
                        water::wave_marks(wl.coverage, &dist, depth, w, h, &b),
                        now() - t0,
                    ),
                };
                let f = water::water_fields(wl.coverage, &dist, depth, w, h, &b);
                let ctx = water_ctx(w, h, Some(&f.input), &dist, depth);
                let ink = color_expr::colorize_paths(
                    plot_order(marks),
                    st.batch_stroke,
                    b.layer.clone(),
                    b.color.as_deref(),
                    b.color_expr.as_deref(),
                    &ctx,
                );
                return (Vec::new(), ink, [ms, 0.0, 0.0, 0.0]);
            }
            let b = seeded(i, b);
            let f = water::water_fields(wl.coverage, &dist, depth, w, h, &b);
            let t1 = now();
            let (lines, paths, t) = trace_batch(&f, &st, true, now);
            let paths = water::filter_paths_by_depth(
                water::fray(paths, &dist, w, h, &st),
                depth,
                w,
                h,
                &b.r#where,
            );
            let ctx = water_ctx(w, h, Some(&f.input), &dist, depth);
            let ink = color_expr::colorize_paths(
                paths,
                st.batch_stroke,
                b.layer.clone(),
                b.color.as_deref(),
                b.color_expr.as_deref(),
                &ctx,
            );
            (lines, ink, [t1 - t0, t[0], t[1], t[2]])
        })
        .collect();
    let (mut lines, mut paths, mut timings) = collect_parts(parts, "water");
    timings.insert(0, ("water shore distance".into(), t1 - t0));
    if let Some(sh) = wl.shoreline {
        let t = now();
        let (l, p) = water::shoreline(&outlines, &dist, w, h, sh);
        timings.push(("water shoreline".into(), now() - t));
        lines.extend(l);
        let ctx = water_ctx(w, h, None, &dist, depth);
        paths.extend(color_expr::colorize_paths(
            p,
            sh.style.batch_stroke,
            sh
                .layer
                .clone()
                .or_else(|| Some("shoreline".into())),
            sh.color.as_deref(),
            sh.color_expr.as_deref(),
            &ctx,
        ));
    }
    (lines, paths, timings)
}

/// Trace, order and dash one batch on its own fields; ms for each of the three.
/// `whole` traces it in one piece rather than in stripes: slower on big
/// grids, but with no seam at all.
fn trace_batch(
    f: &Fields,
    st: &Style,
    whole: bool,
    now: fn() -> f64,
) -> (Vec<Line>, Vec<Line>, [f64; 3]) {
    let t0 = now();
    let phase = (st.mod_wave > 0.0 && st.mod_wave_even > 0.0)
        .then(|| wave::Phase::from_flow(&f.tx, &f.ty, f.w, f.h));
    // Even waves show the faintest seam, so such a batch is traced whole.
    let stripe = if whole || phase.is_some() {
        f.w.max(f.h) as i32
    } else {
        crate::STRIPE
    };
    let traced = wave::undulate(
        trace_stripes(f, st, stripe),
        st,
        st.batch_seed,
        phase.as_ref(),
    );
    let t1 = now();
    let lines = plot_order(traced);
    let t2 = now();
    let paths = dash_lines(&lines, &f.input, f.w, f.h, st);
    let t3 = now();
    (lines, paths, [t1 - t0, t2 - t1, t3 - t2])
}

/// One batch: streamlines, ink runs (pen + dashes), ms per phase.
type Part = (Vec<Line>, Vec<(Pen, Vec<Line>)>, [f64; 4]);

fn water_ctx<'a>(
    w: usize,
    h: usize,
    tone: Option<&'a [f32]>,
    dist: &'a [f32],
    depth: Option<&'a [f32]>,
) -> color_expr::ColorCtx<'a> {
    color_expr::ColorCtx {
        w,
        h,
        elev: None,
        tone,
        dist: Some(dist),
        depth,
    }
}

fn collect_parts(parts: Vec<Part>, kind: &str) -> Traced {
    let mut timings = Timings::new();
    let mut lines = Vec::new();
    let mut paths = Vec::new();
    for (i, (l, ink, t)) in parts.into_iter().enumerate() {
        for (name, ms) in ["fields", "trace", "order lines", "dashes"].iter().zip(t) {
            timings.push((format!("{kind} {i} {name}"), ms));
        }
        lines.extend(l);
        paths.extend(ink);
    }
    (lines, paths, timings)
}

impl Drawing {
    /// The paths, in drawing order, each run with its pen.
    pub fn pens(&self) -> impl Iterator<Item = (&Pen, &[Line])> {
        let mut start = 0;
        self.runs.iter().map(move |(pen, end)| {
            let run = &self.paths[start..*end];
            start = *end;
            (pen, run)
        })
    }

    /// SVG document. With `mm_per_px` it has a physical size in millimetres
    /// (coordinates stay in px; the viewBox scales them).
    pub fn to_svg(&self, mm_per_px: Option<f32>) -> String {
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
            r##"<svg xmlns="http://www.w3.org/2000/svg" {size} viewBox="0 0 {w} {h}"><g fill="none" stroke-linecap="round" stroke-linejoin="round">"##
        );
        s.push('\n');
        for (pen, run) in self.pens() {
            let layer = pen
                .layer
                .as_deref()
                .filter(|id| !id.is_empty())
                .map(svg_id)
                .unwrap_or_default();
            let _ = write!(
                s,
                r#"<g stroke="{}" stroke-width="{}"{layer}>"#,
                pen.color, pen.stroke
            );
            s.push('\n');
            for p in run {
                s.push_str("<path d=\"M");
                for (i, q) in p.iter().enumerate() {
                    if i > 0 {
                        s.push_str(" L");
                    }
                    let _ = write!(s, "{:.2} {:.2}", q[0], q[1]);
                }
                s.push_str("\"/>\n");
            }
            s.push_str("</g>\n");
        }
        s.push_str("</g></svg>\n");
        s
    }
}

fn svg_id(name: &str) -> String {
    let safe: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    format!(r#" id="{safe}""#)
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

        let dc = draw_fields(std::slice::from_ref(&cpu), &[Batch::along(st.clone())], &e);
        let dg = draw_fields(std::slice::from_ref(&gpu), &[Batch::along(st)], &e);
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
        let mut sh = Shoreline::default();
        sh.style.batch_stroke = 2.5;
        let layer = WaterLayer {
            coverage: &c,
            depth: None,
            depth_override: None,
            shoreline: Some(&sh),
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

        // The shoreline's own width is another pen, plotted after the rest.
        let pens: Vec<_> = d.pens().map(|(pen, run)| (pen.stroke, run.len())).collect();
        assert_eq!(pens.len(), 2, "{pens:?}");
        assert_eq!(pens[1].0, 2.5);
        assert_eq!(d.runs.last().unwrap().1, d.paths.len());
        let svg = d.to_svg(None);
        assert!(svg.contains(r#"stroke-width="2.5""#) && svg.contains(r#"stroke-width="1.2""#));
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
                    layer: None,
                    color: None,
                    color_expr: None,
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
    fn each_batch_can_have_its_own_sun() {
        let (w, h) = (200, 200);
        let e = island(w, h);
        let lit = |az: f32| Batch {
            name: None,
            layer: None,
            color: None,
            color_expr: None,
            angle: 90.0,
            style: Style {
                sun_azimuth: az,
                ..Style::default()
            },
        };
        let first = Batch::along(Style::default());
        let (firsts, of) = terrain_groups(&[first.clone(), lit(315.0), lit(135.0)]);
        assert_eq!((firsts, of), (vec![0, 2], vec![0, 0, 1]));

        let same = draw_batches(&e, w, h, 200.0, &[first.clone(), lit(315.0)]);
        let other = draw_batches(&e, w, h, 200.0, &[first, lit(135.0)]);
        assert_ne!(same.paths, other.paths, "the second sun is used");
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
