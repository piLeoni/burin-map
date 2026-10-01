//! Small colour expressions: `#hex`, `lerp(#a, #b, t)`, and scalars built
//! from `tone`, `elev`, `dist`, `depth`, numbers, `lerp`, `clamp`, and
//! `+ - * /`. A width expression is one such scalar, multiplying the stroke.

use std::collections::HashMap;

use crate::pen::{Cap, Ink};
use crate::Line;
use crate::Pen;

/// Grids to read at a point, px coordinates.
pub struct ColorCtx<'a> {
    pub w: usize,
    pub h: usize,
    pub elev: Option<&'a [f32]>,
    /// Shading 0…1 on land (`input`); distance tone on water.
    pub tone: Option<&'a [f32]>,
    pub dist: Option<&'a [f32]>,
    /// Metres below the surface on water; absent when depth was not needed.
    pub depth: Option<&'a [f32]>,
}

/// True when dashes need their own colour from `color_expr`.
pub fn needs_per_path(color: Option<&str>, color_expr: Option<&str>) -> bool {
    if color.is_some_and(|s| !s.is_empty()) {
        return false;
    }
    color_expr
        .filter(|s| !s.is_empty())
        .is_some_and(|e| parse_hex(e).is_none())
}

/// Whether `expr` parses as a width expression; empty counts as none.
pub fn check_width(expr: &str) -> Result<(), String> {
    if expr.trim().is_empty() {
        return Ok(());
    }
    parse_width(expr)
        .map(|_| ())
        .map_err(|()| format!("width_expr \"{expr}\" is not an expression of tone, elev, dist, depth, numbers, lerp, clamp and + - * /"))
}

/// Split `paths` into pen runs by evaluated colour and width, or one run when
/// both are fixed. Dashes whose width comes to nothing are left out.
pub fn colorize_paths(
    paths: Vec<Line>,
    ink: &Ink<'_>,
    ctx: &ColorCtx<'_>,
) -> Vec<(Pen, Vec<Line>)> {
    if paths.is_empty() {
        return Vec::new();
    }
    let mut base = Pen::with_expr(ink.stroke, ink.color, ink.color_expr, ink.layer);
    base.cap = ink.cap;
    let color = needs_per_path(ink.color, ink.color_expr)
        .then(|| parse_color(ink.color_expr.unwrap_or("")).ok())
        .flatten();
    let width = ink
        .width_expr
        .filter(|s| !s.trim().is_empty())
        .and_then(|s| parse_width(s).ok());
    if color.is_none() && width.is_none() {
        return vec![(base, inside(paths, ink.cap, ink.stroke))];
    }
    // Each line in short pieces, each with the colour and width where it sits.
    let pieced: Vec<Vec<(Line, Option<String>, f32)>> = paths
        .iter()
        .map(|l| {
            pieces(l, PIECE)
                .into_iter()
                .map(|p| {
                    let sc = scalar_ctx(ctx, path_midpoint(&p));
                    let hex = color.as_ref().map(|c| rgb_hex(eval_color(c, &sc)));
                    let k = width.as_ref().map_or(1.0, |w| eval_scalar(w, &sc).max(0.0));
                    (p, hex, k)
                })
                .collect()
        })
        .collect();
    let shown = |k: f32| ink.stroke * k >= 0.01;
    let steps = Steps::of(
        pieced.iter().flatten().map(|p| p.2).filter(|&k| shown(k)),
        ink.width_steps,
    );
    type Key = (String, u32, Option<u32>);
    let mut groups: HashMap<Key, Vec<Line>> = HashMap::new();
    for line in pieced {
        // Neighbouring pieces with the same pen join up again.
        let mut runs: Vec<(Key, f32, Line)> = Vec::new();
        let mut gap = true;
        for (p, hex, k) in line {
            if !shown(k) {
                gap = true;
                continue;
            }
            let (k, step) = steps.snap(k);
            // To the hundredth of a px: finer than any pen, and it keeps the
            // number of groups down.
            let stroke = (ink.stroke * k * 100.0).round() / 100.0;
            let key = (
                hex.unwrap_or_else(|| base.color.clone()),
                stroke.to_bits(),
                step,
            );
            match runs.last_mut() {
                Some((last, _, run)) if !gap && *last == key => run.extend_from_slice(&p[1..]),
                _ => runs.push((key, stroke, p)),
            }
            gap = false;
        }
        if ink.cap == Cap::RoundInside {
            // Only the line's own ends draw in; where two widths meet the
            // round caps overlap.
            if let Some((_, w, run)) = runs.first_mut() {
                *run = shorten(std::mem::take(run), *w * 0.5, 0.0);
            }
            if let Some((_, w, run)) = runs.last_mut() {
                *run = shorten(std::mem::take(run), 0.0, *w * 0.5);
            }
        }
        for (key, _, run) in runs {
            groups.entry(key).or_default().push(run);
        }
    }
    let mut out: Vec<(Pen, Vec<Line>)> = groups
        .into_iter()
        .map(|((color, stroke, step), ps)| {
            let stroke = f32::from_bits(stroke);
            let layer = match step {
                Some(s) => Some(format!(
                    "{}-{}",
                    ink.layer.filter(|l| !l.is_empty()).unwrap_or("width"),
                    s + 1
                )),
                None => base.layer.clone(),
            };
            let pen = Pen {
                stroke,
                color,
                layer,
                cap: ink.cap,
                fill: false,
            };
            (pen, ps)
        })
        .collect();
    out.sort_by(|a, b| {
        a.0.color
            .cmp(&b.0.color)
            .then(a.0.stroke.total_cmp(&b.0.stroke))
    });
    out
}

