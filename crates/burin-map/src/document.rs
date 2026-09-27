//! A JSON5 document (plain JSON works too) describing a whole drawing: where,
//! how big, and which batch passes. Used by `--style-json` / `--style-file`,
//! the Python and Node bindings, and the web app (including `#doc=…` URLs).
//!
//! Every field is optional so the same shape works for a bare style dump
//! (`{"batches": [...]}`) and for a fully pinned drawing. Command-line flags
//! override whatever the document says.

use serde::{Deserialize, Serialize};

use crate::{Batch, Frame, Style};

/// Where on the ground to draw and on what. The ground is a named place,
/// explicit bounds, or a centre with a size; the paper is optional. The same
/// choices the CLI accepts.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AreaDoc {
    /// Named area: `bern`, `matterhorn`, `san-francisco`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub place: Option<String>,
    /// Centre as an address / place name, or `"lat,lon"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    /// Explicit `[west, south, east, north]` in degrees.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounds: Option<[f64; 4]>,
    /// Ground size in km: `[side]` for a square, or `[width, height]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_km: Option<Vec<f64>>,
    /// Paper size in mm: `[side]` for a square, or `[width, height]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_mm: Option<Vec<f64>>,
    /// Map scale denominator (the `N` in `1:N`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
    /// Pen width on paper, mm.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pen_mm: Option<f64>,
}

/// `[side]` or `[width, height]` as a pair; `None` for any other length.
pub fn size_pair(v: &[f64]) -> Option<[f64; 2]> {
    match *v {
        [s] => Some([s, s]),
        [w, h] => Some([w, h]),
        _ => None,
    }
}

/// One or more drawing passes.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Document {
    /// Area and frame: `null`/absent means "take it from the command line".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub area: Option<AreaDoc>,
    /// The passes. An absent list means "one pass with defaults".
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub batches: Vec<Batch>,
}

impl Document {
    /// Parse a document written in JSON5 (plain JSON is valid JSON5, so both
    /// work): comments, trailing commas and unquoted keys are allowed.
    /// Accepts either the full object or a bare batch array (the shape older
    /// builds and `--style-json` already used).
    pub fn parse(s: &str) -> Result<Document, json5::Error> {
        let bare_list = strip_json5_comments(s).trim_start().starts_with('[');
        if bare_list {
            let batches: Vec<Batch> = json5::from_str(s)?;
            return Ok(Document { area: None, batches });
        }
        json5::from_str(s)
    }

    /// The batches, or one default pass when the document has none.
    pub fn batches_or_default(&self) -> Vec<Batch> {
        if self.batches.is_empty() {
            vec![Batch::along(Style::default())]
        } else {
            self.batches.clone()
        }
    }

    /// Copy the paper size and pen from the area into a `Frame`.
    pub fn apply_to_frame(&self, frame: &mut Frame) {
        let Some(area) = &self.area else { return };
        if let Some(size) = area.size_mm.as_deref().and_then(size_pair) {
            frame.size_mm = Some(size);
        }
        if let Some(pen) = area.pen_mm {
            frame.pen_width = pen;
        }
    }
}

/// The text after any leading `//` and `/* */` comments.
fn strip_json5_comments(s: &str) -> &str {
    let mut rest = s;
    loop {
        let t = rest.trim_start();
        if let Some(after) = t.strip_prefix("//") {
            rest = after.split_once('\n').map_or("", |(_, r)| r);
        } else if let Some(after) = t.strip_prefix("/*") {
            rest = after.split_once("*/").map_or("", |(_, r)| r);
        } else {
            return t;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_json5_with_comments() {
        let doc = Document::parse(r#"
            // Zermatt, A4
            {
              area: { at: "46.02,7.74", size_mm: [210, 297], },  // trailing commas are fine
              "batches": [
                /* one pass */
                { "angle": 0, "style": { "space": 6, /* "dash_light": 0.2, */ } },
              ],
            }"#).unwrap();
        assert_eq!(doc.area.unwrap().at.as_deref(), Some("46.02,7.74"));
        assert_eq!(doc.batches[0].style.space, 6.0);
        assert_eq!(doc.batches[0].style.dash_light, Style::default().dash_light);
        let list = Document::parse("// passes\n[{\"angle\": 90}]").unwrap();
        assert_eq!(list.batches[0].angle, 90.0);
    }

    #[test]
    fn parses_bare_batch_array() {
        let doc = Document::parse(r#"[{"angle": 90, "style": {"space": 6}}]"#).unwrap();
        assert!(doc.area.is_none());
        assert_eq!(doc.batches.len(), 1);
        assert_eq!(doc.batches[0].angle, 90.0);
    }

    #[test]
    fn parses_full_document() {
        let json = r#"{
            "area": {"at": "Twin Peaks, San Francisco", "size_km": [8, 8], "size_mm": [200], "pen_mm": 0.25},
            "batches": [
                {"angle": 0, "style": {"space": 5}},
                {"name": "Valleys", "angle": 90, "style": {"space": 6, "batch_floor": 0.4}}
            ]
        }"#;
        let doc = Document::parse(json).unwrap();
        let area = doc.area.as_ref().unwrap();
        assert_eq!(area.at.as_deref(), Some("Twin Peaks, San Francisco"));
        assert_eq!(area.size_km.as_deref(), Some(&[8.0, 8.0][..]));
        assert_eq!(doc.batches.len(), 2);
        assert_eq!(doc.batches[1].style.batch_floor, 0.4);
        assert_eq!(doc.batches[0].name, None);
        assert_eq!(doc.batches[1].name.as_deref(), Some("Valleys"));

        let mut frame = Frame::default();
        doc.apply_to_frame(&mut frame);
        assert_eq!(frame.size_mm, Some([200.0, 200.0]));
        assert_eq!(frame.pen_width, 0.25);
    }

    #[test]
    fn old_frame_keys_are_rejected() {
        for key in ["px", "width_mm", "height_mm"] {
            let json = format!(r#"{{"area": {{"{key}": 100}}}}"#);
            let err = Document::parse(&json).unwrap_err().to_string();
            assert!(err.contains(key), "{err}");
        }
    }

    #[test]
    fn absent_batches_gives_one_pass() {
        let doc = Document::parse(r#"{"area": {"place": "bern"}}"#).unwrap();
        assert_eq!(doc.batches_or_default().len(), 1);
    }

    #[test]
    fn round_trips() {
        let doc = Document {
            area: Some(AreaDoc { size_mm: Some(vec![210.0, 297.0]), scale: Some(25000.0), ..Default::default() }),
            batches: vec![Batch::along(Style::default())],
        };
        let json = serde_json::to_string(&doc).unwrap();
        let back = Document::parse(&json).unwrap();
        let area = back.area.unwrap();
        assert_eq!(area.size_mm.as_deref(), Some(&[210.0, 297.0][..]));
        assert_eq!(area.scale, Some(25000.0));
        assert_eq!(back.batches.len(), 1);
    }
}
