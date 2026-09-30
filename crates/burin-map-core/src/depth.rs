//! Bathymetry: depth below the surface (metres, ≥ 0 on water). Built from the
//! elevation grid when `terrarium` / `auto` is enough; optional override when
//! a separate fetch was requested.

use rayon::prelude::*;
use serde::{Deserialize, Serialize};

use crate::water::{Shoreline, WaterBatch};

/// Where bathymetry comes from. `terrarium` / `auto` reuse the elevation
/// mosaic (no extra fetch). `gebco` or a tile `url` need a separate download.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct BathymetryDoc {
    /// `terrarium`, `auto`, `gebco`, `url`, or a `{z}/{x}/{y}` template.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Custom tile URL when `source` is `url`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

/// Depth limits for a water batch; absent fields mean no limit on that side.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WaterWhere {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth_min: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth_max: Option<f32>,
}

impl WaterWhere {
    pub fn is_active(&self) -> bool {
        self.depth_min.is_some() || self.depth_max.is_some()
    }
}

/// For `skip_serializing_if` on batch `where`.
pub fn where_inactive(w: &WaterWhere) -> bool {
    !w.is_active()
}

/// Whether a batch draws at this depth (metres below the surface).
#[must_use]
pub fn depth_allows(depth_m: f32, w: &WaterWhere) -> bool {
    if let Some(m) = w.depth_min {
        if depth_m < m {
            return false;
        }
    }
    if let Some(m) = w.depth_max {
        if depth_m > m {
            return false;
        }
    }
    true
}

/// Metres below the surface on water pixels; 0 on land. Uses `override_` when
/// the caller already fetched a dedicated bathymetry grid.
pub fn build_depth_grid(
    elev: &[f32],
    coverage: &[f32],
    override_: Option<&[f32]>,
) -> Vec<f32> {
    if let Some(o) = override_ {
        assert_eq!(o.len(), elev.len(), "depth grid size mismatch");
        return o.to_vec();
    }
    elev.par_iter()
        .zip(coverage)
        .map(|(&e, &c)| {
            if c < 0.5 {
                0.0
            } else {
                (-e).max(0.0)
            }
        })
        .collect()
}

/// True when any water pass or shoreline colouring needs a depth grid.
#[must_use]
pub fn needs_depth(batches: &[WaterBatch], shoreline: Option<&Shoreline>) -> bool {
    batches.iter().any(batch_needs_depth)
        || shoreline.is_some_and(|s| expr_uses_depth(s.color_expr.as_deref()))
}

fn batch_needs_depth(b: &WaterBatch) -> bool {
    b.r#where.is_active() || expr_uses_depth(b.color_expr.as_deref())
}

/// True when `color_expr` references the `depth` scalar.
#[must_use]
pub fn expr_uses_depth(expr: Option<&str>) -> bool {
    let Some(s) = expr.filter(|t| !t.is_empty()) else {
        return false;
    };
    s.split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .any(|w| w == "depth")
}

/// Separate network fetch (GEBCO, custom tiles). `terrarium` / `auto` reuse elevation.
#[must_use]
pub fn bathymetry_source_needs_fetch(source: Option<&str>) -> bool {
    match source.map(str::trim).filter(|s| !s.is_empty()) {
        None => false,
        Some(s) => {
            let low = s.to_ascii_lowercase();
            matches!(low.as_str(), "gebco" | "url")
                || low.contains("{z}")
                || low.starts_with("http://")
                || low.starts_with("https://")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn depth_from_negative_elev_on_water() {
        let elev = vec![100.0, -50.0, -200.0, 0.0];
        let cov = vec![1.0, 1.0, 1.0, 0.0];
        let d = build_depth_grid(&elev, &cov, None);
        assert_eq!(d, vec![0.0, 50.0, 200.0, 0.0]);
    }

    #[test]
    fn where_max() {
        let w = WaterWhere {
            depth_max: Some(100.0),
            ..Default::default()
        };
        assert!(depth_allows(50.0, &w));
        assert!(!depth_allows(150.0, &w));
    }
}
