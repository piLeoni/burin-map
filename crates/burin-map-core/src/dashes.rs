//! Break streamlines into dashes whose length follows the input field.
//!
//! `dash_light` / `dash_dark` set the ink fraction at input 0 and 1.
//! `dash_drop` controls the flat-ground lottery (0 = off).

use crate::rng::Rng;
use crate::streamlines::Line;
use crate::style::Style;

/// Point at arc length `d` along `line`, given cumulative lengths `s`.
fn at(line: &Line, s: &[f32], d: f32) -> [f32; 2] {
    let i = s.partition_point(|&v| v <= d).clamp(1, s.len() - 1);
    let (s0, s1) = (s[i - 1], s[i]);
    let t = if s1 > s0 {
        ((d - s0) / (s1 - s0)).clamp(0.0, 1.0)
    } else {
        0.0
    };
    let (a, b) = (line[i - 1], line[i]);
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

/// The stretch of `line` between arc lengths `a` and `b`.
fn dash(line: &Line, s: &[f32], (a, b): (f32, f32)) -> Line {
    let mut dash = vec![at(line, s, a)];
    let lo = s.partition_point(|&v| v <= a);
    let hi = s.partition_point(|&v| v < b);
    dash.extend_from_slice(&line[lo..hi.max(lo)]);
    dash.push(at(line, s, b));
    dash
}

pub fn dash_lines(lines: &[Line], input: &[f32], w: usize, h: usize, st: &Style) -> Vec<Line> {
    dash_lines_by(lines, st, |p| {
        input[(p[1] as usize).min(h - 1) * w + (p[0] as usize).min(w - 1)]
    })
}

/// [`dash_lines`] with the input at each dash's midpoint given by `tone`.
pub fn dash_lines_by(lines: &[Line], st: &Style, tone: impl Fn([f32; 2]) -> f32) -> Vec<Line> {
    let mut rng = Rng::new(st.batch_seed.wrapping_add(0x9e37_79b9));
    let base = (st.dash_period * st.space) as f64;
    // Fully continuous: both ends solid and no drop lottery.
    let continuous =
        st.dash_light >= 1.0 - 1e-6 && st.dash_dark >= 1.0 - 1e-6 && st.dash_drop <= 1e-6;
    let mut out = Vec::new();
    for line in lines {
        if line.len() < 2 {
            continue;
        }
        if continuous {
            out.push(line.clone());
            continue;
        }
        let mut s = Vec::with_capacity(line.len());
        s.push(0.0f32);
        for k in 1..line.len() {
            let d = (line[k][0] - line[k - 1][0]).hypot(line[k][1] - line[k - 1][1]);
            s.push(s[k - 1] + d);
        }
        let total = *s.last().unwrap() as f64;
        // A dash that reaches the start of the next one is the same stroke on
        // paper, so it is extended rather than emitted; only a real gap ends it.
        let mut open: Option<(f32, f32)> = None;
        let mut cur = -rng.f64() * base;
        while cur < total {
            let jit = st.mod_jitter as f64;
            let period = (base * rng.uniform(1.0 - jit, 1.0 + jit)).max(0.3 * base);
            let mid = (cur + period * 0.5).min(total);
            let p = at(line, &s, mid as f32);
            let mut t = tone(p);
            // Flat/light lottery: with strength dash_drop, randomly drop or bump to flat.
            if st.dash_drop > 1e-6
                && t > st.dash_cutoff
                && t < st.input_flat
                && rng.f64() < st.dash_drop as f64
            {
                t = if rng.f64() * (st.input_flat as f64) < t as f64 {
                    st.input_flat
                } else {
                    0.0
                };
            }
            if t > st.dash_cutoff {
                let duty = (st.dash_light + (st.dash_dark - st.dash_light) * t) as f64;
                let j = st.mod_jitter as f64 * (1.0 - t as f64);
                let spread = if j > 1e-3 {
                    rng.gamma(1.0 / (j * j), j * j)
                } else {
                    1.0
                };
                let a = cur.max(0.0);
                let mut b = cur + period * duty * spread;
                let break_p = st.mod_breaks as f64;
                if b >= cur + period && rng.f64() < break_p {
                    // must beat the round caps (stroke/2 each side) to show
                    b = cur + period - st.batch_stroke as f64 * rng.uniform(1.5, 3.0);
                }
                let b = b.min(total);
                if b - a > 0.6 {
                    let (a, b) = (a as f32, b as f32);
                    open = match open {
                        Some((a0, b0)) if a <= b0 => Some((a0, b0.max(b))),
                        Some(done) => {
                            out.push(dash(line, &s, done));
                            Some((a, b))
                        }
                        None => Some((a, b)),
                    };
                }
            }
            cur += period;
        }
        if let Some(done) = open {
            out.push(dash(line, &s, done));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A straight line in full shadow: every dash fills its whole period, so
    /// each one ends exactly where the next begins.
    fn in_shadow(mod_breaks: f32) -> Vec<Line> {
        let line: Line = (0..=100).map(|x| [x as f32 + 2.0, 5.0]).collect();
        let (w, h) = (110, 10);
        let st = Style {
            dash_light: 0.5,
            dash_drop: 0.0,
            mod_jitter: 0.0,
            mod_breaks,
            ..Style::default()
        };
        dash_lines(&[line], &vec![1.0; w * h], w, h, &st)
    }

    #[test]
    fn touching_dashes_are_one_stroke() {
        let out = in_shadow(0.0);
        assert_eq!(out.len(), 1, "{} strokes", out.len());
        assert_eq!(out[0][0], [2.0, 5.0]);
        assert_eq!(*out[0].last().unwrap(), [102.0, 5.0]);
    }

    #[test]
    fn real_gaps_still_break_the_stroke() {
        assert!(in_shadow(1.0).len() > 1);
    }
}
