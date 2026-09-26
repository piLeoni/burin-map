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
    ("space", "Distance between neighbouring lines. Light and dark set the range from open to dense."),
    ("dash", "How solid each line is along its length. Light and dark set the ink fraction at the extremes."),
    ("mod", "Hand-drawn character: irregularity, extra breaks, wander on flat ground, curve smoothness."),
    ("input", "The 0…1 field built from the terrain and the sun. Space and dash read from this."),
    ("sun", "Where the light comes from."),
    ("engine", "How streamlines are traced — usually leave alone."),
    ("batch", "Per-pass settings: stroke width, seed, and the input floor for this pass."),
];

macro_rules! style {
    ($(
        $group:ident : $name:ident ($short:literal) : $ty:ty = $default:expr, $help:expr;
    )*) => {
        #[derive(Clone, Debug, PartialEq)]
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
        Self::PARAMS.iter().find(|p| p.name == name || p.short == name).map(|p| p.name)
    }
}

style! {
    space: space ("s"): f32 = 4.8, "base distance between neighbouring lines, px";
    space: space_light ("sl"): f32 = 1.0, "space multiplier in full light (>1 = more open)";
    space: space_dark ("sd"): f32 = 0.7, "space multiplier in full dark (<1 = denser)";
    space: space_steep ("ss"): f32 = 0.4, "influence of steep ground on spacing (0 = off, 1 = full)";
    space: space_steep_gamma ("ssg"): f32 = 1.0, "curve of how fast steepness tightens space";

    dash: dash_light ("dl"): f32 = 0.15, "ink fraction of the period in full light (0…1)";
    dash: dash_dark ("dd"): f32 = 1.0, "ink fraction of the period in full dark (0…1)";
    dash: dash_period ("dp"): f32 = 2.2, "dash + gap length, in multiples of space";
    dash: dash_cutoff ("dc"): f32 = 0.03, "below this input draw nothing";
    dash: dash_drop ("ddrop"): f32 = 1.0, "drop random dashes on flat/light ground (0 = off, 1 = full)";

    mod: mod_jitter ("mj"): f32 = 0.45, "irregularity of dash length (0 = mechanical); fades in solid dark";
    mod: mod_breaks ("mb"): f32 = 0.15, "chance of a small break in an otherwise solid run";
    mod: mod_wobble ("mw"): f32 = 20.0, "random swing of line direction on flat ground, degrees";
    mod: mod_wobble_size ("mws"): f32 = 12.0, "size of those wobble bends, px";
    mod: mod_smooth ("ms"): f32 = 6.0, "smooth the surface lines follow: higher = broader curves";

    input: input_flat ("if"): f32 = 0.25, "input value of flat ground";
    input: input_gain ("ig"): f32 = 3.0, "contrast: how fast slopes go solid (dark) or empty (light)";
    input: input_dark_gamma ("idg"): f32 = 0.7, "<1 pushes dark areas toward solid sooner";
    input: input_light_gain ("ilg"): f32 = 1.5, ">1 makes lit slopes bleach sooner";
    input: input_light_floor ("ilf"): f32 = 0.06, "input kept in full light (floor under bleaching)";
    input: input_blur ("ib"): f32 = 1.8, "smooth the detail shading";
    input: input_broad_blur ("ibb"): f32 = 11.5, "smooth the massif-scale shading";
    input: input_broad ("ibr"): f32 = 0.5, "mix of massif shading (0 = detail only, 1 = massif only)";

    sun: sun_azimuth ("sa"): f32 = 315.0, "sun direction, degrees clockwise from north";
    sun: sun_altitude ("salt"): f32 = 45.0, "sun elevation above the horizon, degrees";
    sun: sun_exag ("sx"): f32 = 4.0, "vertical exaggeration for shading";

    engine: engine_step ("es"): f32 = 1.0, "integration step when tracing lines, px";
    engine: engine_min ("em"): f32 = 4.0, "drop streamlines shorter than this, px";
    engine: engine_stop ("esp"): f32 = 0.5, "stop a line when closer than space × this to another";
    engine: engine_fill ("ef"): f32 = 0.8, "seed a mid-gap line when a gap exceeds 2 × this × space (1 = off)";

    batch: batch_floor ("bf"): f32 = 0.0, "minimum input for this pass (0 = draw on all walkable land)";
    batch: batch_stroke ("bs"): f32 = 1.2, "stroke width, px";
    batch: batch_seed ("bseed"): u64 = 1, "random seed for this pass (line order, dash jitter)";
}

/// One drawing pass: streamlines at `angle` degrees relative to the contour
/// tangent (0 = along level curves, 90 = along the slope).
#[derive(Clone, Debug)]
pub struct Batch {
    pub angle: f32,
    pub style: Style,
}

impl Batch {
    pub fn along(style: Style) -> Self {
        Batch { angle: 0.0, style }
    }
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
        assert_eq!(Style::PARAMS.len(), 33);
    }

    #[test]
    fn short_names_are_unique() {
        let mut all: Vec<&str> = Style::PARAMS.iter().flat_map(|p| [p.name, p.short]).collect();
        let n = all.len();
        all.sort();
        all.dedup();
        assert_eq!(all.len(), n);
    }
}
