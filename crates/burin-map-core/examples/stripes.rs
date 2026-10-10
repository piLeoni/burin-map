//! Striped tracing against the sequential tracer: speed, and whether the
//! drawing is still the same drawing.
//!
//! Quality is measured on the finished dashes, not on the lines, because
//! that is what the eye sees: how much ink is on the paper, how it spreads
//! over the ground, and whether the stripe seams carry more or less of it.
//!
//! cargo run --release --example stripes

use std::time::Instant;

use burin_map_core::{
    batch_fields, build_fields, dash_lines, trace_streamlines, trace_stripes, Batch, Style,
};

fn island(w: usize, h: usize) -> Vec<f32> {
    let mut e = vec![0.0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            let bump = |cx: f32, cy: f32, r: f32, hgt: f32| {
                hgt * (-((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)) / (2.0 * r * r)).exp()
            };
            e[y * w + x] = (bump(w as f32 * 0.5, h as f32 * 0.5, w as f32 * 0.2, 1500.0)
                + bump(w as f32 * 0.85, h as f32 * 0.55, w as f32 * 0.12, 900.0)
                + bump(w as f32 * 0.3, h as f32 * 0.8, w as f32 * 0.1, 700.0)
                - 150.0)
                .max(0.0);
        }
    }
    e
}

/// Calls `each(x_mid, len)` for every segment of every polyline.
fn segments(paths: &[Vec<[f32; 2]>], mut each: impl FnMut(f32, f32, f64)) {
    for p in paths {
        for k in 1..p.len() {
            let (a, b) = (p[k - 1], p[k]);
            each(
                (a[0] + b[0]) * 0.5,
                (a[1] + b[1]) * 0.5,
                ((b[0] - a[0]) as f64).hypot((b[1] - a[1]) as f64),
            );
        }
    }
}

/// Total inked length, in px.
fn ink(paths: &[Vec<[f32; 2]>]) -> f64 {
    let mut total = 0.0;
    segments(paths, |_, _, len| total += len);
    total
}

/// Ink per ground pixel over cells `~10 space` wide: the 5th percentile (the
/// barest cells) and the median. A broken rule or a hole moves the first.
fn density(paths: &[Vec<[f32; 2]>], f: &burin_map_core::Fields, space: f32) -> (f64, f64) {
    let cell = (space * 10.0).max(16.0);
    let gw = (f.w as f32 / cell).ceil() as usize + 1;
    let gh = (f.h as f32 / cell).ceil() as usize + 1;
    let at = |x: f32, y: f32| (x / cell) as usize + gw * ((y / cell) as usize);
    let mut ink_cell = vec![0.0f64; gw * gh];
    let mut ground = vec![0usize; gw * gh];
    for y in 0..f.h {
        for x in 0..f.w {
            if f.walkable[y * f.w + x] {
                ground[at(x as f32, y as f32)] += 1;
            }
        }
    }
    segments(paths, |x, y, len| {
        if x >= 0.0 && y >= 0.0 && x < f.w as f32 && y < f.h as f32 {
            ink_cell[at(x, y)] += len;
        }
    });
    let mut vals: Vec<f64> = (0..gw * gh)
        .filter(|&i| ground[i] >= 64)
        .map(|i| ink_cell[i] / ground[i] as f64)
        .collect();
    if vals.is_empty() {
        return (0.0, 0.0);
    }
    vals.sort_by(|a, b| a.total_cmp(b));
    (vals[vals.len() / 20], vals[vals.len() / 2])
}

/// Ink within 8 px of a seam, as a ratio to the same band in `reference`.
/// Above 1 the seams are denser than the sequential drawing, below 1 barer.
fn seam_ratio(paths: &[Vec<[f32; 2]>], reference: &[Vec<[f32; 2]>], stripe: i32) -> f64 {
    let near = |x: f32| {
        let m = x.rem_euclid(stripe as f32);
        x > 8.0 && m.min(stripe as f32 - m) <= 8.0
    };
    let band = |p: &[Vec<[f32; 2]>]| {
        let mut total = 0.0;
        segments(p, |x, _, len| {
            if near(x) {
                total += len
            }
        });
        total
    };
    band(paths) / band(reference)
}

fn main() {
    let threads = rayon::current_num_threads();
    let (w, h) = (3072usize, 2304usize);
    let e = island(w, h);
    let batches = [Batch::along(Style::default())];
    let st = &batches[0].style;
    let f = build_fields(&e, w, h, 20.0, st);
    let bf = batch_fields(&f, &batches[0], st.space_base());

    let _ = trace_streamlines(&bf, st);
    let t = Instant::now();
    let seq = trace_streamlines(&bf, st);
    let seq_ms = t.elapsed().as_secs_f64() * 1e3;
    let seq_d = dash_lines(&seq, &f.input, w, h, st);
    let (s_p5, s_med) = density(&seq_d, &f, st.space_base());
    let seq_ink = ink(&seq_d);
    println!("\n{w}×{h}, {threads} threads; density is ink px per ground px");
    println!(
        "sequential: {seq_ms:4.0} ms · {:6} dashes · ink 100% · density p5 {:.4} median {:.4}",
        seq_d.len(),
        s_p5,
        s_med
    );

    for stripe in [256i32, 384, 512] {
        let t = Instant::now();
        let par = trace_stripes(&bf, st, stripe);
        let ms = t.elapsed().as_secs_f64() * 1e3;
        let par_d = dash_lines(&par, &f.input, w, h, st);
        let (p5, med) = density(&par_d, &f, st.space_base());
        println!(
            "stripe {stripe:>3}: {ms:4.0} ms · {:6} dashes · ink {:3.0}% · density p5 {:.4} median {:.4} · seams {:.2}×",
            par_d.len(),
            100.0 * ink(&par_d) / seq_ink,
            p5,
            med,
            seam_ratio(&par_d, &seq_d, stripe)
        );
    }
}
