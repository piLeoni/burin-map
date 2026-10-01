//! A pen groups dashes that share width, colour, line ends and layer.

use serde::{Deserialize, Serialize};

/// How the ends of a dash look.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Cap {
    /// Rounded, reaching half the width past each end, as a pen draws.
    #[default]
    Round,
    /// Rounded, but within the ends: each dash is shortened by half its width.
    RoundInside,
    /// Square, reaching half the width past each end.
    Square,
    /// Flat, cut exactly at the ends.
    Butt,
}

impl Cap {
    pub fn is_round(&self) -> bool {
        *self == Cap::Round
    }

    /// The SVG `stroke-linecap` that draws it; `round_inside` is a round cap
    /// on shortened dashes.
    pub fn svg(&self) -> &'static str {
        match self {
            Cap::Round | Cap::RoundInside => "round",
            Cap::Square => "square",
            Cap::Butt => "butt",
        }
    }
}

/// Width, stroke colour, line ends and optional SVG layer for one run of dashes.
#[derive(Clone, Debug, PartialEq)]
pub struct Pen {
    pub stroke: f32,
    pub color: String,
    pub layer: Option<String>,
    pub cap: Cap,
    /// Fills its rings instead of stroking them.
    pub fill: bool,
}

impl Default for Pen {
    fn default() -> Self {
        Pen {
            stroke: 1.2,
            color: "#000000".into(),
            layer: None,
            cap: Cap::Round,
            fill: false,
        }
    }
}

impl Pen {
    pub fn new(stroke: f32, color: Option<&str>, layer: Option<&str>) -> Self {
        Pen {
            stroke,
            color: resolve_color(color, None).unwrap_or_else(|| "#000000".into()),
            layer: layer.filter(|s| !s.is_empty()).map(str::to_string),
            cap: Cap::Round,
            fill: false,
        }
    }

    /// `color` wins; otherwise a `color_expr` that is only a hex literal works.
    pub fn with_expr(
        stroke: f32,
        color: Option<&str>,
        color_expr: Option<&str>,
        layer: Option<&str>,
    ) -> Self {
        Pen {
            stroke,
            color: resolve_color(color, color_expr).unwrap_or_else(|| "#000000".into()),
            layer: layer.filter(|s| !s.is_empty()).map(str::to_string),
            cap: Cap::Round,
            fill: false,
        }
    }

    /// A fill in `color` (or a hex `color_expr`), light grey without either.
    pub fn filling(color: Option<&str>, color_expr: Option<&str>, layer: Option<&str>) -> Self {
        Pen {
            stroke: 0.0,
            color: resolve_color(color, color_expr).unwrap_or_else(|| "#dddddd".into()),
            layer: layer.filter(|s| !s.is_empty()).map(str::to_string),
            cap: Cap::Round,
            fill: true,
        }
    }
}

/// How one batch inks its dashes: the settings beside its style that pick
/// each dash's pen.
#[derive(Clone, Copy, Debug, Default)]
pub struct Ink<'a> {
    /// `batch_stroke`, px: the width `width_expr` multiplies.
    pub stroke: f32,
    pub layer: Option<&'a str>,
    pub color: Option<&'a str>,
    pub color_expr: Option<&'a str>,
    pub width_expr: Option<&'a str>,
    /// 0 for any width; 2 or more rounds widths to that many pens.
    pub width_steps: u32,
    pub cap: Cap,
}

fn resolve_color(color: Option<&str>, color_expr: Option<&str>) -> Option<String> {
    if let Some(c) = color.filter(|s| !s.is_empty()) {
        return Some(c.to_string());
    }
    color_expr
        .filter(|s| !s.is_empty())
        .and_then(crate::color_expr::parse_hex)
}
