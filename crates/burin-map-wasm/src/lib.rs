//! WebAssembly bindings for `burin-map-core`.
//!
//! The browser fetches elevation (terrarium PNGs), then calls [`draw`] with a
//! flat `f32` height grid. Paths come back packed for canvas stroking; [`svg`]
//! builds an SVG document for download.

use burin_map_core::{Batch, GeoTransform, Line, Style};
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
    /// Pixel → lon/lat, when geography was provided.
    nw_lon: f64,
    nw_lat: f64,
    deg_per_px_lon: f64,
    deg_per_px_lat: f64,
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
    /// GeoJSON MultiLineString FeatureCollection of the dashes.
    #[wasm_bindgen(getter)]
    pub fn geojson(&self) -> String {
        let tr = GeoTransform { nw_lon: self.nw_lon, nw_lat: self.nw_lat, deg_per_px_lon: self.deg_per_px_lon, deg_per_px_lat: self.deg_per_px_lat };
        let paths: Vec<Line> = {
            let pts: Vec<f32> = self.paths.to_vec();
            let ends: Vec<u32> = self.ends.to_vec();
            let mut out = Vec::with_capacity(ends.len());
            let mut start = 0u32;
            for &e in &ends {
                if e > start {
                    let slice = &pts[start as usize * 2..e as usize * 2];
                    let line: Line = slice.chunks_exact(2).map(|c| [c[0], c[1]]).collect();
                    out.push(line);
                }
                start = e;
            }
            out
        };
        burin_map_core::geojson::to_geojson(&paths, self.width as f64, self.height as f64, |x, y| tr.to_lonlat(x, y))
    }
    /// NW corner longitude.
    #[wasm_bindgen(getter)]
    pub fn nw_lon(&self) -> f64 { self.nw_lon }
    /// NW corner latitude.
    #[wasm_bindgen(getter)]
    pub fn nw_lat(&self) -> f64 { self.nw_lat }
    /// Degrees of longitude per pixel (eastward).
    #[wasm_bindgen(getter)]
    pub fn deg_per_px_lon(&self) -> f64 { self.deg_per_px_lon }
    /// Degrees of latitude per pixel (southward).
    #[wasm_bindgen(getter)]
    pub fn deg_per_px_lat(&self) -> f64 { self.deg_per_px_lat }
}

/// Draw an elevation grid: metres, row 0 = north, square pixels of `px_m`
/// metres. `elev` is a Float32Array (or any TypedArray view) of length
/// `width × height`. Values ≤ 0.5 m are sea. `style` is an object of
/// parameter overrides (missing keys keep the defaults).
/// `geo` is an optional `{ nw_lon, nw_lat, deg_per_px_lon, deg_per_px_lat }` —
/// when present, `.geojson` will contain geographic coordinates.
#[wasm_bindgen]
pub fn draw(
    elev: &[f32],
    width: u32,
    height: u32,
    px_m: f32,
    style: &JsValue,
    geo: &JsValue,
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
    let geo = read_geo(geo);
    Ok(Drawing {
        width,
        height,
        paths: Float32Array::from(pts.as_slice()),
        ends: Uint32Array::from(ends.as_slice()),
        n_lines: d.lines.len() as u32,
        n_dashes: d.paths.len() as u32,
        ms,
        svg,
        nw_lon: geo.nw_lon,
        nw_lat: geo.nw_lat,
        deg_per_px_lon: geo.deg_per_px_lon,
        deg_per_px_lat: geo.deg_per_px_lat,
    })
}

fn now_ms() -> f64 {
    js_sys::Date::now()
}

fn read_double(o: &JsValue, key: &str, default: f64) -> f64 {
    if o.is_undefined() || o.is_null() { return default; }
    Reflect::get(&o, &key.into()).ok().and_then(|v| v.as_f64()).unwrap_or(default)
}

fn read_geo(o: &JsValue) -> GeoTransform {
    if o.is_undefined() || o.is_null() { return GeoTransform { nw_lon: 0.0, nw_lat: 0.0, deg_per_px_lon: 0.0, deg_per_px_lat: 0.0 }; }
    GeoTransform {
        nw_lon: read_double(o, "nw_lon", 0.0),
        nw_lat: read_double(o, "nw_lat", 0.0),
        deg_per_px_lon: read_double(o, "deg_per_px_lon", 0.0),
        deg_per_px_lat: read_double(o, "deg_per_px_lat", 0.0),
    }
}

/// Draw with multiple batch passes — accepts a JSON string of `Vec<Batch>`.
/// `geo` is an optional `{ nw_lon, nw_lat, deg_per_px_lon, deg_per_px_lat }`.
/// With `mm_per_px` the SVG is sized in millimetres, for paper.
#[wasm_bindgen]
pub fn draw_batches(
    elev: &[f32],
    width: u32,
    height: u32,
    px_m: f32,
    json: &str,
    geo: &JsValue,
    mm_per_px: Option<f32>,
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
    let batches: Vec<Batch> = serde_json::from_str(json)
        .map_err(|e| JsValue::from_str(&format!("invalid batch JSON: {e}")))?;
    if batches.is_empty() {
        return Err(JsValue::from_str("batch list is empty"));
    }
    let t0 = now_ms();
    let d = burin_map_core::draw_batches(elev, w, h, px_m, &batches);
    let ms = now_ms() - t0;
    let (pts, ends) = pack(&d.paths);
    let stroke = batches.first().map(|b| b.style.batch_stroke).unwrap_or(1.2);
    let svg = d.to_svg(stroke, mm_per_px);
    let geo = read_geo(geo);
    Ok(Drawing {
        width,
        height,
        paths: Float32Array::from(pts.as_slice()),
        ends: Uint32Array::from(ends.as_slice()),
        n_lines: d.lines.len() as u32,
        n_dashes: d.paths.len() as u32,
        ms,
        svg,
        nw_lon: geo.nw_lon,
        nw_lat: geo.nw_lat,
        deg_per_px_lon: geo.deg_per_px_lon,
        deg_per_px_lat: geo.deg_per_px_lat,
    })
}
