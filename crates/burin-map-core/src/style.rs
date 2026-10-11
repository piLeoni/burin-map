//! All tunable parameters of a drawing. Lengths are in output pixels, so the
//! same style gives the same look whatever the area and scale of the map.
//!
//! Every parameter has a descriptive name, prefixed by its group, and a short
//! form for when you know what you're doing:
//!   space_*   distance between lines, px in light and in shadow
//!   dash_*    ink along each line, share in light and in shadow
//!   mod_*     character (jitter, flat, breaks, wobble, smooth)
//!   input_*   0…1 field from DEM + sun
//!   sun_*     sun direction / elevation
//!   engine_*  streamline tracer
//!   batch_*   per-pass (floor, stroke, seed)

use serde::{Deserialize, Serialize};

pub use crate::fills::Band;
use crate::pen::{Cap, Ink};
pub use crate::water::Draw;

/// One style parameter as exposed to CLI / Python / WASM / the web UI.
#[derive(Clone, Copy, Debug)]
pub struct Param {
    pub name: &'static str,
    pub short: &'static str,
    pub help: &'static str,
    pub group: &'static str,
}

/// Group id → short blurb for the UI.
pub const GROUPS: &[(&str, &str)] = &[
    ("space", "Distance between neighbouring lines, from open on lit slopes to dense in shadow."),
    ("dash", "How much of each line is inked, from broken dashes in light to solid in shadow."),
    ("mod", "Hand-drawn character: uneven dashes, a lottery on flat ground, small breaks, wandering, smooth curves."),
    ("input", "The shading input, 0 to 1, computed from the terrain and the sun. Spacing and dashes follow it."),
    ("sun", "Where the light comes from."),
    ("engine", "How lines are traced. Usually best left alone."),
    ("batch", "Settings for this batch only: line width, random seed, and where it draws."),
    ("water", "Lines on water: where they start and end away from the shore, and how they fade. Water batches only, but for the size and patches of dots on land."),
    ("mask", "How lines ease out at the edge of the regions a batch keeps to (within) or gives way to (outside). Regions come from the program drawing the map."),
];

macro_rules! style {
    ($(
        $group:ident : $name:ident ($short:literal) : $ty:ty = $default:expr, $help:expr;
    )*) => {
        #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
        #[serde(default)]
        pub struct Style {
            $( pub $name: $ty, )*
        }

        impl Default for Style {
            fn default() -> Self {
                Style { $( $name: $default, )* }
            }
        }

        impl Style {
            /// Every parameter in declaration order.
            pub const PARAMS: &'static [Param] = &[ $(
                Param {
                    name: stringify!($name),
                    short: $short,
                    help: $help,
                    group: stringify!($group),
                },
            )* ];

            /// Get by full or short name.
            pub fn get(&self, name: &str) -> Option<f64> {
                match Self::full_name(name)? {
                    $( stringify!($name) => Some(self.$name as f64), )*
                    _ => None,
                }
            }

            /// Set by full or short name; false if there is no such parameter.
            pub fn set(&mut self, name: &str, value: f64) -> bool {
                match Self::full_name(name) {
                    $( Some(stringify!($name)) => { self.$name = value as $ty; true } )*
                    _ => false,
                }
            }
        }
    };
}

