//! The `burin` object a script sees, built on the typed API.
//!
//! No interpreter here: this is the shape the JavaScript front end calls into,
//! one method at a time, and it returns plain JavaScript values so a script
//! can hold them and pass them on. A tree is written as JSON — the same form
//! `expr` saves — so `["lerp", "#e8dcc8", "#2a1810", "tone"]` and
//! `burin.lerp('#e8dcc8', '#2a1810', burin.tone)` are the same thing.

use crate::atlas::{Atlas, WaterDraw};
use crate::expr::{Expr, Scalar};
use crate::water::{Shoreline, WaterBatch};
use crate::{Batch, Cap, Drawing, Follow, Ground, Style};
use serde_json::{json, Map, Value};

/// A batch, or something that is not one yet.
pub type BatchValue = Value;

/// Read a batch from a script's value: an options object, or `null` for a
/// plain run of lines.
pub fn batch_from(v: &Value) -> Result<Batch, String> {
    let o = v.as_object();
    let get = |k: &str| o.and_then(|m| m.get(k));
    let draw = get("draw").and_then(Value::as_str).unwrap_or("lines");
    let mut b = match draw {
        "lines" => Batch::lines(),
        "dots" => Batch::dots(),
        "fill" => Batch::filling(
            get("color").and_then(Value::as_str),
            color_of(get("shade"))?,
            get("name").and_then(Value::as_str),
        ),
        other => return Err(format!("draw {other:?} is not lines, dots or fill")),
    };
    if let Some(name) = get("name").and_then(Value::as_str) {
        b.name = Some(name.to_string());
    }
    if let Some(layer) = get("layer").and_then(Value::as_str) {
        b.layer = Some(layer.to_string());
    }
    if let Some(c) = get("color").and_then(Value::as_str) {
        b.color = Some(c.into());
    }
    if let Some(t) = get("shade") {
        if let Some(e) = color_of(Some(t))? {
            b.color_expr = Some(e);
        }
    }
    if let Some(w) = get("width") {
        b.width_expr = Some(scalar_of(w)?);
    }
    if let Some(n) = get("widthSteps").and_then(Value::as_u64) {
        b.width_steps = n as u32;
    }
    if let Some(a) = get("angle").and_then(Value::as_f64) {
        b.angle = a as f32;
    }
    if let Some(g) = get("ground").and_then(Value::as_str) {
        b.ground = match g {
            "land" => Ground::Land,
            "sea" => Ground::Sea,
            other => return Err(format!("ground {other:?} is land or sea")),
        };
    }
    if let Some(c) = get("cap").and_then(Value::as_str) {
        b.cap = cap_of(c)?;
    }
    apply_style(&mut b.style, get("style"))?;
    Ok(b)
}

/// Read a water pass: `burin.water.lines({ ... })`.
pub fn water_from(v: &Value, kind: Option<&str>) -> Result<WaterBatch, String> {
    let draw = kind.unwrap_or("lines");
    let mut w = WaterBatch {
        draw: match draw {
            "lines" => WaterDraw::Lines,
            "ripples" => WaterDraw::Ripples,
            "dots" => WaterDraw::Dots,
            other => return Err(format!("water draws lines, ripples or dots, not {other:?}")),
        },
        ..WaterBatch::default()
    };
    w.style = Style::drawn(w.draw);
    let o = v.as_object();
    let get = |k: &str| o.and_then(|m| m.get(k));
    if let Some(name) = get("name").and_then(Value::as_str) {
        w.name = Some(name.into());
    }
    if let Some(c) = get("color").and_then(Value::as_str) {
        w.color = Some(c.into());
    }
    if let Some(t) = get("shade") {
        w.color_expr = color_of(Some(t))?;
    }
    if let Some(x) = get("width") {
        w.width_expr = Some(scalar_of(x)?);
    }
    if let Some(c) = get("cap").and_then(Value::as_str) {
        w.cap = cap_of(c)?;
    }
    if let Some(f) = get("follow").and_then(Value::as_str) {
        w.follow = match f {
            "shore" => Follow::Shore,
            "map" | "angle" => Follow::Map,
            other => return Err(format!("follow is shore or angle, not {other:?}")),
        };
    }
    if let Some(a) = get("angle").and_then(Value::as_f64) {
        w.angle = a as f32;
    }
    if let Some(where_) = get("where") {
        let g = |k: &str| where_.get(k).and_then(Value::as_f64).map(|v| v as f32);
        w.r#where.depth_min = g("depthMin");
        w.r#where.depth_max = g("depthMax");
    }
    apply_style(&mut w.style, get("style"))?;
    Ok(w)
}

