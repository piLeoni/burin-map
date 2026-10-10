//! Break streamlines into dashes whose length follows the tone.
//!
//! Each `dash_period` px of line is one period, inked from its start for the
//! share [`Style::duty`] gives at the tone there. With every modifier off,
//! `dash_light` = `dash_dark` = 1 leaves each line whole and 0.5 gives even
//! dashes. See `docs/guide/03-ink.md` for the order the rules apply in.

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

/// Inked share of one period at tone `t`, 0 to 1, with its modifiers.
fn ink(st: &Style, t: f32, jit: f64, rng: &mut Rng) -> f64 {
    if t < st.dash_cutoff {
        return 0.0;
    }
    if t > st.dash_solid {
        return 1.0;
    }
    let mut t = t;
    if st.mod_flat > 1e-6 && t < st.input_flat && rng.f64() < st.mod_flat as f64 {
        // The lighter the ground, the likelier it is left out.
        if rng.f64() * (st.input_flat as f64) < t as f64 {
            t = st.input_flat;
        } else {
            return 0.0;
        }
    }
    let duty = st.duty(st.quantize(t)) as f64;
    let room = duty.min(1.0 - duty);
    if jit > 0.0 && room > 0.0 {
        duty + jit * room * rng.uniform(-1.0, 1.0)
    } else {
        duty
    }
}

pub fn dash_lines(lines: &[Line], input: &[f32], w: usize, h: usize, st: &Style) -> Vec<Line> {
    dash_lines_by(lines, st, |p| {
        input[(p[1] as usize).min(h - 1) * w + (p[0] as usize).min(w - 1)]
    })
}

/// [`dash_lines`] with the input at each dash's midpoint given by `tone`.
pub fn dash_lines_by(lines: &[Line], st: &Style, tone: impl Fn([f32; 2]) -> f32) -> Vec<Line> {
    let mut rng = Rng::new(st.batch_seed.wrapping_add(0x9e37_79b9));
    let base = st.dash_period.max(1.0) as f64;
    let solid = |v: f32| v >= 1.0 - 1e-6;
    let continuous = solid(st.dash_light)
        && solid(st.dash_dark)
        && (st.dash_mid < 0.0 || solid(st.dash_mid))
        && st.dash_cutoff <= 0.0
        && st.mod_flat <= 1e-6
        && st.mod_breaks <= 1e-6;
    let jit = st.mod_jitter.clamp(0.0, 1.0) as f64;
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
            let period = base * rng.uniform(1.0 - jit * 0.5, 1.0 + jit * 0.5);
            let mid = (cur + period * 0.5).min(total);
            let t = tone(at(line, &s, mid as f32));
            let share = ink(st, t, jit, &mut rng);
            if share > 0.0 {
                let a = cur.max(0.0);
                let mut b = cur + period * share;
                if share >= 1.0 && rng.f64() < st.mod_breaks as f64 && t <= st.dash_solid {
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

    const W: usize = 1010;

    fn line() -> Line {
        (0..=1000).map(|x| [x as f32 + 2.0, 5.0]).collect()
    }

    fn dashed(tone: f32, st: Style) -> Vec<Line> {
        dash_lines(&[line()], &vec![tone; W * 10], W, 10, &st)
    }

    fn inked(lines: &[Line]) -> f32 {
        lines
            .iter()
            .map(|l| l.windows(2).map(|p| (p[1][0] - p[0][0]).abs()).sum::<f32>())
            .sum::<f32>()
            / 1000.0
    }

    /// A straight line in full shadow: every dash fills its whole period, so
    /// each one ends exactly where the next begins.
    fn in_shadow(mod_breaks: f32) -> Vec<Line> {
        let st = Style {
            dash_light: 0.5,
            mod_breaks,
            ..Style::default()
        };
        dashed(1.0, st)
    }

    #[test]
    fn touching_dashes_are_one_stroke() {
        let out = in_shadow(0.0);
        assert_eq!(out.len(), 1, "{} strokes", out.len());
        assert_eq!(out[0][0], [2.0, 5.0]);
        assert_eq!(*out[0].last().unwrap(), [1002.0, 5.0]);
    }

    #[test]
    fn real_gaps_still_break_the_stroke() {
        assert!(in_shadow(1.0).len() > 1);
    }

    #[test]
    fn full_ink_everywhere_is_one_line_with_jitter_too() {
        for mod_jitter in [0.0, 1.0] {
            let st = Style {
                dash_light: 1.0,
                dash_dark: 1.0,
                mod_jitter,
                dash_period: 7.0,
                ..Style::default()
            };
            for tone in [0.0, 0.3, 1.0] {
                assert_eq!(dashed(tone, st.clone()).len(), 1);
            }
        }
    }

    #[test]
    fn half_ink_is_even_dashes() {
        let st = Style {
            dash_light: 0.5,
            dash_period: 10.0,
            ..Style::default()
        };
        let out = dashed(0.0, st);
        assert!((inked(&out) - 0.5).abs() < 0.02, "{}", inked(&out));
        for d in &out[1..out.len() - 1] {
            assert!((d.last().unwrap()[0] - d[0][0] - 5.0).abs() < 1e-3);
        }
    }

    #[test]
    fn jitter_keeps_the_average_ink() {
        for share in [0.2, 0.5, 0.8] {
            let st = Style {
                dash_light: share,
                dash_period: 6.0,
                mod_jitter: 1.0,
                ..Style::default()
            };
            let got = inked(&dashed(0.0, st));
            assert!((got - share).abs() < 0.06, "{share}: {got}");
        }
    }

    #[test]
    fn thresholds_win_over_the_ink() {
        let st = Style {
            dash_light: 0.3,
            dash_dark: 0.3,
            dash_cutoff: 0.2,
            dash_solid: 0.6,
            mod_breaks: 1.0,
            ..Style::default()
        };
        assert!(dashed(0.1, st.clone()).is_empty());
        assert_eq!(dashed(0.8, st.clone()).len(), 1);
        assert!((inked(&dashed(0.4, st)) - 0.3).abs() < 0.05);
    }

    #[test]
    fn levels_give_flat_zones() {
        let st = Style {
            dash_levels: 3,
            dash_light: 0.1,
            dash_mid: 0.5,
            dash_dark: 1.0,
            ..Style::default()
        };
        assert!((st.duty(st.quantize(0.2)) - 0.1).abs() < 1e-6);
        assert!((st.duty(st.quantize(0.5)) - 0.5).abs() < 1e-6);
        assert!((st.duty(st.quantize(0.6)) - 0.5).abs() < 1e-6);
        assert!((st.duty(st.quantize(0.9)) - 1.0).abs() < 1e-6);
    }
}
