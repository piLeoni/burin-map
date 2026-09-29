//! Lines on water. The same tracer and dashes as on land, fed with fields made
//! from the distance to the shore instead of from the terrain: the input is 1
//! at the shore and fades to 0 at `water_reach`, so spacing and dashes open up
//! out to sea exactly as they do on lit slopes.

use rayon::prelude::*;
use serde::{Deserialize, Deserializer, Serialize};

use crate::fields::{wobble, Fields};
use crate::grid::{blur, gradient};
use crate::rng::Rng;
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

fn style_over<'de, D: Deserializer<'de>>(d: D, mut st: Style) -> Result<Style, D::Error> {
    let given = std::collections::BTreeMap::<String, f64>::deserialize(d)?;
    for (name, v) in given {
        // Unknown names are ignored, as they are in land styles.
        st.set(&name, v);
    }
    Ok(st)
}

fn water_style<'de, D: Deserializer<'de>>(d: D) -> Result<Style, D::Error> {
    style_over(d, Style::water())
}

fn shore_style<'de, D: Deserializer<'de>>(d: D) -> Result<Style, D::Error> {
    style_over(d, Style::shoreline())
}

/// The shoreline, drawn with the land's modifiers: `dash_*` break it up,
/// `mod_wobble` makes it wander as if drawn by hand, `mod_smooth` rounds off
/// the pixel steps, `batch_stroke` is its width. With `lines` above 1 more
/// lines follow it out to sea, `space` px apart, their input fading over
/// `water_reach` as a water batch's does.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Shoreline {
    pub lines: u32,
    /// Missing parameters take [`Style::shoreline`]'s values.
    #[serde(deserialize_with = "shore_style")]
    pub style: Style,
}

impl Default for Shoreline {
    fn default() -> Self {
        Shoreline {
            lines: 1,
            style: Style::shoreline(),
        }
    }
}

impl Style {
    /// Defaults for the shoreline: one clean, solid line.
    pub fn shoreline() -> Style {
        Style {
            space: 3.0,
            dash_light: 1.0,
            dash_dark: 1.0,
            dash_drop: 0.0,
            mod_breaks: 0.0,
            mod_wobble: 0.0,
            mod_smooth: 0.0,
            ..Style::default()
        }
    }
}

