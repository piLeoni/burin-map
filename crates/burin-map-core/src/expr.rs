//! Expressions: colours and scalars evaluated at a point on the map.
//!
//! One tree for every per-point value burin has — a batch's colour, its line
//! width, the shoreline's. Drawings are built from these trees by the typed
//! API (`Batch::color`, `Batch::width`), and the JavaScript front end
//! (`burin-map-script`) compiles `(p) => …` into the same tree, so the engine
//! never sees a string and never interprets one.
//!
//! A scalar is a number or one of the [`Field`]s the map provides, combined
//! with the arithmetic in [`Bin`], a choice, and the functions below. A colour
//! is a colour or a mix between two of them.

use crate::pen::{Cap, Ink, Pen};
use crate::Line;
use std::collections::HashMap;

/// The values a tree can read where a dash sits. Which ones are live depends
/// on the batch: heights and shading on land, distance and depth on water.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Field {
    /// Height at the point, metres. Land.
    Elev,
    /// Shading input, 0…1 (light…dark). Land, and a water batch's tone.
    Tone,
    /// Distance from the shore, px. Water.
    Dist,
    /// Metres below the surface. Water.
    Depth,
}

impl Field {
    /// The field an expression names, or `None` for anything else: the front
    /// ends and the depth pass both ask this way.
    pub fn named(name: &str) -> Option<Field> {
        Some(match name {
            "elev" => Field::Elev,
            "tone" => Field::Tone,
            "dist" => Field::Dist,
            "depth" => Field::Depth,
            _ => return None,
        })
    }