impl Style {
    /// Full name for a full or short parameter name.
    pub fn full_name(name: &str) -> Option<&'static str> {
        Self::PARAMS
            .iter()
            .find(|p| p.name == name || p.short == name)
            .map(|p| p.name)
    }

    /// Even ink with every modifier off: what water, the shore and dots build
    /// their defaults on. The default style is for land lines, drawn by hand.
    pub fn plain() -> Style {
        Style {
            space_light: 4.8,
            space_dark: 3.4,
            dash_light: 0.15,
            dash_dark: 1.0,
            dash_cutoff: 0.0,
            mod_jitter: 0.0,
            mod_flat: 0.0,
            ..Style::default()
        }
    }

    /// The one distance between lines where a single one is needed, and the
    /// px the spacing fields scale from: the lit spacing.
    pub fn space_base(&self) -> f32 {
        self.space_light.max(0.1)
    }

    /// Spacing at tone `t` (0 lit, 1 shadow) as a multiple of [`Style::space_base`].
    pub fn space_mul(&self, t: f32) -> f32 {
        1.0 + (self.space_dark / self.space_base() - 1.0) * t
    }

    /// Inked share of a dash period at tone `t`, before any modifier.
    pub fn duty(&self, t: f32) -> f32 {
        let t = t.clamp(0.0, 1.0);
        let d = if self.dash_mid < 0.0 {
            self.dash_light + (self.dash_dark - self.dash_light) * t
        } else if t < 0.5 {
            self.dash_light + (self.dash_mid - self.dash_light) * t * 2.0
        } else {
            self.dash_mid + (self.dash_dark - self.dash_mid) * (t * 2.0 - 1.0)
        };
        d.clamp(0.0, 1.0)
    }

    /// Whether ink survives `dash_cutoff` at tone `t`; `u` gives a uniform
    /// 0…1, asked for only inside the fade.
    /// Nothing survives below `dash_cutoff × (1 − dash_cutoff_soft)`; from
    /// there up to the cutoff the chance rises as the square of the way
    /// across. Soft, lit ground thins out as the cutoff rises instead of
    /// vanishing at once: much of it sits on one tone, `input_light_floor`.
    pub fn passes_cutoff(&self, t: f32, u: impl FnOnce() -> f64) -> bool {
        if t >= self.dash_cutoff {
            return true;
        }
        let lo = self.dash_cutoff * (1.0 - self.dash_cutoff_soft.clamp(0.0, 1.0));
        if t < lo {
            return false;
        }
        let r = ((t - lo) / (self.dash_cutoff - lo)) as f64;
        u() < r * r
    }

    /// `t` rounded to `dash_levels` even levels from 0 to 1; as is below 2.
    pub fn quantize(&self, t: f32) -> f32 {
        let n = self.dash_levels;
        if n < 2 {
            return t;
        }
        let bin = ((t.clamp(0.0, 1.0) * n as f32) as u32).min(n - 1);
        bin as f32 / (n - 1) as f32
    }
}