/// `shoreline` in a document: `true` (the default) or `false`, or the
/// [`Shoreline`] itself. For `#[serde(with = "…")]`.
pub mod shoreline_option {
    use super::Shoreline;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn default() -> Option<Shoreline> {
        Some(Shoreline::default())
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Option<Shoreline>, D::Error> {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Given {
            Flag(bool),
            Lines(Shoreline),
        }
        Ok(match Given::deserialize(d)? {
            Given::Flag(false) => None,
            Given::Flag(true) => default(),
            Given::Lines(s) => Some(s),
        })
    }

    pub fn serialize<S: Serializer>(v: &Option<Shoreline>, s: S) -> Result<S::Ok, S::Error> {
        match v {
            None => s.serialize_bool(false),
            Some(sh) if *sh == Shoreline::default() => s.serialize_bool(true),
            Some(sh) => sh.serialize(s),
        }
    }
}

/// Water on the drawing's grid, and what to draw on it.
pub struct WaterLayer<'a> {
    /// Fraction of each pixel that is water, 0…1, row 0 at the top.
    pub coverage: &'a [f32],
    /// The shoreline, if it is drawn.
    pub shoreline: Option<&'a Shoreline>,
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

/// The shoreline's lines, before and after dashing. `outlines` is the shore
/// from [`shore`]; further lines are the distance's contours out to sea.
pub fn shoreline(
    outlines: &[crate::Line],
    dist: &[f32],
    w: usize,
    h: usize,
    sh: &Shoreline,
) -> (Vec<crate::Line>, Vec<crate::Line>) {
    let st = &sh.style;
    let rings: Vec<(Vec<crate::Line>, f32)> = (0..sh.lines.max(1))
        .into_par_iter()
        .map(|k| {
            let out = k as f32 * st.space.max(0.5);
            let lines = if k == 0 {
                outlines.to_vec()
            } else {
                // Coverage that crosses one half `out` px from the shore. Off
                // whole pixels: a contour through pixel centres falls apart.
                let level = out + 0.013;
                let coverage = dist
                    .iter()
                    .map(|&d| (0.5 + d - level).clamp(0.0, 1.0))
                    .collect();
                watermask::Mask {
                    width: w,
                    height: h,
                    coverage,
                }
                .outlines()
            };
            let tone = (1.0 - out / st.water_reach.max(1e-3))
                .clamp(0.0, 1.0)
                .powf(st.water_fade.max(1e-3));
            let mut rng = Rng::new(st.batch_seed.wrapping_add(0x5402 + k as u64));
            let lines = lines
                .iter()
                .filter(|l| l.len() >= 2)
                .map(|l| {
                    let mut l = smooth(l, st.mod_smooth);
                    wander(&mut l, st, &mut rng);
                    l
                })
                .collect();
            (lines, tone)
        })
        .collect();
    let mut lines = Vec::new();
    let mut paths = Vec::new();
    for (k, (ring, tone)) in rings.into_iter().enumerate() {
        let mut st = st.clone();
        st.batch_seed = st.batch_seed.wrapping_add(k as u64 * 0x9E37_79B9);
        paths.extend(crate::dashes::dash_lines_by(&ring, &st, |_| tone));
        lines.extend(ring);
    }
    (lines, paths)
}

fn closed(l: &crate::Line) -> bool {
    l.len() > 2 && l.first() == l.last()
}

/// Rounded off over about `r` px: resampled every px, then averaged twice
/// over a window that wide. Open lines keep their ends.
fn smooth(l: &crate::Line, r: f32) -> crate::Line {
    let k = r.round() as usize;
    if k == 0 || l.len() < 3 {
        return l.clone();
    }
    let ring = closed(l);
    let mut p = resample(l, 1.0);
    if ring {
        p.pop();
    }
    let n = p.len();
    if n < 3 {
        return l.clone();
    }
    for _ in 0..2 {
        let at = |i: isize| -> [f32; 2] {
            if ring {
                p[i.rem_euclid(n as isize) as usize]
            } else {
                p[i.clamp(0, n as isize - 1) as usize]
            }
        };
        let q: Vec<[f32; 2]> = (0..n as isize)
            .map(|i| {
                let mut s = [0.0f32; 2];
                for j in -(k as isize)..=k as isize {
                    let a = at(i + j);
                    s[0] += a[0];
                    s[1] += a[1];
                }
                let m = (2 * k + 1) as f32;
                [s[0] / m, s[1] / m]
            })
            .collect();
        p = q;
    }
    if ring {
        p.push(p[0]);
    } else {
        p[0] = l[0];
        p[n - 1] = *l.last().unwrap();
    }
    p
}

/// Points every `step` px along `l`, its ends included.
fn resample(l: &crate::Line, step: f32) -> crate::Line {
    let mut out = vec![l[0]];
    let mut left = step;
    for s in l.windows(2) {
        let (a, b) = (s[0], s[1]);
        let len = (b[0] - a[0]).hypot(b[1] - a[1]);
        let mut at = 0.0;
        while len - at >= left {
            at += left;
            let t = at / len;
            out.push([a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]);
            left = step;
        }
        left -= len - at;
    }
    if out.last() != l.last() {
        out.push(*l.last().unwrap());
    }
    out
}

/// Pushes the line sideways by smooth noise, one bump every
/// `mod_wobble_size` px, steep enough to turn it by about `mod_wobble`
/// degrees: the land's wander, on a line that is already drawn.
fn wander(l: &mut crate::Line, st: &Style, rng: &mut Rng) {
    if st.mod_wobble <= 0.0 || l.len() < 2 {
        return;
    }
    let size = st.mod_wobble_size.max(1.0);
    let amp = size * st.mod_wobble.min(80.0).to_radians().tan() / 3.0;
    let mut s = vec![0.0f32];
    for p in l.windows(2) {
        s.push(s.last().unwrap() + (p[1][0] - p[0][0]).hypot(p[1][1] - p[0][1]));
    }
    let total = *s.last().unwrap();
    let ring = closed(l);
    // A ring's bumps wrap around, so its seam doesn't show.
    let knots = ((total / size).ceil() as usize).max(1);
    let gap = if ring { total / knots as f32 } else { size };
    let v: Vec<f32> = (0..knots + 1)
        .map(|_| rng.uniform(-1.0, 1.0) as f32)
        .collect();
    let noise = |s: f32| {
        let x = s / gap.max(1e-3);
        let j = x.floor() as usize;
        let f = x - j as f32;
        let f = f * f * (3.0 - 2.0 * f);
        let pick = |j: usize| if ring { v[j % knots] } else { v[j.min(knots)] };
        pick(j) + (pick(j + 1) - pick(j)) * f
    };
    let n = l.len();
    let orig = l.clone();
    for i in 0..n {
        let (a, b) = if ring && (i == 0 || i == n - 1) {
            (orig[n - 2], orig[1])
        } else {
            (orig[i.saturating_sub(1)], orig[(i + 1).min(n - 1)])
        };
        let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
        let m = dx.hypot(dy);
        if m < 1e-6 {
            continue;
        }
        let d = amp * noise(s[i]);
        l[i] = [orig[i][0] - dy / m * d, orig[i][1] + dx / m * d];
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

    /// A round lake in the middle of the grid.
    fn lake(w: usize, h: usize, r: f32) -> Vec<f32> {
        (0..w * h)
            .map(|i| {
                let (x, y) = (
                    (i % w) as f32 - w as f32 / 2.0,
                    (i / w) as f32 - h as f32 / 2.0,
                );
                (r + 0.5 - x.hypot(y)).clamp(0.0, 1.0)
            })
            .collect()
    }

    #[test]
    fn shoreline_takes_the_land_modifiers() {
        let (w, h) = (120, 120);
        let c = lake(w, h, 40.3);
        let (dist, outlines) = shore(&c, w, h);
        let plain = Shoreline::default();
        let (lines, paths) = shoreline(&outlines, &dist, w, h, &plain);
        assert_eq!(lines.len(), 1);
        assert_eq!(paths.len(), 1, "solid by default");

        // Dashed: `dash_dark` sets the ink at the shore, as on dark ground.
        let mut dashed = Shoreline::default();
        dashed.style.dash_dark = 0.5;
        assert!(shoreline(&outlines, &dist, w, h, &dashed).1.len() > 10);

        // Two lines: the second one `space` px out, inside the lake.
        let mut double = Shoreline {
            lines: 2,
            ..Shoreline::default()
        };
        double.style.space = 6.0;
        let (lines, _) = shoreline(&outlines, &dist, w, h, &double);
        assert_eq!(lines.len(), 2);
        let r = |l: &crate::Line| {
            l.iter()
                .map(|p| (p[0] - w as f32 / 2.0).hypot(p[1] - h as f32 / 2.0))
                .sum::<f32>()
                / l.len() as f32
        };
        assert!((r(&lines[0]) - r(&lines[1]) - 6.0).abs() < 1.0);

        // By hand: the line wanders but stays a closed ring near the shore.
        let mut hand = Shoreline::default();
        hand.style.mod_wobble = 30.0;
        hand.style.mod_smooth = 2.0;
        let (lines, _) = shoreline(&outlines, &dist, w, h, &hand);
        let l = &lines[0];
        assert_eq!(l.first(), l.last());
        let off: Vec<f32> = l
            .iter()
            .map(|p| (p[0] - 60.0).hypot(p[1] - 60.0) - 40.3)
            .collect();
        assert!(off.iter().any(|d| d.abs() > 0.5), "it wanders");
        assert!(off.iter().all(|d| d.abs() < 5.0), "but not far");
    }

    #[test]
    fn shoreline_is_a_flag_or_a_style() {
        #[derive(Deserialize, Serialize)]
        struct Doc {
            #[serde(with = "shoreline_option", default = "shoreline_option::default")]
            shoreline: Option<Shoreline>,
        }
        let read = |s: &str| serde_json::from_str::<Doc>(s).unwrap().shoreline;
        assert_eq!(read("{}"), Some(Shoreline::default()));
        assert_eq!(read(r#"{"shoreline": false}"#), None);
        let sh =
            read(r#"{"shoreline": {"lines": 2, "style": {"dash_dark": 0.4, "bs": 2}}}"#).unwrap();
        assert_eq!(
            (sh.lines, sh.style.dash_dark, sh.style.batch_stroke),
            (2, 0.4, 2.0)
        );
        assert_eq!(sh.style.dash_light, 1.0, "the rest from Style::shoreline");
        let json = |shoreline| serde_json::to_string(&Doc { shoreline }).unwrap();
        assert_eq!(json(Some(Shoreline::default())), r#"{"shoreline":true}"#);
        assert_eq!(json(None), r#"{"shoreline":false}"#);
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
