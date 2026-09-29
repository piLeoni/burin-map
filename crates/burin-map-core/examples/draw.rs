//! Draw a raw elevation grid and time each stage.
//!
//!     cargo run --release --example draw -- dem.f32 WIDTH HEIGHT PX_M out.svg
//!
//! dem.f32: little-endian f32 heights in metres, row 0 = north.
//! With BURIN_DUMP=dir, also writes the fields (tx, ty, input, space) there as
//! raw f32, for comparing against the Python implementation.

use std::time::Instant;

use burin_map_core::{
    build_fields, dash_lines, pen_up_travel, plot_order, trace_streamlines, Drawing, Style,
};

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 6 {
        eprintln!("usage: draw dem.f32 WIDTH HEIGHT PX_M out.svg");
        std::process::exit(2);
    }
    let (w, h): (usize, usize) = (a[2].parse().unwrap(), a[3].parse().unwrap());
    let px_m: f32 = a[4].parse().unwrap();
    let raw = std::fs::read(&a[1]).expect("read dem");
    let elev: Vec<f32> = raw
        .as_chunks::<4>()
        .0
        .iter()
        .map(|b| f32::from_le_bytes(*b))
        .collect();
    let st = Style::default();

    #[cfg(feature = "gpu")]
    let gpu = std::env::var("BURIN_GPU").is_ok().then(|| {
        let t = Instant::now();
        let g = burin_map_core::gpu::Gpu::new_blocking().expect("no GPU adapter");
        println!(
            "gpu init    {:7.0} ms  ({})",
            t.elapsed().as_secs_f64() * 1000.0,
            g.name
        );
        let t = Instant::now();
        g.build_fields_blocking(&elev, w, h, px_m, &st);
        println!("gpu warm-up {:7.0} ms", t.elapsed().as_secs_f64() * 1000.0);
        g
    });

    let t0 = Instant::now();
    #[cfg(feature = "gpu")]
    let f = match &gpu {
        Some(g) => g.build_fields_blocking(&elev, w, h, px_m, &st),
        None => build_fields(&elev, w, h, px_m, &st),
    };
    #[cfg(not(feature = "gpu"))]
    let f = build_fields(&elev, w, h, px_m, &st);
    let t1 = Instant::now();
    if let Ok(dir) = std::env::var("BURIN_DUMP") {
        for (name, v) in [
            ("tx", &f.tx),
            ("ty", &f.ty),
            ("input", &f.input),
            ("space", &f.space),
        ] {
            let bytes: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
            std::fs::write(format!("{dir}/{name}.f32"), bytes).unwrap();
        }
    }
    let lines = trace_streamlines(&f, &st);
    let t2 = Instant::now();
    let lines = plot_order(lines);
    let t3 = Instant::now();
    let paths = dash_lines(&lines, &f.input, w, h, &st);
    let t4 = Instant::now();

    let ms = |a: Instant, b: Instant| (b - a).as_secs_f64() * 1000.0;
    println!("fields      {:7.0} ms", ms(t0, t1));
    println!("streamlines {:7.0} ms  ({} lines)", ms(t1, t2), lines.len());
    println!("order       {:7.0} ms", ms(t2, t3));
    println!(
        "dashes      {:7.0} ms  ({} dashes)",
        ms(t3, t4),
        paths.len()
    );
    println!("total       {:7.0} ms", ms(t0, t4));
    println!("pen-up travel {:.0} px", pen_up_travel(&paths));

    let d = Drawing {
        width: w,
        height: h,
        strokes: vec![(st.batch_stroke, paths.len())],
        paths,
        lines,
    };
    std::fs::write(&a[5], d.to_svg(None)).expect("write svg");
}
