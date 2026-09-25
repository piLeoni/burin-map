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
    spacing: f32 = 4.8, "distance between neighbouring lines, px";
    spacing_steep: f32 = 0.6, "spacing multiplier on the steepest ground (1 = constant)";
    spacing_dark: f32 = 0.7, "spacing multiplier in the deepest shadow (1 = constant)";
    spacing_gamma: f32 = 1.0, "curve of the steepness → spacing mapping";
    test_ratio: f32 = 0.5, "a line stops when closer than spacing × this to another";
    fill_ratio: f32 = 0.8, "gaps wider than 2 × this × spacing get a line in the middle (1 = off)";
    step: f32 = 1.0, "integration step, px";
    min_len: f32 = 4.0, "drop lines shorter than this, px";

    // Smoothing of the terrain (gaussian sigma, px)
    flow_blur: f32 = 10.0, "smoothing of the surface lines follow: higher = broader curves";
    shade_blur: f32 = 1.8, "smoothing for detail shading";
    broad_blur: f32 = 11.5, "smoothing for massif-scale shading";
    broad_weight: f32 = 0.5, "mix of massif-scale shading (0 = detail only, 1 = massif only)";
    wobble: f32 = 20.0, "random swing of line direction on flat ground, degrees";
    wobble_size: f32 = 12.0, "size of the wobble's bends, px";

    // Light
    azimuth: f32 = 315.0, "light direction, degrees clockwise from north";
    altitude: f32 = 45.0, "light elevation above the horizon, degrees";
    z_exag: f32 = 4.0, "vertical exaggeration used for shading";

    // Tone: 0 = nothing drawn, 1 = solid line
    flat_level: f32 = 0.25, "tone of flat ground; keeps a minimum stipple everywhere";
    shade_gain: f32 = 3.0, "how quickly slopes saturate to solid (shadow) or white (light)";
    dark_gamma: f32 = 0.7, "<1 pushes shadowed slopes towards solid";
    lit_gain: f32 = 1.5, ">1 makes lit slopes reach full light sooner";
    lit_floor: f32 = 0.06, "tone kept in full light: a few scattered dashes hint at the form";

    // Dashes
    period: f32 = 2.2, "dash + gap length, in multiples of spacing";
    duty_min: f32 = 0.15, "dash fraction of the period at tone 0";
    duty_max: f32 = 1.0, "dash fraction of the period at tone 1";
    cutoff: f32 = 0.03, "below this tone nothing is drawn";
    jitter: f32 = 0.45, "irregularity of dash length and pitch (0 = mechanical); fades in solid shadow";
    breaks: f32 = 0.15, "chance per period of a small random break in otherwise solid line";

    // Output
    stroke: f32 = 1.2, "stroke width, px";
    seed: u64 = 1, "random seed (seed order, dash jitter)";
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
        assert_eq!(Style::PARAMS.len(), 30);
    }
}
