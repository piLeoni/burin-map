//! All tunable parameters of a drawing. Lengths are in output pixels, so the
//! same style gives the same look whatever the area and scale of the map.
//!
//! Every parameter has a descriptive name, prefixed by its group, and a short
//! form for when you know what you're doing:
//!   space_*   distance between lines (range via space_light / space_dark)
//!   dash_*    ink along each line   (range via dash_light / dash_dark)
//!   mod_*     character (jitter, breaks, wobble, smooth)
//!   input_*   0…1 field from DEM + sun
//!   sun_*     sun direction / elevation
//!   engine_*  streamline tracer
//!   batch_*   per-pass (floor, stroke, seed)

use serde::{Deserialize, Serialize};

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
    ("mod", "Hand-drawn character: uneven dashes, small breaks, wandering on flat ground, smooth curves."),
    ("input", "The shading input, 0 to 1, computed from the terrain and the sun. Spacing and dashes follow it."),
    ("sun", "Where the light comes from."),
    ("engine", "How lines are traced. Usually best left alone."),
    ("batch", "Settings for this batch only: line width, random seed, and where it draws."),
    ("water", "Lines on water: where they start and end away from the shore, and how they fade. Water batches only."),
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
}

style! {
    space: space ("s"): f32 = 4.8, "distance between neighbouring lines, in px";
    space: space_light ("sl"): f32 = 1.0, "spacing multiplier on fully lit slopes; above 1 opens them up";
    space: space_dark ("sd"): f32 = 0.7, "spacing multiplier in full shadow; below 1 packs lines closer";
    space: space_steep ("ss"): f32 = 0.4, "how much steep ground tightens the spacing: 0 off, 1 full";
    space: space_steep_gamma ("ssg"): f32 = 1.0, "shape of that tightening; higher values keep it for the steepest ground";

    dash: dash_light ("dl"): f32 = 0.15, "inked share of each dash period on fully lit slopes, 0 to 1";
    dash: dash_dark ("dd"): f32 = 1.0, "inked share of each dash period in full shadow, 0 to 1";
    dash: dash_period ("dp"): f32 = 2.2, "length of one dash plus its gap, as a multiple of the spacing";
    dash: dash_cutoff ("dc"): f32 = 0.03, "draw nothing where the shading input is below this";
    dash: dash_drop ("ddrop"): f32 = 1.0, "randomly leave out dashes on flat and lit ground: 0 off, 1 full";

    mod: mod_jitter ("mj"): f32 = 0.45, "random variation in dash length (0 is mechanical); fades out where lines go solid";
    mod: mod_breaks ("mb"): f32 = 0.15, "chance of a small break in an otherwise solid line";
    mod: mod_wobble ("mw"): f32 = 20.0, "how far lines wander off course on flat ground, in degrees";
    mod: mod_wobble_size ("mws"): f32 = 12.0, "length of each wander, in px";
    mod: mod_smooth ("ms"): f32 = 6.0, "smoothing of the terrain the lines follow; higher gives broader curves";

    input: input_flat ("if"): f32 = 0.25, "shading input given to flat ground, 0 to 1";
    input: input_gain ("ig"): f32 = 3.0, "contrast: how quickly slopes turn solid in shadow and empty in light";
    input: input_dark_gamma ("idg"): f32 = 0.7, "below 1, shadowed ground turns solid sooner";
    input: input_light_gain ("ilg"): f32 = 1.5, "above 1, lit slopes fade to white sooner";
    input: input_light_floor ("ilf"): f32 = 0.06, "shading kept on fully lit slopes, so they never fade out completely";
    input: input_blur ("ib"): f32 = 1.8, "smoothing of the fine-detail shading";
    input: input_broad_blur ("ibb"): f32 = 11.5, "smoothing of the large-scale shading (whole ridges and massifs)";
    input: input_broad ("ibr"): f32 = 0.5, "blend of fine and large-scale shading: 0 fine only, 1 large-scale only";

    sun: sun_azimuth ("sa"): f32 = 315.0, "direction the light comes from, in degrees clockwise from north";
    sun: sun_altitude ("salt"): f32 = 45.0, "height of the sun above the horizon, in degrees";
    sun: sun_exag ("sx"): f32 = 4.0, "vertical exaggeration of the terrain when computing the shading";

    engine: engine_step ("es"): f32 = 1.0, "step length when tracing a line, in px";
    engine: engine_min ("em"): f32 = 4.0, "leave out lines shorter than this, in px";
    engine: engine_stop ("esp"): f32 = 0.5, "end a line when it comes closer to another than this fraction of the spacing";
    engine: engine_fill ("ef"): f32 = 0.8, "start a new line in any gap wider than 2 × this × the spacing (1 turns it off)";

    batch: batch_floor ("bf"): f32 = 0.0, "this batch draws only where the shading input is at least this; 0 draws on all land";
    batch: batch_stroke ("bs"): f32 = 1.2, "line width, in px";
    batch: batch_seed ("bseed"): u64 = 1, "random seed; change it for another variation of the same style";

    water: water_start ("wst"): f32 = 2.0, "gap between the shore and the first line, in px";
    water: water_reach ("wr"): f32 = 60.0, "how far out from the shore the lines go, in px";
    water: water_fade ("wf"): f32 = 1.0, "how the lines fade towards that distance: above 1 sooner, below 1 later";
    water: water_size ("ws"): f32 = 14.0, "length of each wave mark, in px (draw arcs, dashes or waves)";
}