style! {
    space: space_light ("sl"): f32 = 6.0, "distance between neighbouring lines on fully lit slopes, in px";
    space: space_dark ("sd"): f32 = 7.0, "distance between neighbouring lines in full shadow, in px";
    space: space_steep ("ss"): f32 = 0.4, "how much steep ground tightens the spacing: 0 off, 1 full";
    space: space_steep_gamma ("ssg"): f32 = 1.0, "shape of that tightening; higher values keep it for the steepest ground";

    dash: dash_light ("dl"): f32 = 0.3, "inked share of each dash period at tone 0 (full light), 0 to 1";
    dash: dash_dark ("dd"): f32 = 0.8, "inked share of each dash period at tone 1 (full shadow), 0 to 1";
    dash: dash_mid ("dm"): f32 = -1.0, "inked share at tone 0.5; below 0 halfway between dash_light and dash_dark";
    dash: dash_period ("dp"): f32 = 10.5, "length of one dash plus its gap, in px";
    dash: dash_levels ("dlv"): u32 = 0, "round the tone to this many even levels before inking, for flat zones of ink; 0 smooth";
    dash: dash_cutoff ("dc"): f32 = 0.15, "no ink below this tone, for the lightest ground; 0 off";
    dash: dash_cutoff_soft ("dcs"): f32 = 1.0, "how gradually ink fades out towards dash_cutoff: 0 a hard edge, 1 thinning all the way down from the cutoff to tone 0";
    dash: dash_solid ("dsol"): f32 = 1.0, "solid line where the tone is above this, whatever the modifiers; 1 off";

    mod: mod_jitter ("mj"): f32 = 1.0, "uneven dashes: varies dash and period length, keeping the average ink; 0 mechanical";
    mod: mod_flat ("mf"): f32 = 0.5, "on flat and lit ground, chance a dash is left out or takes the flat ground's ink; 0 off";
    mod: mod_breaks ("mb"): f32 = 0.0, "chance of a small break in an otherwise solid dash; 0 off";
    mod: mod_wobble ("mw"): f32 = 20.0, "how far lines wander off course on flat ground, in degrees";
    mod: mod_wobble_size ("mws"): f32 = 12.0, "length of each wander, in px";
    mod: mod_wave ("mwv"): f32 = 0.0, "how far each line swings to either side as it waves along its path, in px; 0 off";
    mod: mod_wave_size ("mwvs"): f32 = 16.0, "length of each of those waves, in px; it drifts along the line, as by hand";
    mod: mod_wave_zone ("mwz"): f32 = 0.0, "how much neighbouring lines wave together, so parallel lines keep their spacing: 0 each its own way, 1 all as one";
    mod: mod_wave_even ("mwe"): f32 = 0.0, "how evenly the lines wave, in step with their neighbours along the flow, like an engraved pattern: 0 as by hand, 1 perfectly even";
    mod: mod_smooth ("ms"): f32 = 6.0, "smoothing of the terrain the lines follow; higher gives broader curves";

    input: input_flat ("if"): f32 = 0.25, "shading input given to flat ground, 0 to 1";
    input: input_gain ("ig"): f32 = 3.0, "contrast: how quickly slopes turn solid in shadow and empty in light";
    input: input_dark_gamma ("idg"): f32 = 0.7, "below 1, shadowed ground turns solid sooner";
    input: input_light_gain ("ilg"): f32 = 1.5, "above 1, lit slopes fade to white sooner";
    input: input_light_floor ("ilf"): f32 = 0.06, "shading kept on fully lit slopes, so they never fade out completely";
    input: input_blur ("ib"): f32 = 1.8, "smoothing of the fine-detail shading";
    input: input_broad_blur ("ibb"): f32 = 11.5, "smoothing of the large-scale shading (whole ridges and massifs)";
    input: input_broad ("ibr"): f32 = 0.5, "blend of fine and large-scale shading: 0 fine only, 1 large-scale only";
    input: input_height ("ih"): f32 = 0.0, "how much height, rather than shade, sets the input: 1 the highest ground darkest, -1 the lowest, 0 shade only";

    sun: sun_azimuth ("sa"): f32 = 315.0, "direction the light comes from, in degrees clockwise from north";
    sun: sun_altitude ("salt"): f32 = 45.0, "height of the sun above the horizon, in degrees";
    sun: sun_exag ("sx"): f32 = 4.0, "vertical exaggeration of the terrain when computing the shading";

    engine: engine_step ("es"): f32 = 1.0, "step length when tracing a line, in px";
    engine: engine_min ("em"): f32 = 4.0, "leave out lines shorter than this, in px";
    engine: engine_stop ("esp"): f32 = 0.5, "end a line when it comes closer to another than this fraction of the spacing";
    engine: engine_fill ("ef"): f32 = 0.8, "start a new line in any gap wider than 2 × this × the spacing (1 turns it off)";
    engine: engine_stripes ("estr"): u32 = 0, "trace in this many stripes side by side, all at once: faster, but solid lines can show where they meet; 1 in one piece, 0 stripes 256 px wide";

    batch: batch_floor ("bf"): f32 = 0.0, "this batch draws only where the shading input is at least this; 0 draws on all land";
    batch: batch_stroke ("bs"): f32 = 1.2, "line width, in px";
    batch: batch_seed ("bseed"): u64 = 1, "random seed; change it for another variation of the same style";

    water: water_start ("wst"): f32 = 2.0, "gap between the shore and the first line, in px";
    water: water_reach ("wr"): f32 = 60.0, "how far out from the shore ink and tracing stay active, in px; does not set line spacing";
    water: water_space_reach ("wsr"): f32 = 120.0, "how far from the shore line spacing eases from space_dark to space_light, in px; 0 keeps spacing fixed at space_light";
    water: water_fade ("wf"): f32 = 1.0, "how the lines fade towards that distance: above 1 sooner, below 1 later";
    water: water_size ("ws"): f32 = 14.0, "length of each ripple stroke or dot, in px (draw ripples or dots; dots on land too)";
    water: water_patches ("wp"): f32 = 0.0, "how much ripples or dots gather in scattered patches: 0 evenly everywhere, 1 a few patches (dots on land too)";
    water: water_fray ("wfr"): f32 = 0.0, "how far out from water_start, in px, lines break up into shorter dashes and dots as they near the shore, with a ragged edge; 0 a clean edge";
    water: water_depth_fray ("wdf"): f32 = 0.0, "metres band at a batch depth limit where lines break up into dashes and dots with a ragged edge; uses where.depth_max or depth_min; 0 a clean cut";

    mask: mask_start ("mst"): f32 = 0.0, "no lines nearer a region's edge than this, in px: inside it for within, outside it for outside";
    mask: mask_reach ("mr"): f32 = 40.0, "distance past mask_start over which the shading, and with it the ink, eases in from nothing, in px";
    mask: mask_fade ("mfd"): f32 = 1.0, "how it eases: above 1 the ink comes later, below 1 sooner";
    mask: mask_fray ("mfr"): f32 = 0.0, "band past mask_start, in px, where lines break up into shorter dashes and dots with a ragged edge; 0 a clean edge";
    mask: mask_start_m ("mstm"): f32 = 0.0, "mask_start in metres on the ground; wins over mask_start when above 0";
    mask: mask_reach_m ("mrm"): f32 = 0.0, "mask_reach in metres on the ground; wins over mask_reach when above 0";
    mask: mask_fray_m ("mfrm"): f32 = 0.0, "mask_fray in metres on the ground; wins over mask_fray when above 0";
}

