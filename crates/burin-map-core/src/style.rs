//! All tunable parameters of a drawing. Lengths are in output pixels, so the
//! same style gives the same look whatever the area and scale of the map.

macro_rules! style {
    ($( $(#[$group:meta])* $name:ident : $ty:ty = $default:expr, $help:expr; )*) => {
        #[derive(Clone, Debug, PartialEq)]
        pub struct Style {
            $( $(#[$group])* pub $name: $ty, )*
        }

        impl Default for Style {
            fn default() -> Self {
                Style { $( $name: $default, )* }
            }
        }

        impl Style {
            /// (name, help) of every parameter, in declaration order. Lets the
            /// CLI, Python and WASM bindings expose them without repeating them.
            pub const PARAMS: &'static [(&'static str, &'static str)] = &[ $( (stringify!($name), $help), )* ];

            pub fn get(&self, name: &str) -> Option<f64> {
                match name {
                    $( stringify!($name) => Some(self.$name as f64), )*
                    _ => None,
                }
            }

            /// Set a parameter by name; false if there is no such parameter.
            pub fn set(&mut self, name: &str, value: f64) -> bool {
                match name {
                    $( stringify!($name) => { self.$name = value as $ty; true } )*
                    _ => false,
                }
            }
        }
    };
}

style! {
    // Line layout
    spacing: f32 = 4.8, "base distance between neighbouring lines, px";
    spacing_steep: f32 = 0.6, "on the steepest ground, multiply spacing by this (1 = ignore slope)";
    spacing_dark: f32 = 0.7, "in deepest shadow, multiply spacing by this (1 = no denser; used with shade_gap)";
    spacing_lit: f32 = 1.0, "in full light, multiply spacing by this (>1 = thinner / more open; used with shade_gap)";
    spacing_gamma: f32 = 1.0, "curve of how fast steepness tightens spacing";
    test_ratio: f32 = 0.5, "stop a line when closer than spacing × this to another";
    fill_ratio: f32 = 0.8, "seed a mid-gap line when a gap exceeds 2 × this × spacing (1 = off)";
    step: f32 = 1.0, "integration step when tracing lines, px";
    min_len: f32 = 4.0, "drop streamlines shorter than this, px";

    // Smoothing of the terrain (gaussian sigma, px)
    flow_blur: f32 = 6.0, "smooth the surface lines follow: higher = broader curves";
    shade_blur: f32 = 1.8, "smooth the detail shading";
    broad_blur: f32 = 11.5, "smooth the massif-scale shading";
    broad_weight: f32 = 0.5, "mix of massif shading (0 = detail only, 1 = massif only)";
    wobble: f32 = 20.0, "random swing of line direction on flat ground, degrees";
    wobble_size: f32 = 12.0, "size of those wobble bends, px";

    // Light
    azimuth: f32 = 315.0, "sun direction, degrees clockwise from north";
    altitude: f32 = 45.0, "sun elevation above the horizon, degrees";
    z_exag: f32 = 4.0, "vertical exaggeration for shading";

    // Tone map (0 = empty … 1 = solid). Feeds both shade_dash and shade_gap.
    flat_level: f32 = 0.25, "tone of flat ground";
    shade_gain: f32 = 3.0, "contrast: how fast slopes go solid (shadow) or empty (light)";
    dark_gamma: f32 = 0.7, "<1 pushes shadows toward solid sooner";
    lit_gain: f32 = 1.5, ">1 makes lit slopes bleach sooner";
    lit_floor: f32 = 0.06, "tone kept in full light (floor under bleaching)";

    // Two parallel ways light/shadow affect the drawing (use either, or both):
    //   shade_dash — break lines into dashes (classic look)
    //   shade_gap  — open/tighten spacing between lines (diradamento)
    shade_dash: f32 = 1.0, "0 = continuous strokes everywhere; 1 = light/shadow mainly as dashes (default look)";
    shade_gap: f32 = 1.0, "0 = uniform line spacing; 1 = light/shadow also as denser/thinner gaps (see spacing_dark / spacing_lit)";

    // Dashes (matter most when shade_dash > 0)
    period: f32 = 2.2, "dash + gap length, in multiples of spacing";
    duty_min: f32 = 0.15, "dash fraction of the period at tone 0 (light)";
    duty_max: f32 = 1.0, "dash fraction of the period at tone 1 (shadow)";
    cutoff: f32 = 0.03, "below this tone draw nothing";
    jitter: f32 = 0.45, "irregularity of dash length/pitch (0 = mechanical); fades in solid shadow";
    breaks: f32 = 0.15, "chance of a small break in an otherwise solid run";

    // Per-hatch gate
    hatch_floor: f32 = 0.0, "minimum tone for this hatch pass (0 = draw on all walkable land)";

    // Output
    stroke: f32 = 1.2, "stroke width, px";
    seed: u64 = 1, "random seed (line order, dash jitter)";
}

/// One hatching pass: streamlines at `angle` degrees relative to the contour
/// tangent (0 = along level curves, 90 = along the slope).
#[derive(Clone, Debug)]
pub struct Hatch {
    pub angle: f32,
    pub style: Style,
}

impl Hatch {
    pub fn along(style: Style) -> Self {
        Hatch { angle: 0.0, style }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_and_get_by_name() {
        let mut st = Style::default();
        assert!(st.set("spacing", 6.0));
        assert_eq!(st.get("spacing"), Some(6.0));
        assert!(st.set("seed", 7.0));
        assert_eq!(st.seed, 7);
        assert!(!st.set("nope", 1.0));
        assert_eq!(Style::PARAMS.len(), 34);
    }
}
