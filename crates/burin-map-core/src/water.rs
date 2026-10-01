//! Lines on water. The same tracer and dashes as on land, fed with fields made
//! from the distance to the shore instead of from the terrain: the input is 1
//! at the shore and fades to 0 at `water_reach`. Line spacing uses
//! [`Style::water_space_reach`], not `water_reach`, so a wide shelf trace does
//! not squeeze lines together.

use rayon::prelude::*;
use serde::{Deserialize, Deserializer, Serialize};

use crate::depth::{depth_weight, WaterWhere};
use crate::fields::{wobble, Fields};
use crate::grid::{blur, gradient};
use crate::pen::{Cap, Ink};
use crate::rng::Rng;
use crate::style::Style;
use crate::Line;

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

/// What a water batch draws.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Draw {
    /// Lines that fill the water, as on land.
    #[default]
    Lines,
    /// Rows of strokes, staggered and broken, bending together with the
    /// swell and waving as by hand: the sea of Olaus Magnus' Carta Marina.
    Ripples,
    /// Stippling, denser towards the coast, as on Ortelius' maps.
    Dots,
    /// A filled area; land batches only.
    Fill,
}

/// One drawing pass on water.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct WaterBatch {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_expr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width_expr: Option<String>,
    #[serde(skip_serializing_if = "crate::style::is_zero")]
    pub width_steps: u32,
    #[serde(skip_serializing_if = "Cap::is_round")]
    pub cap: Cap,
    pub follow: Follow,
    pub angle: f32,
    pub draw: Draw,
    /// Optional depth window in metres below the surface.
    #[serde(
        default,
        skip_serializing_if = "crate::depth::where_inactive",
        rename = "where"
    )]
    pub r#where: WaterWhere,
    /// Missing parameters take [`Style::drawn`]'s values for what the batch
    /// draws, not the land's.
    pub style: Style,
}

impl WaterBatch {
    pub fn ink(&self) -> Ink<'_> {
        Ink {
            stroke: self.style.batch_stroke,
            layer: self.layer.as_deref(),
            color: self.color.as_deref(),
            color_expr: self.color_expr.as_deref(),
            width_expr: self.width_expr.as_deref(),
            width_steps: self.width_steps,
            cap: self.cap,
        }
    }
}

impl Default for WaterBatch {
    fn default() -> Self {
        WaterBatch {
            name: None,
            layer: None,
            color: None,
            color_expr: None,
            width_expr: None,
            width_steps: 0,
            cap: Cap::Round,
            follow: Follow::Shore,
            angle: 0.0,
            draw: Draw::Lines,
            r#where: WaterWhere::default(),
            style: Style::drawn(Draw::Lines),
        }
    }
}

impl<'de> Deserialize<'de> for WaterBatch {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(default)]
        struct Given {
            name: Option<String>,
            layer: Option<String>,
            color: Option<String>,
            color_expr: Option<String>,
            width_expr: Option<String>,
            width_steps: u32,
            cap: Cap,
            follow: Follow,
            angle: f32,
            draw: Draw,
            #[serde(default, rename = "where")]
            r#where: WaterWhere,
            style: std::collections::BTreeMap<String, f64>,
        }
        impl Default for Given {
            fn default() -> Self {
                Given {
                    name: None,
                    layer: None,
                    color: None,
                    color_expr: None,
                    width_expr: None,
                    width_steps: 0,
                    cap: Cap::Round,
                    follow: Follow::Shore,
                    angle: 0.0,
                    draw: Draw::Lines,
                    r#where: WaterWhere::default(),
                    style: Default::default(),
                }
            }
        }
        let g = Given::deserialize(d)?;
        if g.draw == Draw::Fill {
            return Err(serde::de::Error::custom(
                "fills are land batches: put them in batches, with ground: sea for the water",
            ));
        }
        if let Some(e) = &g.width_expr {
            crate::color_expr::check_width(e).map_err(serde::de::Error::custom)?;
        }
        let mut style = Style::drawn(g.draw);
        for (name, v) in g.style {
            // Unknown names are ignored, as they are in land styles.
            style.set(&name, v);
        }
        Ok(WaterBatch {
            name: g.name,
            layer: g.layer,
            color: g.color,
            color_expr: g.color_expr,
            width_expr: g.width_expr,
            width_steps: g.width_steps,
            cap: g.cap,
            follow: g.follow,
            angle: g.angle,
            draw: g.draw,
            r#where: g.r#where,
            style,
        })
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

    /// Defaults for a water batch that draws `draw`.
    pub fn drawn(draw: Draw) -> Style {
        match draw {
            Draw::Lines | Draw::Fill => Style::water(),
            // Rows 9 px apart of strokes about 140 px long, waving gently by
            // hand over the whole sea, a little fuller near the coast.
            Draw::Ripples => Style {
                space: 9.0,
                water_size: 140.0,
                dash_light: 0.6,
                dash_dark: 0.9,
                mod_wobble: 18.0,
                mod_wobble_size: 150.0,
                mod_wave: 1.5,
                mod_wave_size: 40.0,
                water_start: 4.0,
                water_reach: 300.0,
                ..Style::water()
            },
            // A dot every 5 px, closer at the coast, sparser 250 px out.
            Draw::Dots => Style {
                space: 5.0,
                space_dark: 0.45,
                space_light: 1.3,
                water_size: 1.2,
                water_start: 3.0,
                water_reach: 250.0,
                ..Style::water()
            },
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_expr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width_expr: Option<String>,
    #[serde(skip_serializing_if = "crate::style::is_zero")]
    pub width_steps: u32,
    #[serde(skip_serializing_if = "Cap::is_round")]
    pub cap: Cap,
    /// Missing parameters take [`Style::shoreline`]'s values.
    #[serde(deserialize_with = "shore_style")]
    pub style: Style,
}

impl Shoreline {
    pub fn ink(&self) -> Ink<'_> {
        Ink {
            stroke: self.style.batch_stroke,
            layer: self.layer.as_deref().or(Some("shoreline")),
            color: self.color.as_deref(),
            color_expr: self.color_expr.as_deref(),
            width_expr: self.width_expr.as_deref(),
            width_steps: self.width_steps,
            cap: self.cap,
        }
    }
}