    /// The name it is written with.
    pub fn name(&self) -> &'static str {
        match self {
            Field::Elev => "elev",
            Field::Tone => "tone",
            Field::Dist => "dist",
            Field::Depth => "depth",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Un {
    Neg,
    Not,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Bin {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Pow,
    Min,
    Max,
    /// `a < b` → 1 or 0.
    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Ne,
    /// `a && b`, `a || b`.
    And,
    Or,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Func {
    Abs,
    Floor,
    Ceil,
    Round,
    Sqrt,
    Exp,
    Log,
    Sin,
    Cos,
    Tan,
    Sign,
    Clamp,
    Lerp,
    Smoothstep,
    Step,
}

impl Func {
    /// Arity.
    pub fn arity(&self) -> usize {
        match self {
            Func::Clamp | Func::Lerp | Func::Smoothstep => 3,
            Func::Step => 2,
            _ => 1,
        }
    }

    /// Name and arity of an expression function.
    pub fn named(name: &str) -> Option<Func> {
        Some(match name {
            "abs" => Func::Abs,
            "floor" => Func::Floor,
            "ceil" => Func::Ceil,
            "round" => Func::Round,
            "sqrt" => Func::Sqrt,
            "exp" => Func::Exp,
            "log" => Func::Log,
            "sin" => Func::Sin,
            "cos" => Func::Cos,
            "tan" => Func::Tan,
            "sign" => Func::Sign,
            "clamp" => Func::Clamp,
            "lerp" => Func::Lerp,
            "smoothstep" => Func::Smoothstep,
            "step" => Func::Step,
            _ => return None,
        })
    }

    pub fn name(&self) -> &'static str {
        match self {
            Func::Abs => "abs",
            Func::Floor => "floor",
            Func::Ceil => "ceil",
            Func::Round => "round",
            Func::Sqrt => "sqrt",
            Func::Exp => "exp",
            Func::Log => "log",
            Func::Sin => "sin",
            Func::Cos => "cos",
            Func::Tan => "tan",
            Func::Sign => "sign",
            Func::Clamp => "clamp",
            Func::Lerp => "lerp",
            Func::Smoothstep => "smoothstep",
            Func::Step => "step",
        }
    }
}

/// A number, or something computed from the map at the point.
#[derive(Clone, Debug, PartialEq)]
pub enum Scalar {
    Lit(f32),
    Field(Field),
    /// A colour's channel: `r`, `g`, `b` of `t` (0…1). NaN outside colour ops.
    Channel(Box<Color>, char),
    /// A comparison on colours, as the front ends allow.
    Cmp(Bin, Box<Expr>, Box<Expr>),
    Un(Un, Box<Scalar>),
    Bin(Bin, Box<Scalar>, Box<Scalar>),
    /// `c ? a : b`: `a` where `c` is true (non-zero).
    Choose(Box<Scalar>, Box<Scalar>, Box<Scalar>),
    Call(Func, Vec<Scalar>),
}

impl From<f32> for Scalar {
    fn from(v: f32) -> Scalar {
        Scalar::Lit(v)
    }
}

impl From<Field> for Scalar {
    fn from(f: Field) -> Scalar {
        Scalar::Field(f)
    }
}

/// What a dash is inked with: a colour, or a mix between two.
#[derive(Clone, Debug, PartialEq)]
pub enum Color {
    Lit([f32; 3]),
    Lerp(Box<Color>, Box<Color>, Box<Scalar>),
    /// Per-channel: `r`, `g`, `b` of a colour or a scalar, 0…1.
    Chan(Box<Color>, char),
    /// Two colours mixed, `t` decided once (the front ends' `mix(x, y, k)`).
    Mix(Box<Color>, Box<Color>, f32),
    /// A colour written as bytes, as a palette entry is.
    Rgb8(u8, u8, u8),
}

impl Color {
    pub fn hex(text: &str) -> Option<Color> {
        hex_rgb(text).map(Color::Lit)
    }

    pub fn rgb(r: f32, g: f32, b: f32) -> Color {
        Color::Lit([r, g, b])
    }

    pub fn gray(v: f32) -> Color {
        Color::Lit([v, v, v])
    }

    /// `a` mixed `t` of the way to `b`, `t` held to 0…1.
    pub fn mix(a: Color, b: Color, t: impl Into<Scalar>) -> Color {
        Color::lerp(a, b, t)
    }

    fn lerp(a: Color, b: Color, t: impl Into<Scalar>) -> Color {
        Color::Lerp(Box::new(a), Box::new(b), Box::new(t.into()))
    }

    pub fn white() -> Color {
        Color::gray(1.0)
    }

    pub fn black() -> Color {
        Color::gray(0.0)
    }

    /// White to black, through the shading input: `lerp(white, black, tone)`.
    pub fn tone() -> Color {
        Color::mix(Color::gray(1.0), Color::gray(0.0), Field::Tone)
    }
}

/// A tree to evaluate: a scalar or a colour. A scalar used as a colour is a
/// grey, so `width` and `color` can take the same expressions.
#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Scalar(Scalar),
    Color(Color),
    Shade(Scalar),
}

impl From<Scalar> for Expr {
    fn from(s: Scalar) -> Expr {
        Expr::Scalar(s)
    }
}

impl From<Color> for Expr {
    fn from(c: Color) -> Expr {
        Expr::Color(c)
    }
}

impl From<Field> for Expr {
    fn from(f: Field) -> Expr {
        Expr::Scalar(Scalar::Field(f))
    }
}

impl From<f32> for Expr {
    fn from(v: f32) -> Expr {
        Expr::Scalar(Scalar::Lit(v))
    }
}

impl Scalar {
    /// True when the scalar reads no field, so every dash gets the same value.
    pub fn constant_scalar(&self) -> bool {
        Expr::Scalar(self.clone()).constant()
    }
}

impl Expr {
    /// True when the tree reads `f`.
    pub fn uses(&self, f: Field) -> bool {
        fn scalar_uses(s: &Scalar, f: Field) -> bool {
            match s {
                Scalar::Lit(_) => false,
                Scalar::Field(x) => *x == f,
                Scalar::Channel(c, _) => color_uses(c, f),
                Scalar::Cmp(_, a, b) => a.uses(f) || b.uses(f),
                Scalar::Un(_, a) => scalar_uses(a, f),
                Scalar::Bin(_, a, b) => scalar_uses(a, f) || scalar_uses(b, f),
                Scalar::Choose(c, a, b) => {
                    scalar_uses(c, f) || scalar_uses(a, f) || scalar_uses(b, f)
                }
                Scalar::Call(_, args) => args.iter().any(|a| scalar_uses(a, f)),
            }
        }
        fn color_uses(c: &Color, f: Field) -> bool {
            match c {
                Color::Lit(_) | Color::Rgb8(..) => false,
                Color::Lerp(a, b, t) => color_uses(a, f) || color_uses(b, f) || scalar_uses(t, f),
                Color::Chan(c, _) => color_uses(c, f),
                Color::Mix(a, b, _) => color_uses(a, f) || color_uses(b, f),
            }
        }
        match self {
            Expr::Scalar(s) | Expr::Shade(s) => scalar_uses(s, f),
            Expr::Color(c) => color_uses(c, f),
        }
    }

    /// Whether the tree is constant, so every dash gets the same value.
    pub fn constant(&self) -> bool {
        [Field::Elev, Field::Tone, Field::Dist, Field::Depth]
            .into_iter()
            .all(|f| !self.uses(f))
    }
}

/// The grids a tree can read, and how big they are, px.
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

impl ColorCtx<'_> {
    /// What the tree reads `f` at `p`.
    fn at(&self, f: Field, p: [f32; 2]) -> f32 {
        let (grid, missing) = match f {
            Field::Elev => (self.elev, 0.0),
            Field::Tone => (self.tone, 0.5),
            Field::Dist => (self.dist, 0.0),
            Field::Depth => (self.depth, 0.0),
        };
        grid.map(|g| sample(g, self.w, self.h, p)).unwrap_or(missing)
    }
}

/// What a scalar came to at one point.
#[derive(Clone, Copy, Debug)]
struct Value(f32);

fn truthy(v: f32) -> bool {
    v != 0.0
}

fn eval_scalar(s: &Scalar, ctx: &ColorCtx<'_>, p: [f32; 2]) -> Value {
    Value(match s {
        Scalar::Lit(v) => *v,
        Scalar::Field(f) => ctx.at(*f, p),
        Scalar::Channel(c, ch) => {
            let rgb = eval_color(c, ctx, p);
            match ch {
                'r' => rgb[0],
                'g' => rgb[1],
                _ => rgb[2],
            }
        }
        Scalar::Cmp(op, a, b) => {
            let (a, b) = (eval_color_of(a, ctx, p), eval_color_of(b, ctx, p));
            let same = a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-4);
            match op {
                Bin::Eq => same as u8 as f32,
                Bin::Ne => !same as u8 as f32,
                Bin::Lt => (luma(a) < luma(b)) as u8 as f32,
                Bin::Gt => (luma(a) > luma(b)) as u8 as f32,
                Bin::Le => (luma(a) <= luma(b)) as u8 as f32,
                Bin::Ge => (luma(a) >= luma(b)) as u8 as f32,
                _ => 0.0,
            }
        }
        Scalar::Un(op, a) => {
            let a = eval_scalar(a, ctx, p).0;
            match op {
                Un::Neg => -a,
                Un::Not => !truthy(a) as u8 as f32,
            }
        }
        Scalar::Bin(op, a, b) => {
            let (a, b) = (eval_scalar(a, ctx, p).0, eval_scalar(b, ctx, p).0);
            eval_bin(*op, a, b)
        }
        Scalar::Choose(c, a, b) => {
            if truthy(eval_scalar(c, ctx, p).0) {
                eval_scalar(a, ctx, p).0
            } else {
                eval_scalar(b, ctx, p).0
            }
        }
        Scalar::Call(f, args) => {
            let a = |i: usize| eval_scalar(&args[i], ctx, p).0;
            match f {
                Func::Abs => a(0).abs(),
                Func::Floor => a(0).floor(),
                Func::Ceil => a(0).ceil(),
                Func::Round => a(0).round(),
                Func::Sqrt => a(0).max(0.0).sqrt(),
                Func::Exp => a(0).exp(),
                Func::Log => a(0).max(1e-9).ln(),
                Func::Sin => a(0).sin(),
                Func::Cos => a(0).cos(),
                Func::Tan => a(0).tan(),
                Func::Sign => a(0).signum(),
                Func::Clamp => {
                    let (lo, hi) = (a(1), a(2));
                    a(0).clamp(lo.min(hi), lo.max(hi))
                }
                Func::Lerp => {
                    let t = a(2).clamp(0.0, 1.0);
                    a(0) + (a(1) - a(0)) * t
                }
                Func::Smoothstep => {
                    let t = ((a(0) - a(1)) / (a(2) - a(1)).max(1e-9)).clamp(0.0, 1.0);
                    t * t * (3.0 - 2.0 * t)
                }
                Func::Step => (a(1) >= a(0)) as u8 as f32,
            }
        }
    })
}