/// `width_steps` evenly spaced widths from the thinnest dash to the widest.
struct Steps {
    lo: f32,
    hi: f32,
    n: u32,
}

impl Steps {
    fn of(ks: impl Iterator<Item = f32>, n: u32) -> Steps {
        let (lo, hi) = ks.fold((f32::INFINITY, f32::NEG_INFINITY), |(lo, hi), k| {
            (lo.min(k), hi.max(k))
        });
        Steps { lo, hi, n }
    }

    /// The width to draw `k` at, and which step it is when there are steps.
    fn snap(&self, k: f32) -> (f32, Option<u32>) {
        if self.n < 2 {
            return (k, None);
        }
        if self.hi - self.lo < 1e-6 {
            return (k, Some(0));
        }
        let last = (self.n - 1) as f32;
        let s = ((k - self.lo) / (self.hi - self.lo) * last).round();
        (self.lo + (self.hi - self.lo) * s / last, Some(s as u32))
    }
}

/// For `round_inside`, each dash shortened by half `stroke` at both ends so
/// its round caps end where it did; a dash shorter than that becomes a dot.
fn inside(paths: Vec<Line>, cap: Cap, stroke: f32) -> Vec<Line> {
    if cap != Cap::RoundInside {
        return paths;
    }
    paths
        .into_iter()
        .filter(|l| !l.is_empty())
        .map(|l| shorten(l, stroke * 0.5, stroke * 0.5))
        .collect()
}

/// `l` without its first `a` px and its last `b`; a dot when that is all of it.
fn shorten(l: Line, a: f32, b: f32) -> Line {
    if l.is_empty() || (a <= 0.0 && b <= 0.0) {
        return l;
    }
    let len = length(&l);
    if len <= a + b {
        let m = path_midpoint(&l);
        return vec![m, m];
    }
    let s = point_at(&l, a);
    let e = point_at(&l, len - b);
    let mut out = vec![s.1];
    out.extend_from_slice(&l[s.0 + 1..=e.0]);
    out.push(e.1);
    out
}

fn length(l: &Line) -> f32 {
    l.windows(2)
        .map(|q| (q[1][0] - q[0][0]).hypot(q[1][1] - q[0][1]))
        .sum()
}

/// Lines longer than this are coloured and sized piece by piece, px.
const PIECE: f32 = 8.0;