/// One drawing pass: streamlines at `angle` degrees relative to the contour
/// tangent (0 = along level curves, 90 = along the slope).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Batch {
    /// Optional label; only used for display (logs, the web app's cards).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default)]
    pub angle: f32,
    pub style: Style,
}

impl Batch {
    pub fn along(style: Style) -> Self {
        Batch {
            name: None,
            angle: 0.0,
            style,
        }
    }
}

impl Style {
    /// Whether the land's terrain fields are built from this parameter: the
    /// shading (`input_*`, `sun_*`), how it sets the spacing, and how lines
    /// wander and smooth. Batches that agree on all of them share one set of
    /// fields; a batch that changes one gets its own.
    pub fn is_terrain(name: &str) -> bool {
        Self::PARAMS.iter().any(|p| {
            p.name == name
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
/// the batch before, and the default only in the first. Everything else left
/// out takes its default. For `#[serde(deserialize_with = "…")]`.
pub fn batches_inheriting<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<Batch>, D::Error> {
    #[derive(Deserialize)]
    struct Given {
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        angle: f32,
        #[serde(default)]
        style: std::collections::BTreeMap<String, f64>,
    }
    let given = Vec::<Given>::deserialize(d)?;
    let mut out: Vec<Batch> = Vec::with_capacity(given.len());
    for g in given {
        let mut style = Style::default();
        if let Some(prev) = out.last() {
            style.inherit_terrain(&prev.style);
        }
        for (name, v) in g.style {
            // Unknown names are ignored.
            style.set(&name, v);
        }
        out.push(Batch {
            name: g.name,
            angle: g.angle,
            style,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get_by_name() {
        let mut st = Style::default();
        assert!(st.set("space", 6.0));
        assert_eq!(st.get("space"), Some(6.0));
        assert!(st.set("sl", 1.8));
        assert_eq!(st.space_light, 1.8);
        assert!(st.set("bseed", 9.0));
        assert_eq!(st.batch_seed, 9);
        assert!(!st.set("spacing", 1.0));
        assert!(!st.set("nope", 1.0));
        assert_eq!(Style::PARAMS.len(), 37);
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

    #[test]
    fn round_trip_batches() {
        let batches = vec![
            Batch::along(Style::default()),
            Batch {
                name: Some("Valleys".into()),
                angle: 90.0,
                style: Style {
                    space: 6.0,
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
        assert_eq!(back[1].style.space, 6.0);
        assert_eq!(back[1].style.batch_floor, 0.4);
    }

    #[test]
    fn partial_style_uses_defaults() {
        let json = r#"[{"angle": 45, "style": {"space": 7.5}}]"#;
        let back = from_json_batches(json).expect("should parse");
        assert_eq!(back.len(), 1);
        assert_eq!(back[0].angle, 45.0);
        assert_eq!(back[0].style.space, 7.5);
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
            {"style": {"sun_azimuth": 45, "space": 6}},
            {"angle": 90},
            {"style": {"sun_azimuth": 200}}
        ]"#;
        let b = from_json_batches(json).unwrap();
        assert_eq!(b[1].style.sun_azimuth, 45.0, "inherited");
        assert_eq!(
            b[1].style.space,
            Style::default().space,
            "not terrain: default"
        );
        assert_eq!(b[2].style.sun_azimuth, 200.0);
        assert!(b[0].style.same_terrain(&b[1].style));
        assert!(!b[1].style.same_terrain(&b[2].style));
    }

    #[test]
    fn unknown_fields_ignored() {
        let json = r#"[{"angle": 0, "style": {"space": 5, "nonsense": 99}}]"#;
        let back = from_json_batches(json).expect("should parse");
        assert_eq!(back[0].style.space, 5.0);
    }
}