fn eval_bin(op: Bin, a: f32, b: f32) -> f32 {
    match op {
        Bin::Add => a + b,
        Bin::Sub => a - b,
        Bin::Mul => a * b,
        Bin::Div => {
            if b.abs() < 1e-9 {
                0.0
            } else {
                a / b
            }
        }
        Bin::Rem => {
            if b.abs() < 1e-9 {
                0.0
            } else {
                a % b
            }
        }
        Bin::Pow => a.powf(b),
        Bin::Min => a.min(b),
        Bin::Max => a.max(b),
        Bin::Lt => (a < b) as u8 as f32,
        Bin::Gt => (a > b) as u8 as f32,
        Bin::Le => (a <= b) as u8 as f32,
        Bin::Ge => (a >= b) as u8 as f32,
        Bin::Eq => (a == b) as u8 as f32,
        Bin::Ne => (a != b) as u8 as f32,
        Bin::And => (truthy(a) && truthy(b)) as u8 as f32,
        Bin::Or => (truthy(a) || truthy(b)) as u8 as f32,
    }
}

fn eval_color(c: &Color, ctx: &ColorCtx<'_>, p: [f32; 2]) -> [f32; 3] {
    match c {
        Color::Lit(rgb) => *rgb,
        Color::Rgb8(r, g, b) => [*r as f32 / 255.0, *g as f32 / 255.0, *b as f32 / 255.0],
        Color::Lerp(a, b, t) => {
            let (a, b) = (eval_color(a, ctx, p), eval_color(b, ctx, p));
            let t = eval_scalar(t, ctx, p).0.clamp(0.0, 1.0);
            mix_rgb(a, b, t)
        }
        Color::Chan(c, ch) => {
            let rgb = eval_color(c, ctx, p);
            let v = match ch {
                'r' => rgb[0],
                'g' => rgb[1],
                _ => rgb[2],
            };
            [v, v, v]
        }
        Color::Mix(a, b, t) => mix_rgb(eval_color(a, ctx, p), eval_color(b, ctx, p), *t),
    }
}