/// Which ground a land batch draws: the land, or the seabed under the water,
/// traced like land from the negative heights there.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Ground {
    #[default]
    Land,
    Sea,
}

impl Ground {
    pub fn is_land(&self) -> bool {
        *self == Ground::Land
    }
}

/// One drawing pass: streamlines at `angle` degrees relative to the contour
/// tangent (0 = along level curves, 90 = along the slope).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Batch {
    /// What the batch is called: in progress reports, and as its layer
    /// unless `layer` is set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// The layer of this batch's dashes; defaults to its [`Batch::label`].
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layer: Option<String>,
    /// Stroke colour as `#rgb` or `#rrggbb`. Black when omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// How the colour is decided at each dash; ignored when `color` is set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub color_expr: Option<crate::expr::Expr>,
    /// The line width at each dash, as a multiple of `batch_stroke`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width_expr: Option<crate::expr::Scalar>,
    /// Rounds the widths from `width_expr` to this many, each its own layer;
    /// 0 keeps them as they come.
    #[serde(skip_serializing_if = "is_zero")]
    pub width_steps: u32,
    /// How the ends of the dashes look.
    #[serde(skip_serializing_if = "Cap::is_round")]
    pub cap: Cap,
    #[serde(default)]
    pub angle: f32,
    #[serde(skip_serializing_if = "Ground::is_land")]
    pub ground: Ground,
    /// Lines, dots, or a fill; land has no ripples.
    #[serde(skip_serializing_if = "is_lines")]
    pub draw: Draw,
    /// The heights or depths a fill keeps to.
    #[serde(rename = "where", skip_serializing_if = "Band::is_open")]
    pub band: Band,
    /// Regions the lines keep to, fading out at their edge; several are
    /// taken together, as one area. Named here, given at draw time.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub within: Vec<String>,
    /// Regions the lines give way to, fading out as they near them.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub outside: Vec<String>,
    pub style: Style,
}

fn is_lines(d: &Draw) -> bool {
    *d == Draw::Lines
}

pub(crate) fn is_zero(n: &u32) -> bool {
    *n == 0
}

impl Batch {
    /// A run of streamlines with the default style. The typed API's way in,
    /// and what a document's `batches` becomes.
    pub fn lines() -> Self {
        Batch::along(Style::default())
    }

    /// Stippling, denser where the shading is darker: a dot every `space_light` px in light.
    pub fn dots() -> Self {
        Batch {
            draw: Draw::Dots,
            style: Style {
                water_size: LAND_DOT,
                ..Style::plain()
            },
            ..Batch::along(Style::default())
        }
    }

    /// A filled area over `ground`, its own rings. `color` is the ink; a fill
    /// with none is left light grey.
    pub fn filling(color: Option<&str>, color_expr: Option<crate::expr::Expr>, name: Option<&str>) -> Self {
        Batch {
            name: name.map(str::to_string),
            color: color.map(str::to_string),
            color_expr,
            draw: Draw::Fill,
            ..Batch::along(Style::default())
        }
    }

