//! A pen groups dashes that share width, colour and layer.

/// Width, stroke colour and optional SVG layer for one run of dashes.
#[derive(Clone, Debug, PartialEq)]
pub struct Pen {
    pub stroke: f32,
    pub color: String,
    pub layer: Option<String>,
}

impl Default for Pen {
    fn default() -> Self {
        Pen {
            stroke: 1.2,
            color: "#000000".into(),
            layer: None,
        }
    }
}

impl Pen {
    pub fn new(stroke: f32, color: Option<&str>, layer: Option<&str>) -> Self {
        Pen {
            stroke,
            color: resolve_color(color, None).unwrap_or_else(|| "#000000".into()),
            layer: layer.filter(|s| !s.is_empty()).map(str::to_string),
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
        }
    }
}

fn resolve_color(color: Option<&str>, color_expr: Option<&str>) -> Option<String> {
    if let Some(c) = color.filter(|s| !s.is_empty()) {
        return Some(c.to_string());
    }
    color_expr
        .filter(|s| !s.is_empty())
        .and_then(crate::color_expr::parse_hex)
}