/// `l` cut into pieces about `len` px long, each starting where the one
/// before ends; short lines stay whole.
fn pieces(l: &Line, len: f32) -> Vec<Line> {
    let total = length(l);
    if l.len() < 2 || total <= len * 1.5 {
        return vec![l.clone()];
    }
    let n = (total / len).round() as usize;
    let step = total / n as f32;
    let mut out = Vec::with_capacity(n);
    let mut cur = vec![l[0]];
    let (mut run, mut next) = (0.0, step);
    for q in l.windows(2) {
        let d = (q[1][0] - q[0][0]).hypot(q[1][1] - q[0][1]);
        while d > 0.0 && run + d >= next && out.len() + 1 < n {
            let t = (next - run) / d;
            let p = [
                q[0][0] + (q[1][0] - q[0][0]) * t,
                q[0][1] + (q[1][1] - q[0][1]) * t,
            ];
            cur.push(p);
            out.push(std::mem::replace(&mut cur, vec![p]));
            next += step;
        }
        cur.push(q[1]);
        run += d;
    }
    out.push(cur);
    out
}

/// The point `s` px along `l`, and the index of the vertex before it.
fn point_at(l: &Line, s: f32) -> (usize, [f32; 2]) {
    let mut run = 0.0;
    for (i, q) in l.windows(2).enumerate() {
        let d = (q[1][0] - q[0][0]).hypot(q[1][1] - q[0][1]);
        if run + d >= s && d > 0.0 {
            let t = (s - run) / d;
            return (
                i,
                [
                    q[0][0] + (q[1][0] - q[0][0]) * t,
                    q[0][1] + (q[1][1] - q[0][1]) * t,
                ],
            );
        }
        run += d;
    }
    (l.len().saturating_sub(2), *l.last().unwrap())
}

fn scalar_ctx(ctx: &ColorCtx<'_>, p: [f32; 2]) -> EvalCtx {
    EvalCtx {
        tone: ctx.tone.map(|g| sample(g, ctx.w, ctx.h, p)).unwrap_or(0.5),
        elev: ctx.elev.map(|g| sample(g, ctx.w, ctx.h, p)).unwrap_or(0.0),
        dist: ctx.dist.map(|g| sample(g, ctx.w, ctx.h, p)).unwrap_or(0.0),
        depth: ctx.depth.map(|g| sample(g, ctx.w, ctx.h, p)).unwrap_or(0.0),
    }
}

struct EvalCtx {
    tone: f32,
    elev: f32,
    dist: f32,
    depth: f32,
}

#[derive(Clone, Debug, PartialEq)]
enum Color {
    Hex([f32; 3]),
    Lerp(Box<Color>, Box<Color>, Box<Scalar>),
}

#[derive(Clone, Debug, PartialEq)]
enum Scalar {
    Lit(f32),
    Tone,
    Elev,
    Dist,
    Depth,
    Clamp(Box<Scalar>, Box<Scalar>, Box<Scalar>),
    Lerp(Box<Scalar>, Box<Scalar>, Box<Scalar>),
    Div(Box<Scalar>, Box<Scalar>),
    Mul(Box<Scalar>, Box<Scalar>),
    Add(Box<Scalar>, Box<Scalar>),
    Sub(Box<Scalar>, Box<Scalar>),
}

fn parse_color(s: &str) -> Result<Color, ()> {
    let mut p = Parser::new(s);
    let c = p.color()?;
    p.skip_ws();
    if p.more() {
        return Err(());
    }
    Ok(c)
}

fn parse_width(s: &str) -> Result<Scalar, ()> {
    let mut p = Parser::new(s);
    let k = p.scalar()?;
    p.skip_ws();
    if p.more() {
        return Err(());
    }
    Ok(k)
}

struct Parser<'a> {
    s: &'a str,
    i: usize,
}

impl<'a> Parser<'a> {
    fn new(s: &'a str) -> Self {
        Parser { s, i: 0 }
    }

