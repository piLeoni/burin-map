//! Lines on water. The same tracer and dashes as on land, fed with fields made
//! from the distance to the shore instead of from the terrain: the input is 1
//! at the shore and fades to 0 at `water_reach`, so spacing and dashes open up
//! out to sea exactly as they do on lit slopes.

use rayon::prelude::*;
use serde::{Deserialize, Deserializer, Serialize};

use crate::fields::{wobble, Fields};
use crate::grid::{blur, gradient};
use crate::style::Style;

/// What a water batch's lines follow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Follow {
    /// The shore: `angle` 0 runs parallel to it, 90 straight out to sea.
    #[default]
    Shore,
    /// One direction over the whole map: `angle` in degrees clockwise from
    /// north, like an engraver's hatching.
    Map,
}

/// One drawing pass on water.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct WaterBatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub follow: Follow,
    pub angle: f32,
    /// Missing parameters take [`Style::water`]'s values, not the land's.
    #[serde(deserialize_with = "water_style")]
    pub style: Style,
}

impl Default for WaterBatch {
    fn default() -> Self {
        WaterBatch {
            name: None,
            follow: Follow::Shore,
            angle: 0.0,
            style: Style::water(),
        }
    }
}

impl Style {
    /// Defaults for water batches: tighter near the shore, opening and
    /// breaking up further out, and no wandering, which would read as waves.
    pub fn water() -> Style {
        Style {
            space: 4.0,
            space_light: 1.8,
            space_dark: 0.8,
            dash_light: 0.25,
            dash_drop: 0.6,
            mod_wobble: 0.0,
            mod_smooth: 3.0,
            ..Style::default()
        }
    }
}

fn water_style<'de, D: Deserializer<'de>>(d: D) -> Result<Style, D::Error> {
    let given = std::collections::BTreeMap::<String, f64>::deserialize(d)?;
    let mut st = Style::water();
    for (name, v) in given {
        // Unknown names are ignored, as they are in land styles.
        st.set(&name, v);
    }
    Ok(st)
}

/// Water on the drawing's grid, and what to draw on it.
pub struct WaterLayer<'a> {
    /// Fraction of each pixel that is water, 0…1, row 0 at the top.
    pub coverage: &'a [f32],
    /// Draw the shoreline itself.
    pub shore: bool,
    pub batches: &'a [WaterBatch],
}

/// Distance to the shore in px, positive on water, and the shoreline.
pub fn shore(coverage: &[f32], w: usize, h: usize) -> (Vec<f32>, Vec<crate::Line>) {
    let m = watermask::Mask {
        width: w,
        height: h,
        coverage: coverage.to_vec(),
    };
    (m.distance(), m.outlines())
}

/// Fields for one water batch, from the distance to the shore.
pub fn water_fields(coverage: &[f32], dist: &[f32], w: usize, h: usize, b: &WaterBatch) -> Fields {
    let st = &b.style;
    // No shore on the grid: the distance is infinite everywhere, and so the
    // input is 0; a finite stand-in keeps the blur and gradient clean.
    let far = (st.water_start + st.water_reach) * 2.0 + 1.0;
    let d: Vec<f32> = dist.par_iter().map(|&d| d.min(far)).collect();

    let (mut tx, mut ty) = match b.follow {
        Follow::Shore => {
            let (gy, gx) = gradient(&blur(&d, w, h, st.mod_smooth), w, h, 1.0);
            let (sa, ca) = b.angle.to_radians().sin_cos();
            gx.par_iter()
                .zip(&gy)
                .map(|(&gx, &gy)| {
                    let m = gx.hypot(gy);
                    if m < 1e-6 {
                        return (0.0, 0.0);
                    }
                    let (u, v) = (-gy / m, gx / m);
                    (u * ca - v * sa, u * sa + v * ca)
                })
                .unzip()
        }
        Follow::Map => {
            // Image y points south, so north is -y.
            let (sa, ca) = b.angle.to_radians().sin_cos();
            (vec![sa; w * h], vec![-ca; w * h])
        }
    };
    wobble(&mut tx, &mut ty, w, h, st, |_| 1.0);

    let reach = st.water_reach.max(1e-3);
    let input: Vec<f32> = d
        .par_iter()
        .zip(coverage)
        .map(|(&d, &c)| {
            if c < 0.5 || d < st.water_start {
                return 0.0;
            }
            (1.0 - (d - st.water_start) / reach)
                .clamp(0.0, 1.0)
                .powf(st.water_fade.max(1e-3))
        })
        .collect();
    let loosen: Vec<f32> = input
        .par_iter()
        .map(|&t| st.space_light + (st.space_dark - st.space_light) * t)
        .collect();
    let space = blur(&loosen, w, h, 3.0)
        .par_iter()
        .map(|k| st.space * k.clamp(0.05, 4.0))
        .collect();
    let walkable = (0..w * h)
        .into_par_iter()
        .map(|i| input[i] > 0.0 && (tx[i] != 0.0 || ty[i] != 0.0))
        .collect();
    Fields {
        w,
        h,
        tx,
        ty,
        input,
        walkable,
        space,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Sea on the left half, land on the right.
    fn half(w: usize, h: usize) -> Vec<f32> {
        (0..w * h)
            .map(|i| if i % w < w / 2 { 1.0 } else { 0.0 })
            .collect()
    }

    #[test]
    fn partial_style_starts_from_water_defaults() {
        let b: WaterBatch = serde_json::from_str(
            r#"{"follow": "map", "angle": 20, "style": {"wr": 90, "space": 5}}"#,
        )
        .unwrap();
        assert_eq!(b.follow, Follow::Map);
        assert_eq!(b.style.water_reach, 90.0);
        assert_eq!(b.style.space, 5.0);
        assert_eq!(b.style.space_light, Style::water().space_light);
        let bare: WaterBatch = serde_json::from_str("{}").unwrap();
        assert_eq!(bare, WaterBatch::default());
    }

    #[test]
    fn input_fades_out_to_sea_and_stays_off_land() {
        let (w, h) = (100, 20);
        let c = half(w, h);
        let (dist, outline) = shore(&c, w, h);
        assert!(!outline.is_empty());
        let b = WaterBatch {
            style: Style {
                water_start: 2.0,
                water_reach: 30.0,
                ..Style::water()
            },
            ..WaterBatch::default()
        };
        let f = water_fields(&c, &dist, w, h, &b);
        let row = 10 * w;
        assert_eq!(f.input[row + 70], 0.0, "land");
        assert!(
            f.input[row + 45] > f.input[row + 30],
            "fades away from the shore"
        );
        assert_eq!(f.input[row + 5], 0.0, "past the reach");
        // Along the shore: the shoreline is vertical, so lines run up and down.
        assert!(f.ty[row + 40].abs() > 0.99, "{}", f.ty[row + 40]);
    }

    #[test]
    fn map_follows_a_compass_angle() {
        let (w, h) = (40, 20);
        let c = half(w, h);
        let (dist, _) = shore(&c, w, h);
        let b = WaterBatch {
            follow: Follow::Map,
            angle: 90.0,
            ..WaterBatch::default()
        };
        let f = water_fields(&c, &dist, w, h, &b);
        assert!(
            (f.tx[5] - 1.0).abs() < 1e-6 && f.ty[5].abs() < 1e-6,
            "east-west"
        );
    }
}