    /// The batch's name in reports and as its layer.
    pub fn named(mut self, name: &str) -> Self {
        self.name = Some(name.to_string());
        self
    }

    /// The colour every dash is inked with, as `#rgb` or `#rrggbb`.
    pub fn color(mut self, c: &str) -> Self {
        self.color = Some(c.to_string());
        self.color_expr = None;
        self
    }

    /// The colour at each dash, from a tree; wins over [`Batch::color`].
    pub fn shade(mut self, e: impl Into<crate::expr::Expr>) -> Self {
        self.color_expr = Some(e.into());
        self
    }

    /// The width at each dash, as a multiple of `style.batch_stroke`.
    pub fn width(mut self, e: impl Into<crate::expr::Scalar>) -> Self {
        self.width_expr = Some(e.into());
        self
    }

    /// How the ends of the dashes look.
    pub fn cap(mut self, cap: Cap) -> Self {
        self.cap = cap;
        self
    }

    /// The field the streamlines run along, degrees.
    pub fn angle(mut self, deg: f32) -> Self {
        self.angle = deg;
        self
    }

    /// What a fill is drawn over: land, or the sea.
    pub fn ground(mut self, g: Ground) -> Self {
        self.ground = g;
        self
    }

    /// How many widths `width_expr` is rounded to, each with its own layer.
    pub fn width_steps(mut self, n: u32) -> Self {
        self.width_steps = n;
        self
    }

    /// One style parameter by name, as a document's `style` block does.
    pub fn set(mut self, name: &str, v: f64) -> Self {
        self.style.set(name, v);
        self
    }

    /// Keep the lines to region `name` too, fading out at its edge.
    pub fn within(mut self, name: &str) -> Self {
        self.within.push(name.to_string());
        self
    }

    /// Have the lines give way to region `name` too, fading out as they near it.
    pub fn outside(mut self, name: &str) -> Self {
        self.outside.push(name.to_string());
        self
    }

    /// Whether the batch keeps to or gives way to any region.
    pub fn is_masked(&self) -> bool {
        !self.within.is_empty() || !self.outside.is_empty()
    }

    pub fn along(style: Style) -> Self {
        Batch {
            name: None,
            layer: None,
            color: None,
            color_expr: None,
            width_expr: None,
            width_steps: 0,
            cap: Cap::Round,
            angle: 0.0,
            ground: Ground::Land,
            draw: Draw::Lines,
            band: Band::default(),
            within: Vec::new(),
            outside: Vec::new(),
            style,
        }
    }

    /// What batch `i` is called in progress reports, and its layer unless it
    /// sets one: its `name`, else its `layer`, else as "2 dots".
    pub fn label(&self, i: usize) -> String {
        crate::progress::label(
            self.name.as_deref(),
            self.layer.as_deref(),
            i,
            self.draw.name(),
        )
    }

    /// The layer batch `i` draws on: its `layer`, else its label.
    pub fn layer_name(&self, i: usize) -> String {
        self.layer.clone().unwrap_or_else(|| self.label(i))
    }

    pub fn ink(&self) -> Ink<'_> {
        self.ink_scaled(1.0)
    }

    pub fn ink_scaled(&self, px_per_mm: f32) -> Ink<'_> {
        Ink {
            stroke: self.style.batch_stroke * px_per_mm,
            layer: self.layer.as_deref(),
            color: self.color.as_deref(),
            expr: self.color_expr.clone(),
            width: self.width_expr.clone(),
            width_steps: self.width_steps,
            cap: self.cap,
        }
    }
}

/// Dots on land are this many px across unless the style says otherwise.
const LAND_DOT: f32 = 1.2;

impl Style {
    /// Whether the land's terrain fields are built from this parameter: the
    /// shading (`input_*`, `sun_*`), how it sets the spacing, and how lines
    /// wander and smooth. Batches that agree on all of them share one set of
    /// fields; a batch that changes one gets its own.
    pub fn is_terrain(name: &str) -> bool {
        // input_height is laid over each batch's own fields, so it neither
        // splits the shared ones nor carries over.
        Self::PARAMS.iter().any(|p| {
            p.name == name
                && p.name != "input_height"
                && (matches!(p.group, "input" | "sun")
                    || matches!(
                        p.name,
                        "space_light"
                            | "space_dark"
                            | "space_steep"
                            | "space_steep_gamma"
                            | "mod_wobble"
                            | "mod_wobble_size"
                            | "mod_smooth"
                    ))
        })
    }

