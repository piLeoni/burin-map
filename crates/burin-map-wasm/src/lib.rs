//! WebAssembly bindings for `burin-map-core`.
//!
//! The browser fetches elevation (terrarium PNGs), then calls [`draw`] with a
//! flat `f32` height grid. Paths come back packed for canvas stroking; [`svg`]
//! builds an SVG document for download.

use burin_map_core::{Line, Style};
use js_sys::{Float32Array, Object, Reflect, Uint32Array};
use wasm_bindgen::prelude::*;

fn style_from(obj: &JsValue) -> Result<Style, JsValue> {
    let mut st = Style::default();
    if obj.is_undefined() || obj.is_null() {
        return Ok(st);
    }
    let o = Object::from(obj.clone());
    for p in Style::PARAMS {
        if let Ok(v) = Reflect::get(&o, &p.name.into()) {
            if let Some(n) = v.as_f64() {
                st.set(p.name, n);
            }
        }
    }
    Ok(st)
}

/// (name, default, help, group) for every style parameter — drives the web UI.
#[wasm_bindgen]
pub fn style_params() -> Result<JsValue, JsValue> {
    let d = Style::default();
    let arr = js_sys::Array::new();
    for p in Style::PARAMS {
        let o = js_sys::Object::new();
        Reflect::set(&o, &"name".into(), &p.name.into())?;
        Reflect::set(&o, &"default".into(), &(d.get(p.name).unwrap() as f32 as f64).into())?;
        Reflect::set(&o, &"help".into(), &p.help.into())?;
        Reflect::set(&o, &"group".into(), &p.group.into())?;
        arr.push(&o);
    }
    Ok(arr.into())
}

/// (id, blurb) for each style group — section headers in the web UI.
#[wasm_bindgen]
pub fn style_groups() -> Result<JsValue, JsValue> {
    let arr = js_sys::Array::new();
    for &(id, blurb) in burin_map_core::GROUPS {
        let o = js_sys::Object::new();
        Reflect::set(&o, &"id".into(), &id.into())?;
        Reflect::set(&o, &"blurb".into(), &blurb.into())?;
        arr.push(&o);
    }
    Ok(arr.into())
}

/// Packed polylines: little-endian f32 x,y pairs and u32 end offsets (in points).
fn pack(lines: &[Line]) -> (Vec<f32>, Vec<u32>) {
    let n_pts: usize = lines.iter().map(|l| l.len()).sum();
    let mut pts = Vec::with_capacity(n_pts * 2);
    let mut ends = Vec::with_capacity(lines.len());
    let mut n = 0u32;
    for l in lines {
        for p in l {
            pts.push(p[0]);
            pts.push(p[1]);
        }
        n += l.len() as u32;
        ends.push(n);
    }
    (pts, ends)
}

/// Result of a draw: canvas-ready paths plus an SVG string.
#[wasm_bindgen]
pub struct Drawing {
    width: u32,
    height: u32,
    paths: Float32Array,
    ends: Uint32Array,
    n_lines: u32,
    n_dashes: u32,
    ms: f64,
    svg: String,
}

#[wasm_bindgen]
impl Drawing {
    #[wasm_bindgen(getter)]
    pub fn width(&self) -> u32 {
        self.width
    }
    #[wasm_bindgen(getter)]
    pub fn height(&self) -> u32 {
        self.height
    }
    #[wasm_bindgen(getter)]
    pub fn paths(&self) -> Float32Array {
        self.paths.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn ends(&self) -> Uint32Array {
        self.ends.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn lines(&self) -> u32 {
        self.n_lines
    }
    #[wasm_bindgen(getter)]
    pub fn dashes(&self) -> u32 {
        self.n_dashes
    }
    #[wasm_bindgen(getter)]
    pub fn ms(&self) -> f64 {
        self.ms
    }
    #[wasm_bindgen(getter)]
    pub fn svg(&self) -> String {
        self.svg.clone()
    }
}

/// Draw an elevation grid: metres, row 0 = north, square pixels of `px_m`
/// metres. `elev` is a Float32Array (or any TypedArray view) of length
/// `width × height`. Values ≤ 0.5 m are sea. `style` is an object of
/// parameter overrides (missing keys keep the defaults).
#[wasm_bindgen]
pub fn draw(
    elev: &[f32],
    width: u32,
    height: u32,
    px_m: f32,
    style: &JsValue,
) -> Result<Drawing, JsValue> {
    let (w, h) = (width as usize, height as usize);
    if elev.len() != w * h {
        return Err(JsValue::from_str(&format!(
            "elevation length {} ≠ {}×{}",
            elev.len(),
            w,
            h
        )));
    }
    let st = style_from(style)?;
    let t0 = now_ms();
    let d = burin_map_core::draw(elev, w, h, px_m, &st);
    let ms = now_ms() - t0;
    let (pts, ends) = pack(&d.paths);
    let svg = d.to_svg(st.batch_stroke, None);
    Ok(Drawing {
        width,
        height,
        paths: Float32Array::from(pts.as_slice()),
        ends: Uint32Array::from(ends.as_slice()),
        n_lines: d.lines.len() as u32,
        n_dashes: d.paths.len() as u32,
        ms,
        svg,
    })
}

fn now_ms() -> f64 {
    js_sys::Date::now()
}