pub fn shore_from(v: &Value) -> Result<Shoreline, String> {
    let mut s = Shoreline::default();
    let o = v.as_object();
    let get = |k: &str| o.and_then(|m| m.get(k));
    if let Some(n) = get("lines").and_then(Value::as_u64) {
        s.lines = n as u32;
    }
    if let Some(c) = get("color").and_then(Value::as_str) {
        s.color = Some(c.into());
    }
    if let Some(t) = get("shade") {
        s.color_expr = color_of(Some(t))?;
    }
    if let Some(x) = get("width") {
        s.width_expr = Some(scalar_of(x)?);
    }
    if let Some(c) = get("cap").and_then(Value::as_str) {
        s.cap = cap_of(c)?;
    }
    apply_style(&mut s.style, get("style"))?;
    Ok(s)
}

fn cap_of(c: &str) -> Result<Cap, String> {
    Ok(match c {
        "round" => Cap::Round,
        "round-inside" | "roundInside" => Cap::RoundInside,
        "butt" | "flat" => Cap::Butt,
        other => return Err(format!("cap {other:?} is round, round-inside or butt")),
    })
}

fn apply_style(style: &mut Style, v: Option<&Value>) -> Result<(), String> {
    let Some(Value::Object(m)) = v else {
        return Ok(());
    };
    for (name, value) in m {
        let Some(x) = value.as_f64() else { continue };
        // Unknown names are ignored, as in a document's style block.
        style.set(name, x);
    }
    Ok(())
}

/// A colour tree from a script's value: a hex string, a grey, an rgb triple,
/// or `["lerp", …]`. `null` when there is none.
pub fn color_of(v: Option<&Value>) -> Result<Option<Expr>, String> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(v) => serde_json::from_value::<Expr>(v.clone())
            .map(Some)
            .map_err(|e| e.to_string()),
    }
}

/// A width tree from a script's value.
pub fn scalar_of(v: &Value) -> Result<Scalar, String> {
    serde_json::from_value::<Scalar>(v.clone()).map_err(|e| e.to_string())
}

/// Read an [`Atlas`] from a script's list of batches and water passes, in the
/// order they were written. A water pass or the shore carries `"kind"`; the
/// rest of its keys are its own, the same shape it saves as.
pub fn atlas_from(v: &Value) -> Result<Atlas, String> {
    let mut atlas = Atlas::new();
    let Some(items) = v.as_array() else {
        return Err("an atlas is a list of batches and water passes".into());
    };
    for item in items {
        match item.get("kind").and_then(Value::as_str) {
            Some("water") => {
                let kind = item
                    .get("draw")
                    .and_then(Value::as_str)
                    .unwrap_or("lines")
                    .to_string();
                let w = water_from(item, Some(&kind))?;
                atlas.water(w);
            }
            Some("shore") => {
                let s = shore_from(item)?;
                atlas.shore(s);
            }
            _ => {
                atlas.batch(batch_from(item)?);
            }
        }
    }
    Ok(atlas)
}

/// What an `Atlas` serializes to, so a script's drawing can be saved and
/// rebuilt: the same JSON a document carries.
pub fn atlas_to_json(atlas: &Atlas) -> Value {
    let mut out: Vec<Value> = atlas
        .batches
        .iter()
        .map(|b| serde_json::to_value(b).unwrap_or(Value::Null))
        .collect();
    if let Some(w) = &atlas.water {
        for pass in &w.batches {
            let mut o = serde_json::to_value(pass).unwrap_or(Value::Null);
            if let Some(m) = o.as_object_mut() {
                m.insert("kind".into(), json!("water"));
            }
            out.push(o);
        }
        if let Some(s) = &w.shoreline {
            let mut o = serde_json::to_value(s).unwrap_or(Value::Null);
            if let Some(m) = o.as_object_mut() {
                m.insert("kind".into(), json!("shore"));
            }
            out.push(o);
        }
    }
    Value::Array(out)
}

/// The summary of a drawing a script gets back: dashes as `[x, y]` pairs,
/// the runs of pen, and the SVG.
pub fn drawing_to_json(d: &Drawing) -> Value {
    let mut runs: Vec<Value> = Vec::with_capacity(d.runs.len());
    let mut start = 0;
    for (pen, end) in &d.runs {
        let n = end.saturating_sub(start);
        runs.push(json!({
            "stroke": pen.stroke,
            "color": pen.color,
            "layer": pen.layer,
            "dashes": n,
        }));
        start = *end;
    }
    let paths: Vec<Value> = d
        .paths
        .iter()
        .map(|l| json!(l.iter().map(|p| json!([p[0], p[1]])).collect::<Vec<_>>()))
        .collect();
    json!({
        "width": d.width,
        "height": d.height,
        "paths": paths,
        "lines": d.lines.len(),
        "runs": runs,
        "fills": d.fills.len(),
        "svg": d.to_svg(None),
        "layers": d.layers().count(),
    })
}

