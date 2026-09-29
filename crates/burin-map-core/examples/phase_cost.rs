//! Where the time actually goes in one batch draw, at browser sizes.
//!
//! Answers a specific question before any parallel tracer is written: if
//! `trace_streamlines` became free, how much of the draw would be left? Only
//! the phases that would survive a striped tracer are worth optimising.
//!
//! cargo run --release --example phase_cost

use std::time::Instant;

use burin_map_core::{
    batch_fields, build_fields, dash_lines, plot_order, trace_streamlines, Batch, Style,
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

fn main() {
    let (w, h) = (3072, 2304);
    let e = island(w, h);
    let batches = [Batch::along(Style::default())];
    let st = &batches[0].style;

    let t = Instant::now();
    let f = build_fields(&e, w, h, 20.0, st);
    let fields = t.elapsed().as_secs_f64() * 1e3;

    let t = Instant::now();
    let bf = batch_fields(&f, &batches[0], st.space);
    let batch = t.elapsed().as_secs_f64() * 1e3;

    let t = Instant::now();
    let lines = trace_streamlines(&bf, st);
    let tracing = t.elapsed().as_secs_f64() * 1e3;

    let t = Instant::now();
    let ordered = plot_order(lines.clone());
    let order = t.elapsed().as_secs_f64() * 1e3;

    let t = Instant::now();
    let paths = dash_lines(&ordered, &f.input, w, h, st);
    let dashes = t.elapsed().as_secs_f64() * 1e3;

    let t = Instant::now();
    let _ordered2 = plot_order(paths.clone());
    let order2 = t.elapsed().as_secs_f64() * 1e3;

    let total = fields + batch + tracing + order + dashes + order2;
    println!(
        "\n{w}×{h}  one batch, {} lines → {} dashes",
        lines.len(),
        paths.len()
    );
    for (name, ms) in [
        ("build_fields", fields),
        ("batch_fields", batch),
        ("trace_streamlines", tracing),
        ("plot_order(lines)", order),
        ("dash_lines", dashes),
        ("plot_order(dashes)", order2),
    ] {
        println!("  {name:<20} {ms:8.1} ms   {:5.1}%", ms / total * 100.0);
    }
    println!("  {:<20} {total:8.1} ms", "TOTAL");
    println!(
        "\n  tracing is {:.0}% of the draw; making it free leaves {:.1}× the speed",
        tracing / total * 100.0,
        total / (total - tracing)
    );
}