fn mix_rgb(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn luma(rgb: [f32; 3]) -> f32 {
    0.2126 * rgb[0] + 0.7152 * rgb[1] + 0.0722 * rgb[2]
}

/// Whether `ink` needs its dashes split up: a per-point colour or width.
pub fn per_point(ink: &Ink<'_>) -> bool {
    ink.color.is_none() && ink.expr.as_ref().is_some_and(|e| !e.constant())
        || ink.width.as_ref().is_some_and(|e| !e.constant_scalar())
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
    let mut base = Pen::with_expr(ink.stroke, ink.color, ink.expr.as_ref(), ink.layer);
    base.cap = ink.cap;
    let color = ink.color.is_none().then_some(ink.expr.as_ref()).flatten().cloned();
    let width = ink.width.clone();
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
                    let m = path_midpoint(&p);
                    let hex = color
                        .as_ref()
                        .map(|c| rgb_hex(eval_color_of(c, ctx, m)));
                    let k = width
                        .as_ref()
                        .map_or(1.0, |w| eval_scalar_of(w, ctx, m).max(0.0));
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

/// `[r, g, b]` of a tree, for callers outside this module.
pub fn eval_color_of(c: &Expr, ctx: &ColorCtx<'_>, p: [f32; 2]) -> [f32; 3] {
    match c {
        Expr::Color(c) => eval_color(c, ctx, p),
        Expr::Shade(s) => {
            let v = eval_scalar(s, ctx, p).0;
            [v, v, v]
        }
        Expr::Scalar(s) => {
            let v = eval_scalar(s, ctx, p).0;
            [v, v, v]
        }
    }
}

/// A scalar tree at a point.
pub fn eval_scalar_of(s: &Scalar, ctx: &ColorCtx<'_>, p: [f32; 2]) -> f32 {
    eval_scalar(s, ctx, p).0
}

/// `#rgb` / `#rrggbb`, or a few names, as written by hand.
pub fn parse_hex(s: &str) -> Option<String> {
    let t = s.trim();
    let h = t.strip_prefix('#')?;
    if !matches!(h.len(), 3 | 6) || !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    Some(t.to_string())
}

/// The rgb of `#rgb` or `#rrggbb`.
pub fn hex_rgb(text: &str) -> Option<[f32; 3]> {
    let t = parse_hex(text)?;
    let h = &t[1..];
    let expand = |c: char| {
        let v = u8::from_str_radix(&c.to_string().repeat(2), 16).unwrap_or(0);
        v as f32 / 255.0
    };
    if h.len() == 3 {
        let mut cs = h.chars();
        return Some([
            expand(cs.next().unwrap()),
            expand(cs.next().unwrap()),
            expand(cs.next().unwrap()),
        ]);
    }
    let parse2 = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0) as f32 / 255.0;
    Some([parse2(0), parse2(2), parse2(4)])
}

pub fn rgb_hex(rgb: [f32; 3]) -> String {
    let b = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!("#{:02x}{:02x}{:02x}", b(rgb[0]), b(rgb[1]), b(rgb[2]))
}

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

/// Lines longer than this are coloured and sized piece by piece, px.
const PIECE: f32 = 8.0;

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

fn sample(grid: &[f32], w: usize, h: usize, p: [f32; 2]) -> f32 {
    let _ = w;
    let _ = h;
    if grid.is_empty() {
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

// ---------------------------------------------------------------------------
// JSON form
// ---------------------------------------------------------------------------
//
// The form a saved document and the JavaScript front end write, and `serde`
// reads: a number, a field name, a hex colour, an rgb triple, or `[op, …]`.
// Short, and readable at a glance:
//
//   0.25                       a number; as a colour, a grey
//   "tone"                     the shading input
//   ["lerp", 2, 0.5, "tone"]   numbers, mixed
//   ["lerp", "#e8dcc8", "#2a1810", "tone"]   colours, mixed
//   ["-", "elev"]              negated
//   ["<", "elev", 100]         a comparison, 1 or 0
//   ["if", ["<", "elev", 100], 0.5, 1]       a choice
//   ["clamp", "dist", 0, 120]  a function

impl serde::Serialize for Field {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(self.name())
    }
}

impl<'de> serde::Deserialize<'de> for Field {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let t = String::deserialize(d)?;
        Field::named(&t).ok_or_else(|| serde::de::Error::custom(format!("unknown field {t:?}")))
    }
}

impl serde::Serialize for Scalar {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        fn list<S: serde::Serializer>(
            s: S,
            head: &str,
            out: impl FnOnce(&mut S::SerializeSeq) -> Result<(), S::Error>,
        ) -> Result<S::Ok, S::Error> {
            let mut q = s.serialize_seq(None)?;
            q.serialize_element(head)?;
            out(&mut q)?;
            q.end()
        }
        match self {
            Scalar::Lit(v) => s.serialize_f32(*v),
            Scalar::Field(f) => f.serialize(s),
            Scalar::Channel(c, ch) => list(s, &ch.to_string(), |q| q.serialize_element(c)),
            Scalar::Cmp(op, a, b) => list(s, op_name(*op), |q| {
                q.serialize_element(a)?;
                q.serialize_element(b)
            }),
            Scalar::Un(op, a) => list(
                s,
                match op {
                    Un::Neg => "-",
                    Un::Not => "!",
                },
                |q| q.serialize_element(a),
            ),
            Scalar::Bin(op, a, b) => list(s, op_name(*op), |q| {
                q.serialize_element(a)?;
                q.serialize_element(b)
            }),
            Scalar::Choose(c, a, b) => list(s, "if", |q| {
                q.serialize_element(c)?;
                q.serialize_element(a)?;
                q.serialize_element(b)
            }),
            Scalar::Call(f, args) => list(s, f.name(), |q| {
                for a in args {
                    q.serialize_element(a)?;
                }
                Ok(())
            }),
        }
    }
}