    fn more(&self) -> bool {
        self.i < self.s.len()
    }

    fn peek(&self) -> Option<char> {
        self.s[self.i..].chars().next()
    }

    fn bump(&mut self) -> Option<char> {
        let c = self.peek()?;
        self.i += c.len_utf8();
        Some(c)
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.bump();
        }
    }

    fn ident(&mut self) -> Option<String> {
        self.skip_ws();
        let start = self.i;
        while matches!(self.peek(), Some(c) if c.is_ascii_alphanumeric() || c == '_') {
            self.bump();
        }
        if start == self.i {
            return None;
        }
        Some(self.s[start..self.i].to_ascii_lowercase())
    }

    fn number(&mut self) -> Option<f32> {
        self.skip_ws();
        let start = self.i;
        if self.peek() == Some('-') {
            self.bump();
        }
        while matches!(self.peek(), Some(c) if c.is_ascii_digit() || c == '.') {
            self.bump();
        }
        let n = self.s[start..self.i].parse().ok();
        if n.is_none() {
            self.i = start;
        }
        n
    }

    fn hex_color(&mut self) -> Option<[f32; 3]> {
        self.skip_ws();
        if self.peek() != Some('#') {
            return None;
        }
        self.bump();
        let start = self.i;
        while matches!(self.peek(), Some(c) if c.is_ascii_hexdigit()) {
            self.bump();
        }
        let h = &self.s[start..self.i];
        parse_hex(&format!("#{h}")).map(|_| hex_rgb(h))
    }

    fn color(&mut self) -> Result<Color, ()> {
        self.skip_ws();
        if let Some(rgb) = self.hex_color() {
            return Ok(Color::Hex(rgb));
        }
        let id = self.ident().ok_or(())?;
        if id != "lerp" {
            return Err(());
        }
        self.skip_ws();
        if self.bump() != Some('(') {
            return Err(());
        }
        let a = Box::new(self.color()?);
        self.skip_ws();
        if self.bump() != Some(',') {
            return Err(());
        }
        let b = Box::new(self.color()?);
        self.skip_ws();
        if self.bump() != Some(',') {
            return Err(());
        }
        let t = Box::new(self.scalar()?);
        self.skip_ws();
        if self.bump() != Some(')') {
            return Err(());
        }
        Ok(Color::Lerp(a, b, t))
    }

    fn scalar(&mut self) -> Result<Scalar, ()> {
        let left = self.scalar_term()?;
        self.scalar_rest(left)
    }

    fn scalar_rest(&mut self, mut left: Scalar) -> Result<Scalar, ()> {
        loop {
            self.skip_ws();
            left = match self.peek() {
                Some('+') => {
                    self.bump();
                    Scalar::Add(Box::new(left), Box::new(self.scalar_term()?))
                }
                Some('-') => {
                    self.bump();
                    Scalar::Sub(Box::new(left), Box::new(self.scalar_term()?))
                }
                _ => return Ok(left),
            };
        }
    }

    fn scalar_term(&mut self) -> Result<Scalar, ()> {
        let mut left = self.scalar_factor()?;
        loop {
            self.skip_ws();
            left = match self.peek() {
                Some('*') => {
                    self.bump();
                    Scalar::Mul(Box::new(left), Box::new(self.scalar_factor()?))
                }
                Some('/') => {
                    self.bump();
                    Scalar::Div(Box::new(left), Box::new(self.scalar_factor()?))
                }
                _ => return Ok(left),
            };
        }
    }

    /// `(a, b, c)` after a function name.
    fn three(&mut self) -> Result<[Box<Scalar>; 3], ()> {
        self.skip_ws();
        if self.bump() != Some('(') {
            return Err(());
        }
        let a = Box::new(self.scalar()?);
        self.skip_ws();
        if self.bump() != Some(',') {
            return Err(());
        }
        let b = Box::new(self.scalar()?);
        self.skip_ws();
        if self.bump() != Some(',') {
            return Err(());
        }
        let c = Box::new(self.scalar()?);
        self.skip_ws();
        if self.bump() != Some(')') {
            return Err(());
        }
        Ok([a, b, c])
    }

    fn scalar_factor(&mut self) -> Result<Scalar, ()> {
        self.skip_ws();
        if let Some(n) = self.number() {
            return Ok(Scalar::Lit(n));
        }
        if self.peek() == Some('-') {
            self.bump();
            let f = self.scalar_factor()?;
            return Ok(Scalar::Sub(Box::new(Scalar::Lit(0.0)), Box::new(f)));
        }
        if self.peek() == Some('(') {
            self.bump();
            let inner = self.scalar()?;
            self.skip_ws();
            if self.bump() != Some(')') {
                return Err(());
            }
            return Ok(inner);
        }
        let id = self.ident().ok_or(())?;
        if id == "clamp" {
            let [x, lo, hi] = self.three()?;
            return Ok(Scalar::Clamp(x, lo, hi));
        }
        if id == "lerp" {
            let [a, b, t] = self.three()?;
            return Ok(Scalar::Lerp(a, b, t));
        }
        Ok(match id.as_str() {
            "tone" | "input" => Scalar::Tone,
            "elev" | "elevation" => Scalar::Elev,
            "dist" | "distance" => Scalar::Dist,
            "depth" => Scalar::Depth,
            _ => return Err(()),
        })
    }
}

