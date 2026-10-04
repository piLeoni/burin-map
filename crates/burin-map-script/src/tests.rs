//! A document script becomes the atlas the engine draws, and its mistakes
//! come back with a position the editor can underline.

use crate::{evaluate, evaluate_document};
use serde_json::json;

#[test]
fn a_script_builds_a_batch_and_a_water_pass() {
    let atlas = evaluate(
        r##"
        burin.batch({ shade: burin.lerp("#e8dcc8", "#2a1810", burin.tone), angle: 45 });
        burin.water("dots", { color: "#9db4c0" });
        burin.draw();
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
        for (const [from, to] of [[0, 200], [200, 400], [400, 600]])
          burin.fill({ color: "#8a7a5c", where: { elevMin: from, elevMax: to } });
        burin.draw();
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
    let atlas = evaluate(r#"burin.batch({ style: { space: 1 } }); burin.json();"#)
        .expect("the script runs");
    assert_eq!(atlas.as_array().unwrap().len(), 1);
}

#[test]
fn a_script_can_set_the_area() {
    let doc = evaluate_document(
        r#"burin.area({ at: "37.75,-122.45", size_km: [8, 8] }); burin.batch({}); burin.draw();"#,
    )
    .expect("the script runs");
    let area = doc.area.expect("the area is kept");
    assert_eq!(area["at"], json!("37.75,-122.45"));
    assert_eq!(area["size_km"], json!([8, 8]));
    assert_eq!(doc.atlas.as_array().unwrap().len(), 1);
}

#[test]
fn an_unknown_area_key_is_reported() {
    let error = evaluate(r#"burin.area({ where: "nowhere" }); burin.batch({}); burin.draw();"#)
        .unwrap_err();
    assert!(error.message.contains("where"), "{error}");
}

#[test]
fn a_bad_option_is_reported_with_a_position() {
    let error = evaluate("burin.batch({ angle: 'steep' }); burin.draw();").unwrap_err();
    assert!(error.at.is_some(), "the position is known: {error}");
    assert!(error.message.contains("angle"), "the message names the key: {error}");
}

#[test]
fn an_empty_document_is_an_error() {
    let error = evaluate("burin.draw();").unwrap_err();
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

#[test]
fn every_shipped_preset_runs() {
    // The template scripts the CLI and the page both offer. A broken one is a
    // broken first impression, so they are all run here.
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../presets");
    let mut checked = 0;
    for entry in std::fs::read_dir(&dir).expect("presets/ is there") {
        let path = entry.expect("readable").path();
        if path.extension().and_then(|e| e.to_str()) != Some("js") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("readable");
        let doc = evaluate_document(&source)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        assert!(
            doc.atlas.as_array().is_some_and(|a| !a.is_empty()),
            "{}: draws nothing",
            path.display()
        );
        checked += 1;
    }
    assert!(checked >= 1, "at least one preset ships");
}