fn op_name(op: Bin) -> &'static str {
    match op {
        Bin::Add => "+",
        Bin::Sub => "-",
        Bin::Mul => "*",
        Bin::Div => "/",
        Bin::Rem => "%",
        Bin::Pow => "**",
        Bin::Min => "min",
        Bin::Max => "max",
        Bin::Lt => "<",
        Bin::Gt => ">",
        Bin::Le => "<=",
        Bin::Ge => ">=",
        Bin::Eq => "==",
        Bin::Ne => "!=",
        Bin::And => "&&",
        Bin::Or => "||",
    }
}

fn op_from_name(name: &str) -> Option<Bin> {
    Some(match name {
        "+" | "add" => Bin::Add,
        "-" | "sub" => Bin::Sub,
        "*" | "mul" => Bin::Mul,
        "/" | "div" => Bin::Div,
        "%" | "rem" => Bin::Rem,
        "**" | "pow" => Bin::Pow,
        "min" => Bin::Min,
        "max" => Bin::Max,
        "<" => Bin::Lt,
        ">" => Bin::Gt,
        "<=" => Bin::Le,
        ">=" => Bin::Ge,
        "==" => Bin::Eq,
        "!=" => Bin::Ne,
        "&&" => Bin::And,
        "||" => Bin::Or,
        _ => return None,
    })
}