impl Default for Shoreline {
    fn default() -> Self {
        Shoreline {
            lines: 1,
            layer: None,
            color: None,
            color_expr: None,
            width_expr: None,
            width_steps: 0,
            cap: Cap::Round,
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
            Lines(Box<Shoreline>),
        }
        Ok(match Given::deserialize(d)? {
            Given::Flag(false) => None,
            Given::Flag(true) => default(),
            Given::Lines(s) => Some(*s),
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
    /// Metres below the surface on water; built only when [`crate::needs_depth`] says so.
    pub depth: Option<&'a [f32]>,
    /// Dedicated bathymetry fetch, when the document asked for one.
    pub depth_override: Option<&'a [f32]>,
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

/// Drop paths whose midpoint falls outside the batch's depth window.
pub fn filter_paths_by_depth(
    paths: Vec<Line>,
    depth: Option<&[f32]>,
    w: usize,
    h: usize,
    wh: &WaterWhere,
) -> Vec<Line> {
    if !wh.is_active() {
        return paths;
    }
    let Some(depth) = depth else {
        return paths;
    };
    paths
        .into_iter()
        .filter(|l| {
            let p = path_mid(l);
            let (x, y) = (p[0].round() as i32, p[1].round() as i32);
            if x < 0 || y < 0 || x as usize >= w || y as usize >= h {
                return false;
            }
            let i = y as usize * w + x as usize;
            depth_weight(depth[i], wh) > 0.02
        })
        .collect()
}

fn path_mid(l: &Line) -> [f32; 2] {
    if l.is_empty() {
        return [0.0, 0.0];
    }
    let m = l.len() / 2;
    l[m]
}

/// Fields for one water batch, from the distance to the shore.
pub fn water_fields(
    coverage: &[f32],
    dist: &[f32],
    depth: Option<&[f32]>,
    w: usize,
    h: usize,
    b: &WaterBatch,
) -> Fields {
    let st = &b.style;
    // No shore on the grid: the distance is infinite everywhere, and so the
    // input is 0; a finite stand-in keeps the blur and gradient clean.
    let far = (st.water_start + st.water_reach) * 2.0 + 1.0;
    let d: Vec<f32> = dist.par_iter().map(|&d| d.min(far)).collect();

    let (mut tx, mut ty) = directions(&d, w, h, b, (0.0, 0.0));
    wobble(&mut tx, &mut ty, w, h, st, |_| 1.0);

    let wh = &b.r#where;
    let input: Vec<f32> = d
        .par_iter()
        .zip(coverage)
        .enumerate()
        .map(|(i, (&d, &c))| {
            if c < 0.5 {
                return 0.0;
            }
            let mut t = tone(d, st);
            if let Some(dep) = depth {
                t *= depth_weight(dep[i], wh);
            }
            t
        })
        .collect();
    let loosen: Vec<f32> = d
        .par_iter()
        .map(|&dist| {
            if st.water_space_reach <= 0.0 {
                return 1.0f32;
            }
            let t = spacing_tone(dist, st);
            st.space_light + (st.space_dark - st.space_light) * t
        })
        .collect();
    // Very small multipliers pack streamlines until they cross; keep a sane floor.
    const SPACE_MUL_MIN: f32 = 0.72;
    let space = blur(&loosen, w, h, 3.0)
        .par_iter()
        .map(|k| st.space * k.clamp(SPACE_MUL_MIN, 4.0))
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

/// 1 at `water_start` px from the shore, fading to 0 `water_reach` px further
/// out; 0 nearer the shore than `water_start`.
fn tone(d: f32, st: &Style) -> f32 {
    fade_from_shore(d, st.water_start, st.water_reach, st.water_fade)
}

/// Like [`tone`], but for [`Style::water_space_reach`] (line spacing only).
fn spacing_tone(d: f32, st: &Style) -> f32 {
    if st.water_space_reach <= 0.0 {
        return 1.0;
    }
    fade_from_shore(d, st.water_start, st.water_space_reach, st.water_fade)
}

fn fade_from_shore(d: f32, start: f32, reach: f32, fade: f32) -> f32 {
    if d < start {
        return 0.0;
    }
    (1.0 - (d - start) / reach.max(1e-3))
        .clamp(0.0, 1.0)
        .powf(fade.max(1e-3))
}

/// The direction a batch follows at every pixel: along the shore turned by
/// `angle`, or `angle` from north. `none` where the shore gives no direction.
fn directions(
    d: &[f32],
    w: usize,
    h: usize,
    b: &WaterBatch,
    none: (f32, f32),
) -> (Vec<f32>, Vec<f32>) {
    let (sa, ca) = b.angle.to_radians().sin_cos();
    match b.follow {
        Follow::Shore => {
            let (gy, gx) = gradient(&blur(d, w, h, b.style.mod_smooth), w, h, 1.0);
            gx.par_iter()
                .zip(&gy)
                .map(|(&gx, &gy)| {
                    let m = gx.hypot(gy);
                    if m < 1e-6 {
                        return none;
                    }
                    let (u, v) = (-gy / m, gx / m);
                    (u * ca - v * sa, u * sa + v * ca)
                })
                .unzip()
        }
        // Image y points south, so north is -y.
        Follow::Map => (vec![sa; w * h], vec![-ca; w * h]),
    }
}

/// Dots scattered over the water: evenly, `space` px apart
/// (times `space_dark` at `water_start`, easing to `space_light` over
/// `water_reach`), never nearer the shore than `water_start`. Each is
/// `water_size` px across, turned as the batch follows, varied by
/// `mod_jitter` and bent by `mod_wobble`; `water_patches` gathers them in
/// patches. Solid: no dashes.
pub fn wave_marks(
    coverage: &[f32],
    dist: &[f32],
    depth: Option<&[f32]>,
    w: usize,
    h: usize,
    b: &WaterBatch,
) -> Vec<crate::Line> {
    let st = &b.style;
    // Far out, or with no shore at all, the distance is huge or infinite; a
    // finite ceiling keeps the gradient clean where there is one.
    let top = dist
        .iter()
        .copied()
        .filter(|d| d.is_finite())
        .fold(0.0f32, f32::max)
        + 1.0;
    let d: Vec<f32> = dist
        .par_iter()
        .map(|&d| if d.is_finite() { d } else { top })
        .collect();
    let (sa, ca) = b.angle.to_radians().sin_cos();
    let (tx, ty) = directions(&d, w, h, b, (sa, -ca));

    let at = |p: [f32; 2]| -> Option<usize> {
        let (x, y) = (p[0].round(), p[1].round());
        (x >= 0.0 && y >= 0.0 && (x as usize) < w && (y as usize) < h)
            .then(|| y as usize * w + x as usize)
    };
    let wh = &b.r#where;
    let open = |p: [f32; 2]| {
        at(p).is_some_and(|i| {
            coverage[i] >= 0.5
                && d[i] >= st.water_start
                && depth.is_none_or(|dep| depth_weight(dep[i], wh) > 0.02)
        })
    };
    let spacing = |i: usize| {
        let t = tone(d[i], st);
        st.space.max(1.0) * (st.space_light + (st.space_dark - st.space_light) * t).clamp(0.05, 4.0)
    };
    let lo = st.space.max(1.0) * st.space_light.min(st.space_dark).clamp(0.05, 4.0);
    let hi = st.space.max(1.0) * st.space_light.max(st.space_dark).clamp(0.05, 4.0);
    scatter(w, h, st, b.draw, (lo, hi), open, spacing, |i| {
        (tx[i], ty[i])
    })
}

/// Dots on land: `space` px apart as the batch's fields set it — closer in
/// shadow and on steep ground, as lines would be — wherever the batch may
/// draw and the input reaches `dash_cutoff`. Each is `water_size` px across,
/// turned along the batch's direction.
pub fn land_dots(f: &crate::Fields, st: &Style) -> Vec<crate::Line> {
    let (w, h) = (f.w, f.h);
    let at = |p: [f32; 2]| -> Option<usize> {
        let (x, y) = (p[0].round(), p[1].round());
        (x >= 0.0 && y >= 0.0 && (x as usize) < w && (y as usize) < h)
            .then(|| y as usize * w + x as usize)
    };
    let open = |p: [f32; 2]| at(p).is_some_and(|i| f.walkable[i] && f.input[i] >= st.dash_cutoff);
    let (lo, hi) = f
        .space
        .iter()
        .zip(&f.walkable)
        .filter(|(_, &k)| k)
        .fold((f32::MAX, 0.0f32), |(a, b), (&s, _)| (a.min(s), b.max(s)));
    if hi <= 0.0 {
        return Vec::new();
    }
    scatter(
        w,
        h,
        st,
        Draw::Dots,
        (lo.max(1.0), hi.max(1.0)),
        open,
        |i| f.space[i].max(1.0),
        |i| (f.tx[i], f.ty[i]),
    )
}

/// Dart throwing: jittered candidates in random order, each kept when no
/// kept mark lies within its spacing, `lo`…`hi` px. A mark is kept only if
/// all of it is `open`, turned along `dir`.
#[allow(clippy::too_many_arguments)]
fn scatter(
    w: usize,
    h: usize,
    st: &Style,
    draw: Draw,
    (lo, hi): (f32, f32),
    open: impl Fn([f32; 2]) -> bool,
    spacing: impl Fn(usize) -> f32,
    dir: impl Fn(usize) -> (f32, f32),
) -> Vec<crate::Line> {
    let at = |p: [f32; 2]| -> Option<usize> {
        let (x, y) = (p[0].round(), p[1].round());
        (x >= 0.0 && y >= 0.0 && (x as usize) < w && (y as usize) < h)
            .then(|| y as usize * w + x as usize)
    };
    let mut rng = Rng::new(st.batch_seed.wrapping_add(0x3A7E));
    let pitch = (lo * 0.5).max(2.0);
    let (cols, rows) = (
        (w as f32 / pitch).ceil() as usize,
        (h as f32 / pitch).ceil() as usize,
    );
    let mut cands: Vec<[f32; 2]> = (0..cols * rows)
        .map(|k| {
            let (cx, cy) = ((k % cols) as f32, (k / cols) as f32);
            [
                (cx + rng.f64() as f32) * pitch,
                (cy + rng.f64() as f32) * pitch,
            ]
        })
        .collect();
    for i in (1..cands.len()).rev() {
        let j = (rng.f64() * (i + 1) as f64) as usize;
        cands.swap(i, j.min(i));
    }
    let cell = hi.max(1.0);
    let (gw, gh) = (
        (w as f32 / cell).ceil() as usize + 1,
        (h as f32 / cell).ceil() as usize + 1,
    );
    let mut grid: Vec<Vec<[f32; 2]>> = vec![Vec::new(); gw * gh];

    // Patches: smooth noise over cells a few marks wide; the more
    // `water_patches`, the higher the bar a candidate's noise must clear.
    let patches = st.water_patches.clamp(0.0, 1.0);
    let noise = noise_field(w, h, hi * 4.0, &mut rng);

    let mut marks = Vec::new();
    for p in cands {
        let Some(i) = at(p) else { continue };
        if !open(p) || noise(p) < patches * 0.75 {
            continue;
        }
        let r = spacing(i);
        let (gx, gy) = ((p[0] / cell) as usize, (p[1] / cell) as usize);
        let near = (gy.saturating_sub(1)..(gy + 2).min(gh)).any(|y| {
            (gx.saturating_sub(1)..(gx + 2).min(gw)).any(|x| {
                grid[y * gw + x]
                    .iter()
                    .any(|q| (q[0] - p[0]).hypot(q[1] - p[1]) < r)
            })
        });
        if near {
            continue;
        }
        let jitter = st.mod_jitter.clamp(0.0, 1.0);
        let size = st.water_size.max(1.0) * (1.0 + jitter * rng.uniform(-0.5, 0.5) as f32);
        let turn = (jitter * 20.0 * rng.uniform(-1.0, 1.0) as f32).to_radians();
        let (s, c) = turn.sin_cos();
        let (tx, ty) = dir(i);
        let (u, v) = (tx * c - ty * s, tx * s + ty * c);
        let m = u.hypot(v).max(1e-6);
        let (u, v) = (u / m, v / m);
        let shape = mark(draw, size);
        let placed: Vec<crate::Line> = shape
            .iter()
            .map(|l| {
                l.iter()
                    .map(|q| [p[0] + q[0] * u - q[1] * v, p[1] + q[0] * v + q[1] * u])
                    .collect()
            })
            .collect();
        if !placed.iter().flatten().all(|&q| open(q)) {
            continue;
        }
        grid[gy * gw + gx].push(p);
        for mut l in placed {
            wander(&mut l, st, &mut rng);
            marks.push(l);
        }
    }
    marks
}

/// One mark `size` px across, centred on the origin, along +x.
fn mark(draw: Draw, size: f32) -> Vec<crate::Line> {
    match draw {
        Draw::Lines | Draw::Ripples | Draw::Fill => Vec::new(),
        Draw::Dots => vec![vec![[-size / 2.0, 0.0], [size / 2.0, 0.0]]],
    }
}

/// Rows `space` px apart, broken into strokes about `water_size` px long:
/// ink `dash_dark` at `water_start`, easing to `dash_light` over
/// `water_reach`. The rows run at `angle` from north, or along the shore,
/// bend together by `mod_wobble` over `mod_wobble_size` px like a swell, and
/// each waves by `mod_wave` as by hand; `water_patches` gathers them in
/// patches.
pub fn ripples(
    coverage: &[f32],
    dist: &[f32],
    depth: Option<&[f32]>,
    w: usize,
    h: usize,
    b: &WaterBatch,
) -> Vec<crate::Line> {
    let st = &b.style;
    let top = dist
        .iter()
        .copied()
        .filter(|d| d.is_finite())
        .fold(0.0f32, f32::max)
        + 1.0;
    let d: Vec<f32> = dist
        .par_iter()
        .map(|&d| if d.is_finite() { d } else { top })
        .collect();
    let at = |p: [f32; 2]| -> Option<usize> {
        let (x, y) = (p[0].round(), p[1].round());
        (x >= 0.0 && y >= 0.0 && (x as usize) < w && (y as usize) < h)
            .then(|| y as usize * w + x as usize)
    };
    let gap = st.space.max(1.5);
    let stroke = st.water_size.max(4.0);

    // The rows before they ripple: straight across the map, or the
    // distance's contours out from the shore.
    let rows: Vec<crate::Line> = match b.follow {
        Follow::Map => {
            let (sa, ca) = b.angle.to_radians().sin_cos();
            let (u, n) = ([sa, -ca], [ca, sa]);
            let corners = [
                [0.0, 0.0],
                [w as f32, 0.0],
                [0.0, h as f32],
                [w as f32, h as f32],
            ];
            let along = |p: &[f32; 2], v: [f32; 2]| p[0] * v[0] + p[1] * v[1];
            let lo = |v| corners.iter().map(|p| along(p, v)).fold(f32::MAX, f32::min);
            let hi = |v| corners.iter().map(|p| along(p, v)).fold(f32::MIN, f32::max);
            let (u0, u1, n0, n1) = (lo(u), hi(u), lo(n), hi(n));
            let count = ((n1 - n0) / gap).ceil() as usize + 1;
            let steps = (u1 - u0).ceil() as usize + 1;
            (0..count)
                .map(|k| {
                    let o = n0 + k as f32 * gap;
                    (0..steps)
                        .map(|j| {
                            let s = u0 + j as f32;
                            [s * u[0] + o * n[0], s * u[1] + o * n[1]]
                        })
                        .collect()
                })
                .collect()
        }
        Follow::Shore => {
            let count = ((st.water_reach / gap).ceil() as usize).clamp(1, 400);
            (0..count)
                .into_par_iter()
                .flat_map_iter(|k| {
                    let level = st.water_start + k as f32 * gap + 0.013;
                    let coverage = d
                        .iter()
                        .map(|&d| (0.5 + d - level).clamp(0.0, 1.0))
                        .collect();
                    watermask::Mask {
                        width: w,
                        height: h,
                        coverage,
                    }
                    .outlines()
                    .into_iter()
                    .map(|l| resample(&smooth(&l, st.mod_smooth), 1.0))
                })
                .collect()
        }
    };

    let wh = &b.r#where;
    let depth_ok = |i: usize| depth.is_none_or(|dep| depth_weight(dep[i], wh) > 0.02);

    let mut rng = Rng::new(st.batch_seed.wrapping_add(0x21FF));
    let lump = (st.mod_wobble_size.max(8.0), gap * 30.0);
    let swell = noise_field(w, h, lump.0, &mut rng);
    let patches = st.water_patches.clamp(0.0, 1.0);
    let patch = noise_field(w, h, lump.1, &mut rng);
    let sway = st.mod_wobble_size.max(1.0) * st.mod_wobble.min(80.0).to_radians().tan() / 3.0;

    // Bent by the swell, the same for neighbouring rows...
    let swollen: Vec<crate::Line> = rows
        .into_iter()
        .filter(|row| row.len() >= 3)
        .map(|row| {
            (0..row.len())
                .map(|i| {
                    let (a, c) = (row[i.saturating_sub(1)], row[(i + 1).min(row.len() - 1)]);
                    let (dx, dy) = (c[0] - a[0], c[1] - a[1]);
                    let m = dx.hypot(dy).max(1e-6);
                    let off = sway * (swell(row[i]) * 2.0 - 1.0);
                    [row[i][0] - dy / m * off, row[i][1] + dx / m * off]
                })
                .collect()
        })
        .collect();
    // ...then each waving on its own, as by hand.
    let waved = crate::wave::undulate(swollen, st, st.batch_seed, None);

    let mut out = Vec::new();
    for row in waved {
        // Broken into strokes, with gaps as the ink says.
        let mut piece: crate::Line = Vec::new();
        let mut inked = true;
        let mut left = stroke * rng.uniform(0.0, 1.0) as f32;
        for (k, &p) in row.iter().enumerate() {
            if k > 0 {
                left -= (p[0] - row[k - 1][0]).hypot(p[1] - row[k - 1][1]);
            }
            let here =
                at(p).filter(|&i| coverage[i] >= 0.5 && d[i] >= st.water_start && depth_ok(i));
            if left <= 0.0 {
                inked = !inked;
                let ink = here.map_or(0.5, |i| {
                    let t = tone(d[i], st);
                    (st.dash_light + (st.dash_dark - st.dash_light) * t).clamp(0.02, 1.0)
                });
                let run = stroke * rng.uniform(0.5, 1.5) as f32;
                left = if inked {
                    run
                } else {
                    run * (1.0 - ink) / ink * rng.uniform(0.6, 1.4) as f32
                };
                if !inked && ink >= 0.999 {
                    inked = true;
                }
            }
            if inked && here.is_some() && patch(p) >= patches * 0.75 {
                piece.push(p);
            } else if piece.len() >= 2 {
                out.push(std::mem::take(&mut piece));
            } else {
                piece.clear();
            }
        }
        if piece.len() >= 2 {
            out.push(piece);
        }
    }
    out
}

/// `paths` breaking up as they near the shore: over `water_fray` px beyond
/// `water_start` they thin into shorter and shorter dashes, then dots, and
/// the edge where they give out wanders, so the lines do not stop along a
/// clean contour. Unchanged with `water_fray` 0.
pub fn fray(
    paths: Vec<crate::Line>,
    dist: &[f32],
    w: usize,
    h: usize,
    st: &Style,
) -> Vec<crate::Line> {
    let band = st.water_fray;
    if band <= 0.0 || w == 0 || h == 0 {
        return paths;
    }
    let mut rng = Rng::new(st.batch_seed ^ 0xF4A7);
    let wander = noise_field(w, h, band.max(8.0), &mut rng);
    let d = |p: [f32; 2]| {
        let (x, y) = (
            p[0].clamp(0.0, (w - 1) as f32),
            p[1].clamp(0.0, (h - 1) as f32),
        );
        dist[y as usize * w + x as usize].min(1e6)
    };
    // 0 where the lines give out, 1 where they are whole.
    let whole = move |p: [f32; 2]| (d(p) - st.water_start) / band + (wander(p) - 0.5) * 0.8;
    fray_along(paths, w, h, st, whole)
}

/// Like [`fray`], but along a batch [`WaterWhere`] depth limit (metres).
pub fn fray_by_depth(
    paths: Vec<crate::Line>,
    depth: Option<&[f32]>,
    wh: &WaterWhere,
    w: usize,
    h: usize,
    st: &Style,
) -> Vec<crate::Line> {
    let band = st.water_depth_fray;
    if band <= 0.0 || w == 0 || h == 0 || !wh.is_active() {
        return paths;
    }
    let Some(depth) = depth else {
        return paths;
    };
    let mut rng = Rng::new(st.batch_seed ^ 0xD3A7);
    let wander = noise_field(w, h, 48.0, &mut rng);
    let dep_at = |p: [f32; 2]| {
        let (x, y) = (p[0].round() as i32, p[1].round() as i32);
        if x < 0 || y < 0 || x as usize >= w || y as usize >= h {
            return 0.0;
        }
        depth[y as usize * w + x as usize]
    };
    let whole = move |p: [f32; 2]| {
        let dep = dep_at(p);
        let mut t = 1.0f32;
        if let Some(m) = wh.depth_max {
            t = t.min((m - dep) / band);
        }
        if let Some(m) = wh.depth_min {
            t = t.min((dep - m) / band);
        }
        t + (wander(p) - 0.5) * 0.35
    };
    fray_along(paths, w, h, st, whole)
}

fn fray_along(
    paths: Vec<crate::Line>,
    w: usize,
    h: usize,
    st: &Style,
    whole: impl Fn([f32; 2]) -> f32 + Send + Sync,
) -> Vec<crate::Line> {
    let _ = (w, h);
    let period = (st.space * 1.5).max(3.0);
    paths
        .into_par_iter()
        .enumerate()
        .flat_map_iter(|(i, l)| {
            if l.len() < 2 || l.iter().all(|&p| whole(p) >= 1.0) {
                return vec![l];
            }
            let mut rng = Rng::new(st.batch_seed ^ 0xF4A7 ^ (i as u64).wrapping_mul(0x9E37_79B9));
            let p = resample(&l, 1.0);
            let mut out = Vec::new();
            let mut piece: crate::Line = Vec::new();
            let (mut s, mut cell, mut ink) = (0.0f32, usize::MAX, (0.0f32, f32::MAX));
            for (k, &q) in p.iter().enumerate() {
                if k > 0 {
                    s += (q[0] - p[k - 1][0]).hypot(q[1] - p[k - 1][1]);
                }
                let c = (s / period) as usize;
                if c != cell {
                    cell = c;
                    let t = whole(q);
                    ink = if t >= 1.0 {
                        (0.0, f32::MAX)
                    } else {
                        let t = t.max(0.0);
                        let mut len = period * t.powf(0.8) * rng.uniform(0.6, 1.4) as f32;
                        if len < 1.0 {
                            len = if rng.uniform(0.0, 1.0) < (t * 1.5) as f64 {
                                1.0
                            } else {
                                0.0
                            };
                        }
                        let len = len.min(period);
                        let at = c as f32 * period + rng.uniform(0.0, (period - len) as f64) as f32;
                        (at, at + len)
                    };
                }
                if s >= ink.0 && s <= ink.1 {
                    piece.push(q);
                } else if !piece.is_empty() {
                    if piece.len() == 1 {
                        piece.push(q);
                    }
                    out.push(std::mem::take(&mut piece));
                }
            }
            if piece.len() >= 2 {
                out.push(piece);
            }
            out
        })
        .collect()
}

/// Smooth noise, 0…1, varying over about `cell` px.
fn noise_field(w: usize, h: usize, cell: f32, rng: &mut Rng) -> impl Fn([f32; 2]) -> f32 {
    let cell = cell.max(1.0);
    let (nw, nh) = (
        (w as f32 / cell) as usize + 3,
        (h as f32 / cell) as usize + 3,
    );
    let lattice: Vec<f32> = (0..nw * nh).map(|_| rng.f64() as f32).collect();
    move |p: [f32; 2]| {
        let (x, y) = ((p[0] / cell).max(0.0), (p[1] / cell).max(0.0));
        let (i, j) = (x as usize, y as usize);
        let e = |f: f32| f * f * (3.0 - 2.0 * f);
        let (fx, fy) = (e(x - i as f32), e(y - j as f32));
        let v = |a: usize, b: usize| lattice[b.min(nh - 1) * nw + a.min(nw - 1)];
        let top = v(i, j) + (v(i + 1, j) - v(i, j)) * fx;
        let bottom = v(i, j + 1) + (v(i + 1, j + 1) - v(i, j + 1)) * fx;
        top + (bottom - top) * fy
    }
}

/// True when concentric shore lines should follow distance contours instead of
/// the streamline tracer (avoids crossings when spacing varies).
#[must_use]
pub fn shore_contour_trace(b: &WaterBatch) -> bool {
    b.follow == Follow::Shore && b.draw == Draw::Lines && b.angle.abs() < 1e-3
}

/// Concentric lines parallel to the shore: distance contours stepped by
/// [`Style::space`] (times the near/far multipliers over [`Style::water_space_reach`]).
pub fn shore_rings(
    dist: &[f32],
    coverage: &[f32],
    depth: Option<&[f32]>,
    w: usize,
    h: usize,
    b: &WaterBatch,
) -> Vec<crate::Line> {
    let st = &b.style;
    let wh = &b.r#where;
    let top = dist
        .iter()
        .copied()
        .filter(|d| d.is_finite())
        .fold(0.0f32, f32::max);
    let max_d = (st.water_start + st.water_reach).min(top + 1.0);
    if max_d <= st.water_start + 0.01 || w == 0 || h == 0 {
        return Vec::new();
    }
    const SPACE_MUL_MIN: f32 = 0.72;
    let max_rings = (((max_d - st.water_start) / st.space.max(1.0)).ceil() as usize)
        .saturating_add(4)
        .clamp(1, 800);

    let mut level = st.water_start + 0.013;
    let mut rings = Vec::new();
    for _ in 0..max_rings {
        if level >= max_d {
            break;
        }
        let t = spacing_tone(level, st);
        let mul = if st.water_space_reach <= 0.0 {
            1.0
        } else {
            st.space_light + (st.space_dark - st.space_light) * t
        };
        let step = (st.space * mul.clamp(SPACE_MUL_MIN, 4.0)).max(1.0);

        let cov: Vec<f32> = dist
            .iter()
            .map(|&d| (0.5 + d - level).clamp(0.0, 1.0))
            .collect();
        let outlines = watermask::Mask {
            width: w,
            height: h,
            coverage: cov,
        }
        .outlines();

        let mut rng = Rng::new(st.batch_seed.wrapping_add((level * 512.0) as u64 ^ 0xA11C));
        for outline in outlines {
            if outline.len() < 2 {
                continue;
            }
            let mid = path_mid(&outline);
            let (x, y) = (mid[0].round() as i32, mid[1].round() as i32);
            if x < 0 || y < 0 || x as usize >= w || y as usize >= h {
                continue;
            }
            let i = y as usize * w + x as usize;
            if coverage[i] < 0.5 {
                continue;
            }
            let mut wt = tone(dist[i], st);
            if let Some(dep) = depth {
                wt *= depth_weight(dep[i], wh);
            }
            if wt < 0.02 {
                continue;
            }
            let mut l = smooth(&outline, st.mod_smooth);
            l = resample(&l, 1.0);
            wander(&mut l, st, &mut rng);
            rings.push(l);
        }
        level += step;
    }
    rings
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
            let lines =
                crate::wave::undulate(lines, st, st.batch_seed.wrapping_add(k as u64), None);
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
pub(crate) fn resample(l: &crate::Line, step: f32) -> crate::Line {
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
        let f = water_fields(&c, &dist, None, w, h, &b);
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
    fn shore_rings_widen_with_higher_space_dark() {
        let (w, h) = (120, 120);
        let c = lake(w, h, 45.0);
        let (dist, _) = shore(&c, w, h);
        let batch = |space_dark: f32| WaterBatch {
            style: Style {
                space: 6.0,
                space_dark,
                space_light: 1.4,
                water_start: 3.0,
                water_reach: 80.0,
                water_space_reach: 60.0,
                ..Style::water()
            },
            ..WaterBatch::default()
        };
        let tight = shore_rings(&dist, &c, None, w, h, &batch(0.85));
        let loose = shore_rings(&dist, &c, None, w, h, &batch(1.2));
        assert!(
            loose.len() < tight.len(),
            "wider spacing near the shore → fewer rings"
        );
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
    fn marks_and_ripples_keep_to_the_open_sea() {
        let (w, h) = (200, 200);
        let c = half(w, h);
        let (dist, _) = shore(&c, w, h);
        for (draw, style, least) in [
            (Draw::Dots, serde_json::json!({}), 100),
            (Draw::Ripples, serde_json::json!({}), 20),
        ] {
            let b: WaterBatch = serde_json::from_value(
                serde_json::json!({ "follow": "map", "angle": 90, "draw": draw, "style": style }),
            )
            .unwrap();
            assert_eq!(
                b.style.dash_light,
                Style::drawn(draw).dash_light,
                "{draw:?} defaults"
            );
            let run = || {
                if draw == Draw::Ripples {
                    ripples(&c, &dist, None, w, h, &b)
                } else {
                    wave_marks(&c, &dist, None, w, h, &b)
                }
            };
            let marks = run();
            assert!(marks.len() >= least, "{draw:?}: {}", marks.len());
            for p in marks.iter().flatten() {
                assert!(
                    p[0] < 100.0 - b.style.water_start + 1.5,
                    "{draw:?} too near the shore: {p:?}"
                );
            }
            assert_eq!(marks, run(), "{draw:?}: the same seed, the same drawing");
        }
        let lines: WaterBatch = serde_json::from_str("{}").unwrap();
        assert_eq!(lines.draw, Draw::Lines);
        assert_eq!(lines.style, Style::water());
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
        let f = water_fields(&c, &dist, None, w, h, &b);
        assert!(
            (f.tx[5] - 1.0).abs() < 1e-6 && f.ty[5].abs() < 1e-6,
            "east-west"
        );
    }
}
