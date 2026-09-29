//! Greyscale PNG: black round-capped strokes on white, antialiased.

use burin_map_core::Drawing;
use tiny_skia::{LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform};

use crate::Error;

pub fn to_png(d: &Drawing) -> Result<Vec<u8>, Error> {
    let (width, height) = (d.width, d.height);
    let mut pm = Pixmap::new(width as u32, height as u32)
        .ok_or_else(|| Error::Input(format!("bad image size {width}x{height}")))?;
    pm.fill(tiny_skia::Color::WHITE);
    let mut paint = Paint::default();
    paint.set_color_rgba8(0, 0, 0, 255);
    paint.anti_alias = true;
    for (stroke, paths) in d.pens() {
        let st = Stroke {
            width: stroke,
            line_cap: LineCap::Round,
            line_join: LineJoin::Round,
            ..Stroke::default()
        };
        // One path per batch of dashes: far fewer calls than one per dash.
        for chunk in paths.chunks(256) {
            let mut pb = PathBuilder::new();
            for p in chunk {
                if let Some(first) = p.first() {
                    pb.move_to(first[0], first[1]);
                    if p.len() == 1 {
                        pb.line_to(first[0], first[1]);
                    }
                    for q in &p[1..] {
                        pb.line_to(q[0], q[1]);
                    }
                }
            }
            if let Some(path) = pb.finish() {
                pm.stroke_path(&path, &paint, &st, Transform::identity(), None);
            }
        }
    }

    // Opaque on white, so any channel is the grey level.
    let grey: Vec<u8> = pm
        .data()
        .as_chunks::<4>()
        .0
        .iter()
        .map(|px| px[0])
        .collect();
    let mut out = Vec::new();
    {
        let mut enc = png::Encoder::new(&mut out, width as u32, height as u32);
        enc.set_color(png::ColorType::Grayscale);
        enc.set_depth(png::BitDepth::Eight);
        let mut w = enc.write_header().map_err(|e| Error::Data(e.to_string()))?;
        w.write_image_data(&grey)
            .map_err(|e| Error::Data(e.to_string()))?;
    }
    Ok(out)
}