impl serde::Serialize for Color {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeSeq;
        fn list<S: serde::Serializer>(
            s: S,
            head: Option<&str>,
            out: impl FnOnce(&mut S::SerializeSeq) -> Result<(), S::Error>,
        ) -> Result<S::Ok, S::Error> {
            let mut q = s.serialize_seq(None)?;
            if let Some(h) = head {
                q.serialize_element(h)?;
            }
            out(&mut q)?;
            q.end()
        }
        match self {
            Color::Lit(rgb) => s.serialize_str(&rgb_hex(*rgb)),
            Color::Rgb8(r, g, b) => {
                let mut q = s.serialize_seq(Some(3))?;
                q.serialize_element(r)?;
                q.serialize_element(g)?;
                q.serialize_element(b)?;
                q.end()
            }
            Color::Lerp(a, b, t) => list(s, Some("lerp"), |q| {
                q.serialize_element(a)?;
                q.serialize_element(b)?;
                q.serialize_element(t)
            }),
            Color::Mix(a, b, t) => list(s, Some("mix"), |q| {
                q.serialize_element(a)?;
                q.serialize_element(b)?;
                q.serialize_element(t)
            }),
            Color::Chan(c, ch) => list(s, Some(&ch.to_string()), |q| q.serialize_element(c)),
        }
    }
}

impl serde::Serialize for Expr {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        match self {
            Expr::Scalar(x) | Expr::Shade(x) => x.serialize(s),
            Expr::Color(c) => c.serialize(s),
        }
    }
}

impl<'de> serde::Deserialize<'de> for Scalar {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = serde_json::Value::deserialize(d)?;
        scalar_from_json(&v).map_err(serde::de::Error::custom)
    }
}

impl<'de> serde::Deserialize<'de> for Color {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = serde_json::Value::deserialize(d)?;
        color_from_json(&v).map_err(serde::de::Error::custom)
    }
}

impl<'de> serde::Deserialize<'de> for Expr {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let v = serde_json::Value::deserialize(d)?;
        expr_from_json(&v).map_err(serde::de::Error::custom)
    }
}

/// A tree is a colour when it asks for one and a shade when it does not: a
/// number, a field, arithmetic or a comparison are all scalars, and a scalar
/// used as a colour is a grey.
pub fn expr_from_json(v: &serde_json::Value) -> Result<Expr, String> {
    if wants_color(v) {
        color_from_json(v).map(Expr::Color)
    } else {
        scalar_from_json(v).map(Expr::Shade)
    }
}

/// Whether a tree in a colour's place is a colour at all.
fn wants_color(v: &serde_json::Value) -> bool {
    match v {
        serde_json::Value::String(t) => parse_hex(t).is_some() || is_palette_name(t),
        serde_json::Value::Array(a) => match a.first().and_then(|h| h.as_str()) {
            Some(head) => {
                matches!(head, "lerp" | "mix" | "r" | "g" | "b")
                    || (op_from_name(head).is_none()
                        && Func::named(head).is_none()
                        && !matches!(head, "if" | "!"))
            }
            None => false,
        },
        serde_json::Value::Number(_) => false,
        _ => false,
    }
}

/// Colour names the front ends know beyond hex.
fn is_palette_name(t: &str) -> bool {
    matches!(
        t,
        "white" | "black" | "grey" | "gray" | "ivory" | "sepia" | "ink" | "cream" | "red" | "blue"
    )
}

fn palette(t: &str) -> Option<[f32; 3]> {
    Some(match t {
        "white" => [1.0, 1.0, 1.0],
        "black" => [0.0, 0.0, 0.0],
        "grey" | "gray" => [0.5, 0.5, 0.5],
        "ivory" => [1.0, 0.99, 0.94],
        "cream" => [0.98, 0.96, 0.88],
        "sepia" => [0.44, 0.26, 0.08],
        "ink" => [0.1, 0.1, 0.12],
        "red" => [0.8, 0.1, 0.1],
        "blue" => [0.1, 0.2, 0.6],
        _ => return None,
    })
}

fn arr<'a>(v: &'a serde_json::Value, what: &str) -> Result<&'a Vec<serde_json::Value>, String> {
    v.as_array().ok_or_else(|| format!("{what}: expected a list, got {v}"))
}

fn at<'a>(a: &'a [serde_json::Value], i: usize, what: &str) -> Result<&'a serde_json::Value, String> {
    a.get(i).ok_or_else(|| format!("{what}: too few arguments"))
}

