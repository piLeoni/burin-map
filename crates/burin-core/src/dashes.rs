//! Break streamlines into dashes whose length follows the tone.
//! Mirrors `src/burin/dashes.py`; see there for the full explanation.

use crate::rng::Rng;
use crate::streamlines::Line;
use crate::style::Style;

/// Point at arc length `d` along `line`, given cumulative lengths `s`.
fn at(line: &Line, s: &[f32], d: f32) -> [f32; 2] {
    let i = s.partition_point(|&v| v <= d).clamp(1, s.len() - 1);
    let (s0, s1) = (s[i - 1], s[i]);
    let t = if s1 > s0 { ((d - s0) / (s1 - s0)).clamp(0.0, 1.0) } else { 0.0 };
    let (a, b) = (line[i - 1], line[i]);
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

pub fn dash_lines(lines: &[Line], tone: &[f32], w: usize, h: usize, st: &Style) -> Vec<Line> {
    let mut rng = Rng::new(st.seed.wrapping_add(0x9e37_79b9));
    let base = (st.period * st.spacing) as f64;
    let mut out = Vec::new();
    for line in lines {
        if line.len() < 2 {
            continue;
        }
        let mut s = Vec::with_capacity(line.len());
        s.push(0.0f32);
        for k in 1..line.len() {
            let d = (line[k][0] - line[k - 1][0]).hypot(line[k][1] - line[k - 1][1]);
            s.push(s[k - 1] + d);
        }
        let total = *s.last().unwrap() as f64;
        let mut cur = -rng.f64() * base;
        while cur < total {
            let jit = st.jitter as f64;
            let period = (base * rng.uniform(1.0 - jit, 1.0 + jit)).max(0.3 * base);
            let mid = (cur + period * 0.5).min(total);
            let p = at(line, &s, mid as f32);
            let mut t = tone[(p[1] as usize).min(h - 1) * w + (p[0] as usize).min(w - 1)];
            // Below the flat tone, keep the flat dash length and drop dashes at random.
            if t > st.cutoff && t < st.flat_level {
                t = if rng.f64() * (st.flat_level as f64) < t as f64 { st.flat_level } else { 0.0 };
            }
            if t > st.cutoff {
                let duty = (st.duty_min + (st.duty_max - st.duty_min) * t) as f64;
                let j = st.jitter as f64 * (1.0 - t as f64);
                let spread = if j > 1e-3 { rng.gamma(1.0 / (j * j), j * j) } else { 1.0 };
                let a = cur.max(0.0);
                let mut b = cur + period * duty * spread;
                if b >= cur + period && rng.f64() < st.breaks as f64 {
                    // must beat the round caps (stroke/2 each side) to show
                    b = cur + period - st.stroke as f64 * rng.uniform(1.5, 3.0);
                }
                let b = b.min(total);
                if b - a > 0.6 {
                    let (a, b) = (a as f32, b as f32);
                    let mut dash = vec![at(line, &s, a)];
                    let lo = s.partition_point(|&v| v <= a);
                    let hi = s.partition_point(|&v| v < b);
                    dash.extend_from_slice(&line[lo..hi.max(lo)]);
                    dash.push(at(line, &s, b));
                    out.push(dash);
                }
            }
            cur += period;
        }
    }
    out
}