    /// Same terrain parameters: the same terrain fields.
    pub fn same_terrain(&self, other: &Style) -> bool {
        Self::PARAMS
            .iter()
            .filter(|p| Self::is_terrain(p.name))
            .all(|p| self.get(p.name) == other.get(p.name))
    }

    /// Take `from`'s terrain parameters.
    pub fn inherit_terrain(&mut self, from: &Style) {
        for p in Self::PARAMS.iter().filter(|p| Self::is_terrain(p.name)) {
            self.set(p.name, from.get(p.name).unwrap());
        }
    }
}

/// Land batches as written: a terrain parameter left out takes the value of
/// the batch before (fills aside), and the default only in the first.
/// Everything else left out takes its default. For `#[serde(deserialize_with = "…")]`.
pub fn batches_inheriting<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Batch>, D::Error> {
    #[derive(Deserialize)]
    struct Given {
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        layer: Option<String>,
        #[serde(default)]
        color: Option<String>,
        #[serde(default)]
        color_expr: Option<crate::expr::Expr>,
        #[serde(default)]
        width_expr: Option<crate::expr::Scalar>,
        #[serde(default)]
        width_steps: u32,
        #[serde(default)]
        cap: Cap,
        #[serde(default)]
        angle: f32,
        #[serde(default)]
        ground: Ground,
        #[serde(default)]
        draw: Draw,
        #[serde(default, rename = "where")]
        band: Band,
        #[serde(default)]
        within: Vec<String>,
        #[serde(default)]
        outside: Vec<String>,
        #[serde(default)]
        style: std::collections::BTreeMap<String, f64>,
    }
    let given = Vec::<Given>::deserialize(d)?;
    let mut out: Vec<Batch> = Vec::with_capacity(given.len());
    for g in given {
        if g.draw == Draw::Ripples {
            return Err(serde::de::Error::custom(
                "land batches draw lines, dots or fills; ripples are for water",
            ));
        }
        if g.draw != Draw::Fill && !g.band.is_open() {
            return Err(serde::de::Error::custom(
                "where on a land batch is for fills (draw: fill)",
            ));
        }
        if g.draw == Draw::Fill && !(g.within.is_empty() && g.outside.is_empty()) {
            return Err(serde::de::Error::custom(FILL_REGIONS));
        }
        let mut style = Style::default();
        if g.draw == Draw::Dots {
            style = Style {
                water_size: LAND_DOT,
                ..Style::plain()
            };
        }
        // Fills trace no terrain: they neither take it nor hand it on.
        if g.draw != Draw::Fill {
            if let Some(prev) = out.iter().rev().find(|b| b.draw != Draw::Fill) {
                style.inherit_terrain(&prev.style);
            }
        }
        for (name, v) in g.style {
            // Unknown names are ignored.
            style.set(&name, v);
        }
        out.push(Batch {
            name: g.name,
            layer: g.layer,
            color: g.color,
            color_expr: g.color_expr,
            width_expr: g.width_expr,
            width_steps: g.width_steps,
            cap: g.cap,
            angle: g.angle,
            ground: g.ground,
            draw: g.draw,
            band: g.band,
            within: g.within,
            outside: g.outside,
            style,
        });
    }
    Ok(out)
}

/// Why a fill naming regions is refused.
pub(crate) const FILL_REGIONS: &str = "within and outside are for lines and dots; fills don't keep to regions";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get_by_name() {
        let mut st = Style::default();
        assert!(st.set("space_light", 6.0));
        assert_eq!(st.get("space_light"), Some(6.0));
        assert!(st.set("sd", 1.8));
        assert_eq!(st.space_dark, 1.8);
        assert!(!st.set("space", 6.0));
        assert!(!st.set("dash_drop", 1.0));
        assert!(st.set("bseed", 9.0));
        assert_eq!(st.batch_seed, 9);
        assert!(!st.set("spacing", 1.0));
        assert!(!st.set("nope", 1.0));
        assert_eq!(Style::PARAMS.len(), 57);
    }

    #[test]
    fn short_names_are_unique() {
        let mut all: Vec<&str> = Style::PARAMS
            .iter()
            .flat_map(|p| [p.name, p.short])
            .collect();
        let n = all.len();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), n);
    }
}