fn scalar_from_json(v: &serde_json::Value) -> Result<Scalar, String> {
    match v {
        serde_json::Value::Number(n) => {
            return Ok(Scalar::Lit(n.as_f64().unwrap_or(0.0) as f32))
        }
        serde_json::Value::Bool(b) => return Ok(Scalar::Lit(*b as u8 as f32)),
        serde_json::Value::String(t) => {
            if let Some(f) = Field::named(t) {
                return Ok(Scalar::Field(f));
            }
            if t == "NaN" {
                return Ok(Scalar::Lit(f32::NAN));
            }
            if let Some(rgb) = hex_rgb(t).or_else(|| palette(t)) {
                return Ok(Scalar::Lit(luma(rgb)));
            }
            return Err(format!("{t:?} is not a number or a field"));
        }
        _ => {}
    }
    let a = arr(v, "scalar")?;
    let head = at(a, 0, "scalar")?.as_str().unwrap_or_default();
    let num = |i: usize| scalar_from_json(at(a, i, head)?);
    Ok(match head {
        "if" => Scalar::Choose(
            Box::new(num(1)?),
            Box::new(num(2)?),
            Box::new(num(3)?),
        ),
        "!" => Scalar::Un(Un::Not, Box::new(num(1)?)),
        "-" if a.len() == 2 => Scalar::Un(Un::Neg, Box::new(num(1)?)),
        "r" | "g" | "b" => Scalar::Channel(
            Box::new(color_from_json(at(a, 1, head)?)?),
            head.chars().next().unwrap(),
        ),
        _ => {
            if let Some(op) = op_from_name(head) {
                return Ok(Scalar::Bin(op, Box::new(num(1)?), Box::new(num(2)?)));
            }
            match Func::named(head) {
                Some(f) => {
                    let args = a[1..]
                        .iter()
                        .map(scalar_from_json)
                        .collect::<Result<Vec<_>, _>>()?;
                    if args.len() != f.arity() {
                        return Err(format!(
                            "{head} takes {} arguments, got {}",
                            f.arity(),
                            args.len()
                        ));
                    }
                    Scalar::Call(f, args)
                }
                None => return Err(format!("{head:?} is not a function")),
            }
        }
    })
}

