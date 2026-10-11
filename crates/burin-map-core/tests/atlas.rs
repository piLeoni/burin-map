//! The typed batch API draws what the same document draws: one engine, one
//! answer, whichever way the batches were written.

use burin_map_core::atlas::{Atlas, WaterDraw};
use burin_map_core::water::{Shoreline, WaterBatch};
use burin_map_core::{
    from_json_batches, to_json_batches, Batch, Color, Drawing, Field, Region, Ring, Surface,
};

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
    let from_atlas = atlas.draw(&Surface::new(&e, w, h, 200.0), None).unwrap();

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
    let d = atlas.draw(&Surface::new(&e, w, h, 200.0), None).unwrap();

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

/// Ink (path length) left and right of `x`.
fn ink_either_side(d: &Drawing, x: f32) -> (f32, f32) {
    let (mut left, mut right) = (0.0, 0.0);
    for p in &d.paths {
        for s in p.windows(2) {
            let len = (s[1][0] - s[0][0]).hypot(s[1][1] - s[0][1]);
            if (s[0][0] + s[1][0]) / 2.0 < x {
                left += len;
            } else {
                right += len;
            }
        }
    }
    (left, right)
}

/// The left half of the island's grid, as a polygon in pixels.
fn west(w: usize, h: usize) -> Region {
    let (w, h) = (w as f64, h as f64);
    Region::Rings(vec![Ring {
        exterior: true,
        points: vec![[0.0, 0.0], [w / 2.0, 0.0], [w / 2.0, h], [0.0, h]],
    }])
}

fn draw_with(batch: Batch, regions: &[(&str, Region)]) -> Result<Drawing, String> {
    let (w, h) = (320, 240);
    let e = island(w, h);
    let mut atlas = Atlas::new();
    atlas.batch(batch);
    atlas.draw(&Surface::new(&e, w, h, 200.0).regions(regions), None)
}

#[test]
fn regions_nobody_names_change_nothing() {
    let plain = draw_with(Batch::lines(), &[]).unwrap();
    let given = draw_with(Batch::lines(), &[("west", west(320, 240))]).unwrap();
    assert_eq!(plain.paths, given.paths);
}

#[test]
fn within_keeps_the_lines_to_a_region_and_outside_mirrors_it() {
    let regions = [("west", west(320, 240))];
    let (all_l, all_r) = ink_either_side(&draw_with(Batch::lines(), &[]).unwrap(), 160.0);
    let (in_l, in_r) = ink_either_side(&draw_with(Batch::lines().within("west"), &regions).unwrap(), 160.0);
    let (out_l, out_r) =
        ink_either_side(&draw_with(Batch::lines().outside("west"), &regions).unwrap(), 160.0);
    assert!(all_l > 0.0 && all_r > 0.0, "the island spans both halves");
    assert_eq!(in_r, 0.0, "nothing east of the edge");
    assert!(in_l > 0.0 && in_l < all_l, "thinner west, fading towards the edge");
    assert_eq!(out_l, 0.0, "nothing west of the edge");
    assert!(out_r > 0.0 && out_r < all_r);
}

#[test]
fn fraying_breaks_lines_up_near_the_edge() {
    let regions = [("west", west(320, 240))];
    let clean = draw_with(Batch::lines().within("west").set("mask_reach", 1.0), &regions).unwrap();
    let frayed = draw_with(
        Batch::lines().within("west").set("mask_reach", 1.0).set("mask_fray", 40.0),
        &regions,
    )
    .unwrap();
    // Ink between x = a and b.
    let band = |d: &Drawing, a: f32, b: f32| ink_either_side(d, b).0 - ink_either_side(d, a).0;
    assert!(
        band(&frayed, 130.0, 160.0) < 0.8 * band(&clean, 130.0, 160.0),
        "thinner near the edge"
    );
    let (far_f, far_c) = (band(&frayed, 0.0, 90.0), band(&clean, 0.0, 90.0));
    assert!((far_f - far_c).abs() < 0.02 * far_c, "untouched well inside: {far_f} vs {far_c}");
    assert_eq!(ink_either_side(&frayed, 160.0).1, 0.0);
}

#[test]
fn a_region_nobody_gave_is_an_error() {
    let Err(e) = draw_with(Batch::lines().named("hills").within("parks"), &[]) else {
        panic!("drew without the region");
    };
    assert!(e.contains("\"hills\"") && e.contains("\"parks\""), "{e}");
}