/// Parse a JSON string into a single `Style`. Missing fields get their
/// default, extraneous ones are ignored (`#[serde(default)]`).
pub fn from_json_style(s: &str) -> Result<Style, serde_json::Error> {
    serde_json::from_str(s)
}

/// Serialise a `Style` to a compact JSON string (no whitespace).
pub fn to_json_style(st: &Style) -> String {
    serde_json::to_string(st).unwrap_or_default()
}

/// Parse a JSON string into a `Vec<Batch>`; see [`batches_inheriting`].
pub fn from_json_batches(s: &str) -> Result<Vec<Batch>, serde_json::Error> {
    batches_inheriting(&mut serde_json::Deserializer::from_str(s))
}

/// Serialise a slice of `Batch` into a compact JSON string.
pub fn to_json_batches(batches: &[Batch]) -> String {
    serde_json::to_string(batches).unwrap_or_default()
}

#[cfg(test)]
mod json_tests {
    use super::*;
    use crate::expr::{Bin, Field, Func, Scalar};

    #[test]
    fn round_trip_batches() {
        let batches = vec![
            Batch::along(Style::default()),
            Batch {
                name: Some("Valleys".into()),
                layer: None,
                color: None,
                color_expr: None,
                width_expr: Some(Scalar::Call(
                Func::Lerp,
                vec![
                    Scalar::Lit(2.0),
                    Scalar::Lit(0.5),
                    Scalar::Call(
                        Func::Clamp,
                        vec![
                            Scalar::Bin(
                                Bin::Div,
                                Box::new(Scalar::Field(Field::Elev)),
                                Box::new(Scalar::Lit(500.0)),
                            ),
                            Scalar::Lit(0.0),
                            Scalar::Lit(1.0),
                        ],
                    ),
                ],
            )),
                width_steps: 3,
                cap: Cap::RoundInside,
                angle: 90.0,
                ground: Ground::Sea,
                draw: Draw::Dots,
                band: Band::default(),
                within: vec!["parks".into()],
                outside: vec!["towns".into(), "roads".into()],
                style: Style {
                    space_light: 6.0,
                    batch_floor: 0.4,
                    ..Style::default()
                },
            },
        ];
        let json = to_json_batches(&batches);
        let back = from_json_batches(&json).expect("should parse");
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].name, None);
        assert_eq!(back[1].name.as_deref(), Some("Valleys"));
        assert_eq!(back[1].angle, 90.0);
        assert_eq!(back[1].draw, Draw::Dots);
        assert_eq!(back[1].width_expr, batches[1].width_expr);
        assert_eq!(back[1].width_steps, 3);
        assert_eq!(back[1].cap, Cap::RoundInside);
        assert!(json.contains(r#""cap":"round_inside""#));
        assert!(!to_json_batches(&batches[..1]).contains("cap"));
        assert_eq!(back[1].style.space_light, 6.0);
        assert_eq!(back[1].style.batch_floor, 0.4);
        assert_eq!(back[1].within, ["parks"]);
        assert_eq!(back[1].outside, ["towns", "roads"]);
        assert!(!to_json_batches(&batches[..1]).contains("within"));
    }

    #[test]
    fn fills_take_no_regions() {
        assert!(from_json_batches(r#"[{"draw": "fill", "within": ["parks"]}]"#).is_err());
        assert!(from_json_batches(r#"[{"draw": "dots", "outside": ["towns"]}]"#).is_ok());
    }

    #[test]
    fn partial_style_uses_defaults() {
        let json = r#"[{"angle": 45, "style": {"space_light": 7.5}}]"#;
        let back = from_json_batches(json).expect("should parse");
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].angle, 45.0);
        assert_eq!(back[0].style.space_light, 7.5);
        // untouched field keeps its default
        assert_eq!(back[0].style.batch_stroke, Style::default().batch_stroke);
    }

    #[test]
    fn empty_array_parses_but_is_empty() {
        assert!(from_json_batches("[]").unwrap().is_empty());
    }

    #[test]
    fn terrain_carries_over_to_the_next_batch() {
        let json = r#"[
            {"style": {"sun_azimuth": 45, "space_light": 6, "dash_light": 0.5}},
            {"angle": 90},
            {"style": {"sun_azimuth": 200}}
        ]"#;
        let b = from_json_batches(json).unwrap();
        assert_eq!(b[1].style.sun_azimuth, 45.0, "inherited");
        assert_eq!(b[1].style.space_light, 6.0, "spacing is terrain: inherited");
        assert_eq!(
            b[1].style.dash_light,
            Style::default().dash_light,
            "not terrain: default"
        );
        assert_eq!(b[2].style.sun_azimuth, 200.0);
        assert!(b[0].style.same_terrain(&b[1].style));
        assert!(!b[1].style.same_terrain(&b[2].style));
    }

    #[test]
    fn land_draws_lines_or_dots() {
        let b = from_json_batches(r#"[{"draw": "dots"}, {}]"#).unwrap();
        assert_eq!(b[0].draw, Draw::Dots);
        assert_eq!(b[0].style.water_size, LAND_DOT);
        assert_eq!(b[1].draw, Draw::Lines);
        assert!(from_json_batches(r#"[{"draw": "ripples"}]"#).is_err());
    }

    #[test]
    fn fills_keep_to_where() {
        let json = r##"[{"draw": "fill", "ground": "sea", "color": "#cde", "where": {"depth_min": 200}}]"##;
        let b = from_json_batches(json).unwrap();
        assert_eq!(b[0].draw, Draw::Fill);
        assert_eq!(b[0].band.depth_min, Some(200.0));
        let back = to_json_batches(&b);
        assert!(back.contains(r#""where":{"depth_min":200.0}"#), "{back}");
        assert_eq!(from_json_batches(&back).unwrap()[0].band, b[0].band);
        assert!(
            from_json_batches(r#"[{"where": {"elev_min": 100}}]"#).is_err(),
            "lines"
        );
        assert!(from_json_batches(r#"[{"draw": "fill", "where": {"height": 1}}]"#).is_err());
    }

    #[test]
    fn fills_leave_the_terrain_alone() {
        let json = r#"[
            {"style": {"sun_azimuth": 45}},
            {"draw": "fill", "style": {"input_blur": 6}},
            {"angle": 90}
        ]"#;
        let b = from_json_batches(json).unwrap();
        assert_eq!(b[1].style.sun_azimuth, Style::default().sun_azimuth);
        assert_eq!(b[2].style.sun_azimuth, 45.0);
        assert_eq!(b[2].style.input_blur, Style::default().input_blur);
    }

    #[test]
    fn width_expr_is_read_as_a_scalar() {
        assert!(from_json_batches(r#"[{"width_expr": ["lerp", 2, 0.5, "tone"]}]"#).is_ok());
        assert!(from_json_batches(r##"[{"width_expr": ["lerp", "#000", "#fff", "tone"]}]"##).is_ok());
        assert!(from_json_batches(r#"[{"width_expr": ["nope", 1]}]"#).is_err());
        assert!(from_json_batches(r#"[{"cap": "pointy"}]"#).is_err());
    }

    #[test]
    fn colour_expr_is_read_as_a_colour() {
        let b = from_json_batches(
            r##"[{"color_expr": ["lerp", "#e8dcc8", "#2a1810", "tone"]},
                 {"color_expr": 0.25}]"##,
        )
        .unwrap();
        assert!(matches!(b[0].color_expr, Some(crate::expr::Expr::Color(_))));
        assert!(matches!(b[1].color_expr, Some(crate::expr::Expr::Shade(_))));
    }

    #[test]
    fn unknown_fields_ignored() {
        let json = r#"[{"angle": 0, "style": {"space_light": 5, "nonsense": 99}}]"#;
        let back = from_json_batches(json).expect("should parse");
        assert_eq!(back[0].style.space_light, 5.0);
    }
}