/// The names of the style parameters, for editors and for a script asking.
pub fn parameters() -> Value {
    Value::Array(
        Style::PARAMS
            .iter()
            .map(|p| json!({"name": p.name, "short": p.short, "help": p.help, "group": p.group}))
            .collect(),
    )
}

/// One entry of an atlas, so a script can build it up before drawing.
pub fn water_entry(kind: &str, options: &Value) -> Value {
    json!({"kind": "water", "draw": kind, "options": options})
}

pub fn shore_entry(options: &Value) -> Value {
    json!({"kind": "shore", "options": options})
}

/// The `Map` a `burin.batch()` call builds, kept in one place.
pub fn empty_options() -> Value {
    Value::Object(Map::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{from_json_batches, to_json_batches};

    fn island(w: usize, h: usize) -> Vec<f32> {
        let mut e = vec![0.0f32; w * h];
        for y in 0..h {
            for x in 0..w {
                let b = |cx: f32, cy: f32, r: f32, hgt: f32| {
                    hgt * (-((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)) / (2.0 * r * r))
                        .exp()
                };
                e[y * w + x] =
                    (b(150.0, 150.0, 60.0, 1500.0) + b(270.0, 170.0, 40.0, 900.0) - 150.0).max(0.0);
            }
        }
        e
    }

    #[test]
    fn a_script_atlas_matches_the_document() {
        // What `burin.atlas()` builds from a script, as JSON…
        let script = serde_json::json!([
            {"draw": "lines", "name": "hills",
             "shade": ["lerp", "#e8dcc8", "#2a1810", "tone"]},
            {"draw": "lines", "name": "coast", "angle": 30, "style": {"space": 6.0},
             "width": ["lerp", 0.5, 2, "tone"]},
            {"draw": "fill", "name": "sea", "ground": "sea", "color": "#bcd6e2"}
        ]);
        let atlas = atlas_from(&script).unwrap();
        // …and the same thing written as a document.
        let json = to_json_batches(&atlas.batches);
        let doc = from_json_batches(&json).unwrap();
        assert_eq!(
            to_json_batches(&doc),
            json,
            "a script's batches save as the document's"
        );
    }

    #[test]
    fn trees_read_the_fields_they_name() {
        let b = batch_from(&serde_json::json!({
            "shade": ["lerp", "#fff", "#000", "tone"],
            "width": ["clamp", ["*", "dist", 0.01], 0, 2]
        }))
        .unwrap();
        let e = b.color_expr.unwrap();
        assert!(e.uses(crate::expr::Field::Tone));
        assert!(!e.uses(crate::expr::Field::Depth));
        let w = b.width_expr.unwrap();
        assert!(w.constant_scalar() == false);
        assert!(crate::expr::Expr::Scalar(w).uses(crate::expr::Field::Dist));
    }

    #[test]
    fn the_atlas_round_trips_through_its_json() {
        let script = serde_json::json!([
            {"draw": "dots", "name": "stipple"},
            {"kind": "water", "draw": "ripples", "color": "#33556b"},
            {"kind": "shore", "lines": 3}
        ]);
        let atlas = atlas_from(&script).unwrap();
        let json = atlas_to_json(&atlas);
        let back = atlas_from(&json).unwrap();
        assert_eq!(atlas_to_json(&back), json, "the saved form is a fixed point");
    }

    #[test]
    fn a_script_draws_its_island() {
        let (w, h) = (320, 240);
        let e = island(w, h);
        let script = serde_json::json!([
            {"draw": "fill", "name": "sea", "ground": "sea", "color": "#cfe3ec"},
            {"draw": "lines", "name": "hills", "shade": ["lerp", "#e8dcc8", "#2a1810", "tone"]}
        ]);
        let atlas = atlas_from(&script).unwrap();
        let d = atlas.draw(&e, w, h, 200.0);
        assert!(!d.paths.is_empty());
        let out = drawing_to_json(&d);
        assert!(out["svg"].as_str().unwrap().contains("<svg"));
        assert!(out["runs"].as_array().unwrap().len() > 1, "the shading changes the pen");
    }

    #[test]
    fn bad_values_say_what_is_wrong() {
        assert!(batch_from(&serde_json::json!({"draw": "curls"})).is_err());
        assert!(batch_from(&serde_json::json!({"shade": ["nope", 1]})).is_err());
        assert!(water_from(&serde_json::json!({}), Some("waves")).is_err());
        assert!(batch_from(&serde_json::json!({"ground": "space"})).is_err());
    }
}