fn color_from_json(v: &serde_json::Value) -> Result<Color, String> {
    match v {
        serde_json::Value::String(t) => {
            return hex_rgb(t)
                .or_else(|| palette(t))
                .map(Color::Lit)
                .ok_or_else(|| format!("{t:?} is not a colour"))
        }
        serde_json::Value::Number(n) => {
            let n = n.as_f64().unwrap_or(0.0) as f32;
            return Ok(Color::Lit([n, n, n]));
        }
        _ => {}
    }
    let a = arr(v, "colour")?;
    if a.len() == 3 && a.iter().all(|x| x.is_u64()) {
        let byte = |i: usize| a[i].as_u64().unwrap_or(0) as u8;
        return Ok(Color::Rgb8(byte(0), byte(1), byte(2)));
    }
    let head = at(a, 0, "colour")?.as_str().unwrap_or_default();
    let side = |i: usize| -> Result<Color, String> {
        let v = at(a, i, head)?;
        // A number or a field in a colour's place is a grey.
        match color_from_json(v) {
            Ok(c) => Ok(c),
            Err(_) => Ok(Color::lerp(
                Color::gray(0.0),
                Color::gray(1.0),
                scalar_from_json(v)?,
            )),
        }
    };
    match head {
        "lerp" => Ok(Color::Lerp(
            Box::new(side(1)?),
            Box::new(side(2)?),
            Box::new(scalar_from_json(at(a, 3, head)?)?),
        )),
        "mix" => {
            let t = at(a, 3, head)?
                .as_f64()
                .ok_or_else(|| "mix: t is a number".to_string())?;
            Ok(Color::Mix(
                Box::new(side(1)?),
                Box::new(side(2)?),
                t as f32,
            ))
        }
        "r" | "g" | "b" => Ok(Color::Chan(
            Box::new(side(1)?),
            head.chars().next().unwrap(),
        )),
        other => Err(format!("{other:?} is not a colour")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx<'a>(elev: Option<&'a [f32]>, tone: Option<&'a [f32]>) -> ColorCtx<'a> {
        ColorCtx {
            w: 100,
            h: 1,
            elev,
            tone,
            dist: None,
            depth: None,
        }
    }

    /// A ramp of elevation 0…99 along x, one row.
    fn ramp() -> Vec<f32> {
        (0..100).map(|x| x as f32).collect()
    }

    #[test]
    fn fields_and_arithmetic() {
        let elev = ramp();
        let c = ctx(Some(&elev), None);
        let at = |s: Scalar, x: f32| eval_scalar_of(&s, &c, [x, 0.0]);
        let e = Scalar::Field(Field::Elev);
        assert_eq!(at(e.clone(), 40.0), 40.0);
        let two = |a: Scalar, b: Scalar| Scalar::Bin(Bin::Sub, Box::new(a), Box::new(b));
        assert_eq!(at(two(Scalar::Lit(10.0), Scalar::Lit(3.0)), 0.0), 7.0);
        assert_eq!(
            at(
                Scalar::Bin(
                    Bin::Div,
                    Box::new(Scalar::Lit(12.0)),
                    Box::new(Scalar::Lit(0.0))
                ),
                0.0
            ),
            0.0,
            "dividing by nothing is nothing, not NaN"
        );
        assert_eq!(
            at(
                Scalar::Un(Un::Neg, Box::new(Scalar::Field(Field::Elev))),
                4.0
            ),
            -4.0
        );
        assert_eq!(
            at(
                Scalar::Call(
                    Func::Lerp,
                    vec![
                        Scalar::Lit(2.0),
                        Scalar::Lit(0.5),
                        Scalar::Bin(
                            Bin::Div,
                            Box::new(Scalar::Field(Field::Elev)),
                            Box::new(Scalar::Lit(100.0))
                        )
                    ]
                ),
                50.0
            ),
            1.25
        );
        assert_eq!(
            at(
                Scalar::Call(
                    Func::Clamp,
                    vec![Scalar::Field(Field::Elev), Scalar::Lit(1.0), Scalar::Lit(0.0)]
                ),
                3.0
            ),
            1.0,
            "lo and hi may come either way round"
        );
    }

    #[test]
    fn colours_mix_and_shade() {
        let c = ctx(None, None);
        let white = Color::gray(1.0);
        let black = Color::gray(0.0);
        let half = Color::mix(black.clone(), white.clone(), 0.5f32);
        let rgb = eval_color_of(&Expr::Color(half), &c, [0.0, 0.0]);
        assert!((rgb[0] - 0.5).abs() < 0.02);
        let shade = Expr::Shade(Scalar::Lit(0.25));
        assert_eq!(eval_color_of(&shade, &c, [0.0, 0.0]), [0.25, 0.25, 0.25]);
        let ramp = [1.0f32; 100];
        let tone = ctx(None, Some(&ramp));
        // tone ramps white → black, so tone 1 is ink and tone 0 is paper.
        let dark = Color::mix(white.clone(), black.clone(), Field::Tone);
        assert_eq!(eval_color_of(&Expr::Color(dark), &tone, [0.0, 0.0]), [0.0; 3]);
        let lit = ctx(None, Some(&[0.0f32; 100]));
        let dark = Color::mix(white, black, Field::Tone);
        assert_eq!(eval_color_of(&Expr::Color(dark), &lit, [0.0, 0.0]), [1.0; 3]);
        assert!(Expr::from(Field::Elev).uses(Field::Elev));
        assert!(Expr::from(Color::tone()).uses(Field::Tone));
        assert!(!Expr::from(Color::tone()).uses(Field::Depth));
        assert!(Expr::from(Color::gray(1.0)).constant());
    }

    #[test]
    fn a_choice_picks_a_side() {
        let elev = ramp();
        let c = ctx(Some(&elev), None);
        let pick = Scalar::Choose(
            Box::new(Scalar::Bin(
                Bin::Gt,
                Box::new(Scalar::Field(Field::Elev)),
                Box::new(Scalar::Lit(50.0)),
            )),
            Box::new(Scalar::Lit(2.0)),
            Box::new(Scalar::Lit(0.5)),
        );
        assert_eq!(eval_scalar_of(&pick, &c, [10.0, 0.0]), 0.5);
        assert_eq!(eval_scalar_of(&pick, &c, [60.0, 0.0]), 2.0);
    }

    #[test]
    fn step_is_a_threshold() {
        let c = ctx(None, None);
        assert_eq!(
            eval_scalar_of(
                &Scalar::Call(Func::Step, vec![Scalar::Lit(1.0), Scalar::Lit(1.5)]),
                &c,
                [0.0, 0.0]
            ),
            1.0
        );
        assert_eq!(
            eval_scalar_of(
                &Scalar::Call(Func::Step, vec![Scalar::Lit(1.0), Scalar::Lit(0.5)]),
                &c,
                [0.0, 0.0]
            ),
            0.0
        );
    }
}
