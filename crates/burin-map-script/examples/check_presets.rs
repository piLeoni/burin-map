//! Run every document in `presets/` and write (or check) the atlases the web
//! page ships, so a preset draws on the page without the script interpreter.
//!
//! ```text
//! cargo run -p burin-map-script --example check_presets            # check
//! cargo run -p burin-map-script --example check_presets -- --write # rewrite
//! ```
//!
//! It fails when a preset stops running, draws nothing, or its saved atlas is
//! stale — the same guard `check_web_presets` is in draw.land.

use std::collections::BTreeMap;
use std::path::PathBuf;

fn presets_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../presets")
}

fn output_file() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../web/src/presets/atlases.json")
}

fn main() {
    let write = std::env::args().any(|a| a == "--write");
    let dir = presets_dir();
    let mut atlases: BTreeMap<String, serde_json::Value> = BTreeMap::new();

    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("reading {}: {e}", dir.display()))
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("js"))
        .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
        .collect();
    names.sort();
    assert!(!names.is_empty(), "no preset scripts in {}", dir.display());

    for name in &names {
        let path = dir.join(format!("{name}.js"));
        let source = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{e}"));
        let atlas = burin_map_script::evaluate(&source)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
        let list = atlas.as_array().expect("an atlas is a list");
        assert!(!list.is_empty(), "{} draws nothing", path.display());
        atlases.insert(name.clone(), atlas);
    }

    let text = serde_json::to_string_pretty(&atlases).expect("an atlas serializes");
    let out = output_file();
    if write {
        std::fs::create_dir_all(out.parent().unwrap()).expect("the web presets dir exists");
        std::fs::write(&out, format!("{text}\n")).expect("writable");
        println!("wrote {} preset atlases to {}", names.len(), out.display());
        return;
    }
    let saved = std::fs::read_to_string(&out).unwrap_or_default();
    if saved.trim() != text.trim() {
        eprintln!(
            "{} is stale: run `cargo run -p burin-map-script --example check_presets -- --write`",
            out.display()
        );
        std::process::exit(1);
    }
    println!("{} preset atlases are up to date", names.len());
}