fn eval_color(c: &Color, ctx: &EvalCtx) -> [f32; 3] {
    match c {
        Color::Hex(rgb) => *rgb,
        Color::Lerp(a, b, t) => {
            let (a, b) = (eval_color(a, ctx), eval_color(b, ctx));
            let t = eval_scalar(t, ctx).clamp(0.0, 1.0);
            [
                a[0] + (b[0] - a[0]) * t,
                a[1] + (b[1] - a[1]) * t,
                a[2] + (b[2] - a[2]) * t,
            ]
        }
    }
}

fn eval_scalar(s: &Scalar, ctx: &EvalCtx) -> f32 {
    match s {
        Scalar::Lit(v) => *v,
        Scalar::Tone => ctx.tone,
        Scalar::Elev => ctx.elev,
        Scalar::Dist => ctx.dist,
        Scalar::Depth => ctx.depth,
        Scalar::Clamp(x, lo, hi) => {
            let (lo, hi) = (eval_scalar(lo, ctx), eval_scalar(hi, ctx));
            eval_scalar(x, ctx).clamp(lo.min(hi), lo.max(hi))
        }
        Scalar::Lerp(a, b, t) => {
            let (a, b) = (eval_scalar(a, ctx), eval_scalar(b, ctx));
            a + (b - a) * eval_scalar(t, ctx).clamp(0.0, 1.0)
        }
        Scalar::Div(a, b) => {
            let d = eval_scalar(b, ctx);
            if d.abs() < 1e-6 {
                0.0
            } else {
                eval_scalar(a, ctx) / d
            }
        }
        Scalar::Mul(a, b) => eval_scalar(a, ctx) * eval_scalar(b, ctx),
        Scalar::Add(a, b) => eval_scalar(a, ctx) + eval_scalar(b, ctx),
        Scalar::Sub(a, b) => eval_scalar(a, ctx) - eval_scalar(b, ctx),
    }
}

pub fn parse_hex(s: &str) -> Option<String> {
    let t = s.trim();
    if !t.starts_with('#') {
        return None;
    }
    let h = &t[1..];
    if !matches!(h.len(), 3 | 6) || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(t.to_string())
}

fn hex_rgb(h: &str) -> [f32; 3] {
    let expand = |c: char| {
        let v = u8::from_str_radix(&c.to_string().repeat(2), 16).unwrap_or(0);
        v as f32 / 255.0
    };
    if h.len() == 3 {
        let mut cs = h.chars();
        return [
            expand(cs.next().unwrap()),
            expand(cs.next().unwrap()),
            expand(cs.next().unwrap()),
        ];
    }
    let parse2 = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0) as f32 / 255.0;
    [parse2(0), parse2(2), parse2(4)]
}

