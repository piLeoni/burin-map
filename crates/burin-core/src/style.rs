//! All tunable parameters of a drawing. Lengths are in output pixels, so the
//! same style gives the same look whatever the area and scale of the map.
//! Mirrors `src/burin/style.py`; see there for what each one does.

#[derive(Clone, Debug)]
pub struct Style {
    // Line layout
    pub spacing: f32,
    pub spacing_steep: f32,
    pub spacing_dark: f32,
    pub spacing_gamma: f32,
    pub test_ratio: f32,
    pub fill_ratio: f32,
    pub step: f32,
    pub min_len: f32,
    // Smoothing (gaussian sigma, px)
    pub flow_blur: f32,
    pub shade_blur: f32,
    pub broad_blur: f32,
    pub broad_weight: f32,
    pub wobble: f32,
    pub wobble_size: f32,
    // Light
    pub azimuth: f32,
    pub altitude: f32,
    pub z_exag: f32,
    // Tone
    pub flat_level: f32,
    pub shade_gain: f32,
    pub dark_gamma: f32,
    pub lit_gain: f32,
    pub lit_floor: f32,
    // Dashes
    pub period: f32,
    pub duty_min: f32,
    pub duty_max: f32,
    pub cutoff: f32,
    pub jitter: f32,
    pub breaks: f32,
    // Output
    pub stroke: f32,
    pub seed: u64,
}

impl Default for Style {
    fn default() -> Self {
        Style {
            spacing: 4.8,
            spacing_steep: 0.6,
            spacing_dark: 0.7,
            spacing_gamma: 1.0,
            test_ratio: 0.5,
            fill_ratio: 0.8,
            step: 1.0,
            min_len: 4.0,
            flow_blur: 10.0,
            shade_blur: 1.8,
            broad_blur: 11.5,
            broad_weight: 0.5,
            wobble: 20.0,
            wobble_size: 12.0,
            azimuth: 315.0,
            altitude: 45.0,
            z_exag: 4.0,
            flat_level: 0.25,
            shade_gain: 3.0,
            dark_gamma: 0.7,
            lit_gain: 1.5,
            lit_floor: 0.06,
            period: 2.2,
            duty_min: 0.15,
            duty_max: 1.0,
            cutoff: 0.03,
            jitter: 0.45,
            breaks: 0.15,
            stroke: 1.2,
            seed: 1,
        }
    }
}
