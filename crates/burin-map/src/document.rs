//! A JSON document describing a whole drawing: where, how big, and which
//! batch passes. Used by `--style-json` / `--style-file`, the Python and Node
//! bindings, and the web app (including `#doc=…` URLs).
//!
//! Every field is optional so the same shape works for a bare style dump
//! (`{"batches": [...]}`) and for a fully pinned drawing. Command-line flags
//! override whatever the document says.

use serde::{Deserialize, Serialize};

use crate::{Batch, Frame, Style};

/// Where on the ground to draw. Give either a named place, a centre with a
/// size, or explicit bounds — the same three ways the CLI accepts.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct AreaDoc {
    /// Named area: `bern`, `matterhorn`, `san-francisco`.
    pub place: Option<String>,
    /// Centre as an address / place name, or `"lat,lon"`.
    pub at: Option<String>,
    /// Explicit `[west, south, east, north]` in degrees.
    pub bounds: Option<[f64; 4]>,
    /// Ground size in km: `[width]`, or `[width, height]`.
    pub size_km: Option<Vec<f64>>,
    /// Frame grid width in px when no physical width is given.
    pub px: Option<usize>,
    /// Physical width / max height in mm, and pen width in mm.
    pub width_mm: Option<f64>,
    pub height_mm: Option<f64>,
    pub pen_mm: Option<f64>,
    /// Map scale denominator (the `N` in `1:N`).
    pub scale: Option<f64>,
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
    /// Parse a document. Accepts either the full object or a bare batch array
    /// (the shape older builds and `--style-json` already used).
    pub fn parse(s: &str) -> Result<Document, serde_json::Error> {
        let trimmed = s.trim_start();
        if trimmed.starts_with('[') {
            let batches: Vec<Batch> = serde_json::from_str(s)?;
            return Ok(Document { area: None, batches });
        }
        serde_json::from_str(s)
    }

    /// The batches, or one default pass when the document has none.
    pub fn batches_or_default(&self) -> Vec<Batch> {
        if self.batches.is_empty() {
            vec![Batch::along(Style::default())]
        } else {
            self.batches.clone()
        }
    }

    /// Merge the area into a `Frame`, letting the document win only where the
    /// command line left a value unset.
    pub fn apply_to_frame(&self, frame: &mut Frame) {
        let Some(area) = &self.area else { return };
        if let Some(px) = area.px {
            *frame = Frame { px, ..frame.clone() };
        }
        if area.width_mm.is_some() {
            frame.width_mm = area.width_mm;
        }
        if area.height_mm.is_some() {
            frame.height_mm = area.height_mm;
        }
        if let Some(pen) = area.pen_mm {
            frame.pen_mm = pen;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            "area": {"at": "Twin Peaks, San Francisco", "size_km": [8, 8], "px": 1200, "pen_mm": 0.25},
            "batches": [
                {"angle": 0, "style": {"space": 5}},
                {"angle": 90, "style": {"space": 6, "batch_floor": 0.4}}
            ]
        }"#;
        let doc = Document::parse(json).unwrap();
        let area = doc.area.as_ref().unwrap();
        assert_eq!(area.at.as_deref(), Some("Twin Peaks, San Francisco"));
        assert_eq!(area.size_km.as_deref(), Some(&[8.0, 8.0][..]));
        assert_eq!(doc.batches.len(), 2);
        assert_eq!(doc.batches[1].style.batch_floor, 0.4);

        let mut frame = Frame::default();
        doc.apply_to_frame(&mut frame);
        assert_eq!(frame.px, 1200);
        assert_eq!(frame.pen_mm, 0.25);
    }

    #[test]
    fn absent_batches_gives_one_pass() {
        let doc = Document::parse(r#"{"area": {"place": "bern"}}"#).unwrap();
        assert_eq!(doc.batches_or_default().len(), 1);
    }

    #[test]
    fn round_trips() {
        let doc = Document {
            area: Some(AreaDoc { px: Some(800), ..Default::default() }),
            batches: vec![Batch::along(Style::default())],
        };
        let json = serde_json::to_string(&doc).unwrap();
        let back = Document::parse(&json).unwrap();
        assert_eq!(back.area.unwrap().px, Some(800));
        assert_eq!(back.batches.len(), 1);
    }
}