fn rgb_hex(rgb: [f32; 3]) -> String {
    let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", b(rgb[0]), b(rgb[1]), b(rgb[2]))
}

fn sample(grid: &[f32], w: usize, h: usize, p: [f32; 2]) -> f32 {
    if w == 0 || h == 0 || grid.is_empty() {
        return 0.0;
    }
    let x = p[0].clamp(0.0, (w - 1) as f32);
    let y = p[1].clamp(0.0, (h - 1) as f32);
    let (i, j) = (x.floor() as usize, y.floor() as usize);
    let (i1, j1) = ((i + 1).min(w - 1), (j + 1).min(h - 1));
    let (fx, fy) = (x - i as f32, y - j as f32);
    let at = |x: usize, y: usize| grid[y * w + x];
    let top = at(i, j) + (at(i1, j) - at(i, j)) * fx;
    let bot = at(i, j1) + (at(i1, j1) - at(i, j1)) * fx;
    top + (bot - top) * fy
}

fn path_midpoint(l: &Line) -> [f32; 2] {
    if l.is_empty() {
        return [0.0, 0.0];
    }
    if l.len() == 1 {
        return l[0];
    }
    let mut s = vec![0.0f32];
    for q in l.windows(2) {
        s.push(s.last().unwrap() + (q[1][0] - q[0][0]).hypot(q[1][1] - q[0][1]));
    }
    let half = s.last().unwrap() * 0.5;
    for i in 1..l.len() {
        if s[i] >= half {
            let t = if s[i] > s[i - 1] {
                (half - s[i - 1]) / (s[i] - s[i - 1])
            } else {
                0.0
            };
            return [
                l[i - 1][0] + (l[i][0] - l[i - 1][0]) * t,
                l[i - 1][1] + (l[i][1] - l[i - 1][1]) * t,
            ];
        }
    }
    *l.last().unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lerp_mid_grey() {
        let c = parse_color("lerp(#000000, #ffffff, 0.5)").unwrap();
        let rgb = eval_color(
            &c,
            &EvalCtx {
                tone: 0.0,
                elev: 0.0,
                dist: 0.0,
                depth: 0.0,
            },
        );
        assert!((rgb[0] - 0.5).abs() < 0.02);
    }

    #[test]
    fn tone_drives_land() {
        let c = parse_color("lerp(#000000, #ffffff, tone)").unwrap();
        let dark = eval_color(
            &c,
            &EvalCtx {
                tone: 0.0,
                elev: 0.0,
                dist: 0.0,
                depth: 0.0,
            },
        );
        let light = eval_color(
            &c,
            &EvalCtx {
                tone: 1.0,
                elev: 0.0,
                dist: 0.0,
                depth: 0.0,
            },
        );
        assert!(light[0] > dark[0] + 0.5);
    }

    #[test]
    fn dist_over_reach() {
        let c = parse_color("lerp(#ffffff, #000000, clamp(dist/100, 0, 1))").unwrap();
        let near = eval_color(
            &c,
            &EvalCtx {
                tone: 0.0,
                elev: 0.0,
                dist: 0.0,
                depth: 0.0,
            },
        );
        let far = eval_color(
            &c,
            &EvalCtx {
                tone: 0.0,
                elev: 0.0,
                dist: 200.0,
                depth: 0.0,
            },
        );
        assert!(near[0] > far[0] + 0.5);
    }

    fn at(elev: f32) -> EvalCtx {
        EvalCtx {
            tone: 0.0,
            elev,
            dist: 0.0,
            depth: 0.0,
        }
    }

    #[test]
    fn scalars_chain_and_negate() {
        let k = |s: &str, e: f32| eval_scalar(&parse_width(s).unwrap(), &at(e));
        assert_eq!(k("10 - 3 - 2", 0.0), 5.0);
        assert_eq!(k("12 / 3 / 2", 0.0), 2.0);
        assert_eq!(k("-elev", 4.0), -4.0);
        assert_eq!(k("lerp(2, 0.5, elev / 100)", 50.0), 1.25);
        assert_eq!(k("clamp(elev, 1, 0)", 3.0), 1.0);
        assert!(parse_width("lerp(#000, #fff, 1)").is_err());
        assert!(check_width("").is_ok() && check_width("elev +").is_err());
    }

    /// A ramp of elevation 0…99 along x, one row.
    fn ramp() -> Vec<f32> {
        (0..100).map(|x| x as f32).collect()
    }

    fn dash(x: f32) -> Line {
        vec![[x, 0.0], [x + 0.5, 0.0]]
    }

    #[test]
    fn widths_follow_the_expression() {
        let elev = ramp();
        let ctx = ColorCtx {
            w: 100,
            h: 1,
            elev: Some(&elev),
            tone: None,
            dist: None,
            depth: None,
        };
        let ink = Ink {
            stroke: 1.0,
            width_expr: Some("elev / 10 - 0.5"),
            ..Ink::default()
        };
        let paths: Vec<Line> = (0..10).map(|i| dash(i as f32 * 10.0)).collect();
        let pens = colorize_paths(paths.clone(), &ink, &ctx);
        // The dash at elevation ~0 has no width and is left out.
        assert_eq!(pens.len(), 9);
        assert!(pens.windows(2).all(|p| p[0].0.stroke < p[1].0.stroke));

        let pens = colorize_paths(
            paths,
            &Ink {
                width_steps: 3,
                layer: Some("hills"),
                ..ink
            },
            &ctx,
        );
        let layers: Vec<_> = pens.iter().map(|p| p.0.layer.clone().unwrap()).collect();
        assert_eq!(layers, ["hills-1", "hills-2", "hills-3"]);
        assert_eq!(pens.iter().map(|p| p.1.len()).sum::<usize>(), 9);
    }

    #[test]
    fn a_long_line_changes_width_along_its_way() {
        let elev = ramp();
        let ctx = ColorCtx {
            w: 100,
            h: 1,
            elev: Some(&elev),
            tone: None,
            dist: None,
            depth: None,
        };
        let ink = Ink {
            stroke: 1.0,
            width_expr: Some("elev / 10 + 1"),
            width_steps: 3,
            ..Ink::default()
        };
        let line: Line = (0..100).map(|x| [x as f32, 0.0]).collect();
        let pens = colorize_paths(vec![line], &ink, &ctx);
        assert_eq!(pens.len(), 3);
        assert!(pens.iter().all(|p| p.1.len() == 1), "one run per width");
        let total: f32 = pens.iter().map(|p| length(&p.1[0])).sum();
        assert!((total - 99.0).abs() < 1e-3, "{total}");
        assert!(pens[0].1[0][0][0] < 1.0 && pens[2].1[0].last().unwrap()[0] > 98.0);
    }

    #[test]
    fn round_inside_keeps_within_the_ends() {
        let l = shorten(vec![[0.0, 0.0], [4.0, 0.0], [10.0, 0.0]], 1.0, 1.0);
        assert_eq!(l, vec![[1.0, 0.0], [4.0, 0.0], [9.0, 0.0]]);
        assert_eq!(
            shorten(vec![[0.0, 0.0], [1.0, 0.0]], 1.0, 1.0),
            vec![[0.5, 0.0], [0.5, 0.0]]
        );
        let ink = Ink {
            stroke: 2.0,
            cap: Cap::RoundInside,
            ..Ink::default()
        };
        let ctx = ColorCtx {
            w: 1,
            h: 1,
            elev: None,
            tone: None,
            dist: None,
            depth: None,
        };
        let pens = colorize_paths(vec![vec![[0.0, 0.0], [10.0, 0.0]]], &ink, &ctx);
        assert_eq!(pens[0].0.cap, Cap::RoundInside);
        assert_eq!(pens[0].1[0], vec![[1.0, 0.0], [9.0, 0.0]]);
    }
}
