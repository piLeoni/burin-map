//! A document script becomes the atlas the engine draws, and its mistakes
//! come back with a position the editor can underline.

use crate::evaluate;
use serde_json::json;

#[test]
fn a_script_builds_a_batch_and_a_water_pass() {
    let atlas = evaluate(
        r##"
        burin
          .atlas()
          .batch(burin.batch({ shade: burin.lerp("#e8dcc8", "#2a1810", burin.tone), angle: 45 }))
          .water(burin.water("dots", { color: "#9db4c0" }))
          .draw();
        "##,
    )
    .expect("the script runs");

    let items = atlas.as_array().expect("an atlas is a list");
    assert_eq!(items.len(), 2, "one batch and one water pass");
    assert_eq!(items[0]["angle"].as_f64(), Some(45.0));
    assert_eq!(items[1]["kind"], json!("water"));
    assert_eq!(items[1]["draw"], json!("dots"));
}

#[test]
fn a_loop_over_bands_makes_stacked_contours() {
    let atlas = evaluate(
        r##"
        const doc = burin.atlas();
        for (const [from, to] of [[0, 200], [200, 400], [400, 600]])
          doc.batch(burin.fill({ color: "#8a7a5c", where: { elevMin: from, elevMax: to } }));
        doc.draw();
        "##,
    )
    .expect("the script runs");

    let items = atlas.as_array().unwrap();
    assert_eq!(items.len(), 3, "three belts");
    // The bounded belts keep both edges; a `0` lower edge is the default and
    // is left out of the saved JSON, so read the band through the engine.
    let parsed = burin_map_core::script::atlas_from(&atlas).expect("the engine reads it");
    assert_eq!(parsed.batches.len(), 3, "three fill batches");
    for (i, belt) in parsed.batches.iter().enumerate() {
        assert_eq!(belt.band.elev_min, Some(i as f32 * 200.0));
        assert_eq!(belt.band.elev_max, Some((i as f32 + 1.0) * 200.0));
    }
}

#[test]
fn json_saves_the_atlas_without_drawing() {
    let atlas = evaluate(r#"burin.atlas().batch(burin.batch({ style: { spacing: 1 } })).json();"#)
        .expect("the script runs");
    assert_eq!(atlas.as_array().unwrap().len(), 1);
}

#[test]
fn a_bad_option_is_reported_with_a_position() {
    let error = evaluate("burin.atlas().batch(burin.batch({ angle: 'steep' })).draw();").unwrap_err();
    assert!(error.at.is_some(), "the position is known: {error}");
    assert!(error.message.contains("angle"), "the message names the key: {error}");
}

#[test]
fn an_empty_document_is_an_error() {
    let error = evaluate("burin.atlas().draw();").unwrap_err();
    assert!(error.message.contains("nothing to draw"), "{error}");
}

#[test]
fn a_script_that_never_draws_is_an_error() {
    let error = evaluate("burin.batch({ angle: 0 });").unwrap_err();
    assert!(error.message.contains("never called"), "{error}");
}

#[test]
fn a_syntax_error_has_a_line() {
    let error = evaluate("const x = ;").unwrap_err();
    assert!(error.at.is_some(), "{error}");
}
