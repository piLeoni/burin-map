//! Small colour expressions: `#hex`, `lerp(#a, #b, t)`, and scalars built
//! from `tone`, `elev`, `dist`, numbers, `clamp`, and `+ - * /`.

use std::collections::HashMap;

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

/// Split `paths` into pen runs by evaluated colour, or one run when fixed.
pub fn colorize_paths(
    paths: Vec<Line>,
    stroke: f32,
    layer: Option<String>,
    color: Option<&str>,
    color_expr: Option<&str>,
    ctx: &ColorCtx<'_>,
) -> Vec<(Pen, Vec<Line>)> {
    if paths.is_empty() {
        return Vec::new();
    }
    if !needs_per_path(color, color_expr) {
        return vec![(
            Pen::with_expr(stroke, color, color_expr, layer.as_deref()),
            paths,
        )];
    }
    let expr = match parse_color(color_expr.unwrap_or("")) {
        Ok(e) => e,
        Err(()) => {
            return vec![(Pen::with_expr(stroke, color, color_expr, layer.as_deref()), paths)];
        }
    };
    let mut by_hex: HashMap<String, Vec<Line>> = HashMap::new();
    for l in paths {
        let p = path_midpoint(&l);
        let sc = scalar_ctx(ctx, p);
        let rgb = eval_color(&expr, &sc);
        let hex = rgb_hex(rgb);
        by_hex.entry(hex).or_default().push(l);
    }
    let mut out: Vec<_> = by_hex
        .into_iter()
        .map(|(hex, ps)| {
            (
                Pen {
                    stroke,
                    color: hex,
                    layer: layer.clone(),
                },
                ps,
            )
        })
        .collect();
    out.sort_by(|a, b| a.0.color.cmp(&b.0.color));
    out
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
        if start == self.i {
            return None;
        }
        self.s[start..self.i].parse().ok()
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

    fn scalar_rest(&mut self, left: Scalar) -> Result<Scalar, ()> {
        self.skip_ws();
        match self.peek() {
            Some('+') => {
                self.bump();
                Ok(Scalar::Add(
                    Box::new(left),
                    Box::new(self.scalar_term()?),
                ))
            }
            Some('-') => {
                self.bump();
                Ok(Scalar::Sub(
                    Box::new(left),
                    Box::new(self.scalar_term()?),
                ))
            }
            _ => Ok(left),
        }
    }

    fn scalar_term(&mut self) -> Result<Scalar, ()> {
        let left = self.scalar_factor()?;
        self.skip_ws();
        match self.peek() {
            Some('*') => {
                self.bump();
                Ok(Scalar::Mul(
                    Box::new(left),
                    Box::new(self.scalar_factor()?),
                ))
            }
            Some('/') => {
                self.bump();
                Ok(Scalar::Div(
                    Box::new(left),
                    Box::new(self.scalar_factor()?),
                ))
            }
            _ => Ok(left),
        }
    }

    fn scalar_factor(&mut self) -> Result<Scalar, ()> {
        self.skip_ws();
        if let Some(n) = self.number() {
            return Ok(Scalar::Lit(n));
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
            self.skip_ws();
            if self.bump() != Some('(') {
                return Err(());
            }
            let x = Box::new(self.scalar()?);
            self.skip_ws();
            if self.bump() != Some(',') {
                return Err(());
            }
            let lo = Box::new(self.scalar()?);
            self.skip_ws();
            if self.bump() != Some(',') {
                return Err(());
            }
            let hi = Box::new(self.scalar()?);
            self.skip_ws();
            if self.bump() != Some(')') {
                return Err(());
            }
            return Ok(Scalar::Clamp(x, lo, hi));
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
        Scalar::Clamp(x, lo, hi) => eval_scalar(x, ctx).clamp(eval_scalar(lo, ctx), eval_scalar(hi, ctx)),
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
        let rgb = eval_color(&c, &EvalCtx { tone: 0.0, elev: 0.0, dist: 0.0, depth: 0.0 });
        assert!((rgb[0] - 0.5).abs() < 0.02);
    }

    #[test]
    fn tone_drives_land() {
        let c = parse_color("lerp(#000000, #ffffff, tone)").unwrap();
        let dark = eval_color(&c, &EvalCtx { tone: 0.0, elev: 0.0, dist: 0.0, depth: 0.0 });
        let light = eval_color(&c, &EvalCtx { tone: 1.0, elev: 0.0, dist: 0.0, depth: 0.0 });
        assert!(light[0] > dark[0] + 0.5);
    }

    #[test]
    fn dist_over_reach() {
        let c = parse_color("lerp(#ffffff, #000000, clamp(dist/100, 0, 1))").unwrap();
        let near = eval_color(&c, &EvalCtx { tone: 0.0, elev: 0.0, dist: 0.0, depth: 0.0 });
        let far = eval_color(&c, &EvalCtx { tone: 0.0, elev: 0.0, dist: 200.0, depth: 0.0 });
        assert!(near[0] > far[0] + 0.5);
    }
}
