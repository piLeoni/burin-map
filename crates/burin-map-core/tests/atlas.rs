//! The typed batch API draws what the same document draws: one engine, one
//! answer, whichever way the batches were written.

use burin_map_core::atlas::{Atlas, WaterDraw};
use burin_map_core::water::{Shoreline, WaterBatch};
use burin_map_core::{from_json_batches, to_json_batches, Batch, Color, Field, Style};

fn island(w: usize, h: usize) -> Vec<f32> {
    let mut e = vec![0.0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            let bump = |cx: f32, cy: f32, r: f32, hgt: f32| {
                hgt * (-((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)) / (2.0 * r * r)).exp()
            };
            e[y * w + x] =
                (bump(150.0, 150.0, 60.0, 1500.0) + bump(270.0, 170.0, 40.0, 900.0) - 150.0)
                    .max(0.0);
        }
    }
    e
}

/// A document with two land batches and water on it, and the tree that builds
/// exactly it.
fn document() -> serde_json::Value {
    serde_json::json!([
        {"name": "hills", "color_expr": ["lerp", "#e8dcc8", "#2a1810", "tone"]},
        {"name": "coast", "angle": 30.0, "style": {"space": 6.0}, "width_expr": ["lerp", 0.5, 2, "tone"]},
        {"draw": "fill", "name": "sea", "ground": "sea", "color": "#bcd6e2"}
    ])
}

#[test]
fn the_typed_api_draws_the_document() {
    let (w, h) = (320, 240);
    let e = island(w, h);

    let doc = from_json_batches(&document().to_string()).unwrap();
    let from_doc = burin_map_core::draw_batches(&e, w, h, 200.0, &doc);

    let mut atlas = Atlas::new();
    atlas.batch(
        Batch::lines()
            .named("hills")
            .shade(Color::mix(
                Color::hex("#e8dcc8").unwrap(),
                Color::hex("#2a1810").unwrap(),
                Field::Tone,
            )),
    );
    atlas.batch(
        Batch::lines()
            .named("coast")
            .angle(30.0)
            .set("space", 6.0)
            .width(burin_map_core::Scalar::Call(
                burin_map_core::Func::Lerp,
                vec![
                    burin_map_core::Scalar::Lit(0.5),
                    burin_map_core::Scalar::Lit(2.0),
                    burin_map_core::Scalar::Field(Field::Tone),
                ],
            )),
    );
    atlas.batch(
        Batch::filling(Some("#bcd6e2"), None, Some("sea")).ground(burin_map_core::Ground::Sea),
    );
    let from_atlas = atlas.draw(&e, w, h, 200.0);

    assert_eq!(from_atlas.paths.len(), from_doc.paths.len());
    assert_eq!(from_atlas.runs, from_doc.runs);
    assert_eq!(from_atlas.fills.len(), from_doc.fills.len());
}

#[test]
fn the_atlas_saves_as_the_document_saves() {
    // A tree built by hand comes back out as the same JSON a document would
    // carry, so a script's batches can be saved like a preset's.
    let mut atlas = Atlas::new();
    atlas.batch(
        Batch::lines()
            .named("hills")
            .shade(Color::mix(Color::black(), Color::white(), Field::Dist)),
    );
    let json = to_json_batches(&atlas.batches);
    let back = from_json_batches(&json).unwrap();
    assert_eq!(to_json_batches(&back), json);
}

#[test]
fn water_passes_draw_on_the_sea() {
    let (w, h) = (320, 240);
    let e = island(w, h);

    let mut atlas = Atlas::new();
    atlas.batch(Batch::filling(Some("#cfe3ec"), None, Some("sea")).ground(burin_map_core::Ground::Sea));
    atlas.water(WaterBatch {
        name: Some("hachures".into()),
        color: Some("#33556b".into()),
        draw: WaterDraw::Lines,
        ..WaterBatch::default()
    });
    atlas.shore(Shoreline::default());
    let d = atlas.draw(&e, w, h, 200.0);

    assert!(!d.paths.is_empty(), "the sea draws something");
    assert!(
        d.runs.iter().any(|(p, _)| p.layer.as_deref() == Some("hachures")),
        "the water pass has its own layer"
    );
    assert!(
        d.runs.iter().any(|(p, _)| p.layer.as_deref() == Some("shoreline")),
        "so does the shore"
    );
}
