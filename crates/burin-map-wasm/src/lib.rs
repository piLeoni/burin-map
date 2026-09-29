//! WebAssembly bindings for `burin-map-core`.
//!
//! The browser fetches elevation (terrarium PNGs), then calls [`draw`] with a
//! flat `f32` height grid. Paths come back packed for canvas stroking; [`svg`]
//! builds an SVG document for download.

use burin_map_core::{Batch, GeoTransform, Line, Style};
use js_sys::{Float32Array, Object, Reflect, Uint32Array};
use wasm_bindgen::prelude::*;

/// `initThreadPool(n)`: start `n` Web Workers sharing this module's memory.
/// Call once, from a worker, before drawing.
#[cfg(feature = "threads")]
pub use wasm_bindgen_rayon::init_thread_pool;

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

/// (name, default, water_default, ripples_default, dots_default,
/// shoreline_default, help, group) for every style parameter — drives the web
/// UI. `water_default` is the value in water batches that draw lines,
/// `ripples_default` and `dots_default` in those that draw ripples or dots,
/// `shoreline_default` on the shoreline.
#[wasm_bindgen]
pub fn style_params() -> Result<JsValue, JsValue> {
    use burin_map_core::Draw;
    let kinds = [
        ("default", Style::default()),
        ("water_default", Style::water()),
        ("ripples_default", Style::drawn(Draw::Ripples)),
        ("dots_default", Style::drawn(Draw::Dots)),
        ("shoreline_default", Style::shoreline()),
    ];
    let arr = js_sys::Array::new();
    for p in Style::PARAMS {
        let o = js_sys::Object::new();
        Reflect::set(&o, &"name".into(), &p.name.into())?;
        for (key, st) in &kinds {
            Reflect::set(
                &o,
                &(*key).into(),
                &(st.get(p.name).unwrap() as f32 as f64).into(),
            )?;
        }
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
    timings: burin_map_core::Timings,
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
    /// `[label, ms]` for each phase of the draw, in the order they ran.
    #[wasm_bindgen(getter)]
    pub fn timings(&self) -> js_sys::Array {
        self.timings
            .iter()
            .map(|(label, ms)| js_sys::Array::of2(&label.into(), &(*ms).into()))
            .collect()
    }
    #[wasm_bindgen(getter)]
    pub fn svg(&self) -> String {
        self.svg.clone()
    }
    /// GeoJSON MultiLineString FeatureCollection of the dashes.
    #[wasm_bindgen(getter)]
    pub fn geojson(&self) -> String {
        let tr = GeoTransform {
            nw_lon: self.nw_lon,
            nw_lat: self.nw_lat,
            deg_per_px_lon: self.deg_per_px_lon,
            deg_per_px_lat: self.deg_per_px_lat,
        };
        let paths: Vec<Line> = {
            let pts: Vec<f32> = self.paths.to_vec();
            let ends: Vec<u32> = self.ends.to_vec();
            let mut out = Vec::with_capacity(ends.len());
            let mut start = 0u32;
            for &e in &ends {
                if e > start {
                    let slice = &pts[start as usize * 2..e as usize * 2];
                    let line: Line = slice
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .map(|c| [c[0], c[1]])
                        .collect();
                    out.push(line);
                }
                start = e;
            }
            out
        };
        burin_map_core::geojson::to_geojson(
            &paths,
            self.width as f64,
            self.height as f64,
            |x, y| tr.to_lonlat(x, y),
        )
    }
    /// NW corner longitude.
    #[wasm_bindgen(getter)]
    pub fn nw_lon(&self) -> f64 {
        self.nw_lon
    }
    /// NW corner latitude.
    #[wasm_bindgen(getter)]
    pub fn nw_lat(&self) -> f64 {
        self.nw_lat
    }
    /// Degrees of longitude per pixel (eastward).
    #[wasm_bindgen(getter)]
    pub fn deg_per_px_lon(&self) -> f64 {
        self.deg_per_px_lon
    }
    /// Degrees of latitude per pixel (southward).
    #[wasm_bindgen(getter)]
    pub fn deg_per_px_lat(&self) -> f64 {
        self.deg_per_px_lat
    }
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
    let svg = d.to_svg(None);
    let geo = read_geo(geo);
    Ok(Drawing {
        width,
        height,
        paths: Float32Array::from(pts.as_slice()),
        ends: Uint32Array::from(ends.as_slice()),
        n_lines: d.lines.len() as u32,
        n_dashes: d.paths.len() as u32,
        ms,
        timings: Vec::new(),
        svg,
        nw_lon: geo.nw_lon,
        nw_lat: geo.nw_lat,
        deg_per_px_lon: geo.deg_per_px_lon,
        deg_per_px_lat: geo.deg_per_px_lat,
    })
}

/// `performance.now()` of the calling thread. Each rayon thread is its own
/// Web Worker with its own time origin, so only differences taken on one
/// thread mean anything.
fn now_ms() -> f64 {
    let global = js_sys::global();
    Reflect::get(&global, &"performance".into())
        .ok()
        .and_then(|perf| {
            let now = Reflect::get(&perf, &"now".into()).ok()?;
            now.dyn_into::<js_sys::Function>()
                .ok()?
                .call0(&perf)
                .ok()?
                .as_f64()
        })
        .unwrap_or_else(js_sys::Date::now)
}

fn read_double(o: &JsValue, key: &str, default: f64) -> f64 {
    if o.is_undefined() || o.is_null() {
        return default;
    }
    Reflect::get(o, &key.into())
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(default)
}

fn read_geo(o: &JsValue) -> GeoTransform {
    if o.is_undefined() || o.is_null() {
        return GeoTransform {
            nw_lon: 0.0,
            nw_lat: 0.0,
            deg_per_px_lon: 0.0,
            deg_per_px_lat: 0.0,
        };
    }
    GeoTransform {
        nw_lon: read_double(o, "nw_lon", 0.0),
        nw_lat: read_double(o, "nw_lat", 0.0),
        deg_per_px_lon: read_double(o, "deg_per_px_lon", 0.0),
        deg_per_px_lat: read_double(o, "deg_per_px_lat", 0.0),
    }
}

/// Water tiles to fetch for a north-up Web Mercator grid `width` px across the
/// box, flat: z, x, y, z, x, y, … Across the 180° meridian x runs past
/// 2^z - 1; wrap it for the URL, but hand it back as it is.
#[wasm_bindgen]
pub fn water_tiles(
    west: f64,
    south: f64,
    east: f64,
    north: f64,
    width: u32,
    max_zoom: u8,
    max_tiles: u32,
) -> Vec<u32> {
    let b = [west, south, east, north];
    let limits = watermask::ZoomLimits {
        max_zoom,
        max_tiles: max_tiles as usize,
    };
    let z = watermask::zoom_for(&b, width as usize, &limits);
    watermask::tiles_for(&b, z)
        .into_iter()
        .flat_map(|t| [t.z as u32, t.x, t.y])
        .collect()
}

/// What the page asks for on water; the document's `water` minus its source.
#[derive(serde::Deserialize)]
struct WaterSpec {
    classes: Option<Vec<String>>,
    #[serde(
        default = "burin_map_core::water::shoreline_option::default",
        with = "burin_map_core::water::shoreline_option"
    )]
    shoreline: Option<burin_map_core::Shoreline>,
    #[serde(default)]
    batches: Vec<burin_map_core::WaterBatch>,
}

/// `water` is `undefined`, or `{ spec, bounds, tiles }`: `spec` the water part
/// of the document as JSON, `bounds` the grid's `[west, south, east, north]`
/// (north-up Web Mercator, like the elevation), `tiles` an array of
/// `[z, x, y, bytes]` as listed by [`water_tiles`].
fn read_water(
    water: &JsValue,
    w: usize,
    h: usize,
) -> Result<Option<(Vec<f32>, WaterSpec)>, JsValue> {
    if water.is_undefined() || water.is_null() {
        return Ok(None);
    }
    let get = |k: &str| Reflect::get(water, &k.into());
    let spec: WaterSpec = serde_json::from_str(&get("spec")?.as_string().unwrap_or_default())
        .map_err(|e| JsValue::from_str(&format!("invalid water: {e}")))?;
    let b: Vec<f64> = js_sys::Array::from(&get("bounds")?)
        .iter()
        .filter_map(|v| v.as_f64())
        .collect();
    let [west, south, east, north] = b[..] else {
        return Err(JsValue::from_str(
            "water.bounds needs [west, south, east, north]",
        ));
    };
    let mut filter = watermask::Filter {
        lines: Vec::new(),
        ..Default::default()
    };
    if let Some(c) = &spec.classes {
        filter.areas = c.clone();
    }
    let mut found = watermask::Water::new();
    for t in js_sys::Array::from(&get("tiles")?).iter() {
        let t = js_sys::Array::from(&t);
        let n = |i| t.get(i).as_f64().unwrap_or(0.0) as u32;
        let bytes = js_sys::Uint8Array::new(&t.get(3)).to_vec();
        if bytes.is_empty() {
            continue;
        }
        let id = watermask::TileId::new(n(0) as u8, n(1), n(2));
        found
            .add_tile(id, &bytes, &filter)
            .map_err(|e| JsValue::from_str(&e.to_string()))?;
    }
    let grid = watermask::Grid::new([west, south, east, north], w, h);
    let mask = found.mask(&grid, &watermask::MaskOptions::default());
    Ok(Some((mask.coverage, spec)))
}

/// Draw with multiple batch passes — accepts a JSON string of `Vec<Batch>`.
/// `geo` is an optional `{ nw_lon, nw_lat, deg_per_px_lon, deg_per_px_lat }`.
/// With `mm_per_px` the SVG is sized in millimetres, for paper. `water`: see
/// [`read_water`]; it decides the land, and its batches draw on it.
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub fn draw_batches(
    elev: &[f32],
    width: u32,
    height: u32,
    px_m: f32,
    json: &str,
    geo: &JsValue,
    mm_per_px: Option<f32>,
    water: &JsValue,
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
    let batches = parse_batches(json)?;
    let t0 = now_ms();
    let wet = read_water(water, w, h)?;
    let mut timings = Vec::new();
    if wet.is_some() {
        timings.push(("water mask".to_string(), now_ms() - t0));
    }
    let t1 = now_ms();
    let coverage = wet.as_ref().map(|(c, _)| c.as_slice());
    let fields: Vec<_> = burin_map_core::terrain_groups(&batches)
        .0
        .iter()
        .map(|&i| burin_map_core::build_fields_with(elev, w, h, px_m, &batches[i].style, coverage))
        .collect();
    timings.push((terrain_label(fields.len(), "CPU"), now_ms() - t1));
    let layer = layer(&wet);
    let (d, t) = burin_map_core::draw_fields_timed(&fields, &batches, layer.as_ref(), now_ms);
    timings.extend(t);
    Ok(finish(d, now_ms() - t0, timings, geo, mm_per_px))
}

fn terrain_label(n: usize, device: &str) -> String {
    match n {
        1 => format!("terrain fields ({device})"),
        n => format!("terrain fields ×{n} ({device})"),
    }
}

fn layer(wet: &Option<(Vec<f32>, WaterSpec)>) -> Option<burin_map_core::WaterLayer<'_>> {
    wet.as_ref().map(|(c, s)| burin_map_core::WaterLayer {
        coverage: c,
        shoreline: s.shoreline.as_ref(),
        batches: &s.batches,
    })
}

fn parse_batches(json: &str) -> Result<Vec<Batch>, JsValue> {
    let batches = burin_map_core::from_json_batches(json)
        .map_err(|e| JsValue::from_str(&format!("invalid batch JSON: {e}")))?;
    if batches.is_empty() {
        return Err(JsValue::from_str("batch list is empty"));
    }
    Ok(batches)
}

#[cfg(feature = "gpu")]
thread_local! {
    static GPU: std::cell::RefCell<Option<std::rc::Rc<burin_map_core::gpu::Gpu>>> = const { std::cell::RefCell::new(None) };
}

/// `initGpu()`: set up WebGPU for [`draw_batches_gpu`]. Resolves to the
/// adapter description, or `undefined` when WebGPU isn't available.
#[cfg(feature = "gpu")]
#[wasm_bindgen(js_name = initGpu)]
pub async fn init_gpu() -> Option<String> {
    if let Some(g) = GPU.with(|g| g.borrow().clone()) {
        return Some(g.name.clone());
    }
    let g = burin_map_core::gpu::Gpu::new().await?;
    let name = g.name.clone();
    GPU.with(|slot| *slot.borrow_mut() = Some(std::rc::Rc::new(g)));
    Some(name)
}

/// [`draw_batches`] with the terrain fields built on the GPU (call
/// [`init_gpu`] first). Lines come out slightly different from the CPU path.
#[cfg(feature = "gpu")]
#[wasm_bindgen]
#[allow(clippy::too_many_arguments)]
pub async fn draw_batches_gpu(
    elev: Vec<f32>,
    width: u32,
    height: u32,
    px_m: f32,
    json: String,
    geo: JsValue,
    mm_per_px: Option<f32>,
    water: JsValue,
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
    let batches = parse_batches(&json)?;
    let gpu = GPU
        .with(|g| g.borrow().clone())
        .ok_or_else(|| JsValue::from_str("call initGpu() first"))?;
    let t0 = now_ms();
    let wet = read_water(&water, w, h)?;
    let mut timings = Vec::new();
    if wet.is_some() {
        timings.push(("water mask".to_string(), now_ms() - t0));
    }
    let t1 = now_ms();
    let coverage = wet.as_ref().map(|(c, _)| c.as_slice());
    let mut fields = Vec::new();
    for i in burin_map_core::terrain_groups(&batches).0 {
        fields.push(
            gpu.build_fields_with(&elev, w, h, px_m, &batches[i].style, coverage)
                .await,
        );
    }
    timings.push((terrain_label(fields.len(), "GPU"), now_ms() - t1));
    let layer = layer(&wet);
    let (d, t) = burin_map_core::draw_fields_timed(&fields, &batches, layer.as_ref(), now_ms);
    timings.extend(t);
    Ok(finish(d, now_ms() - t0, timings, &geo, mm_per_px))
}

fn finish(
    d: burin_map_core::Drawing,
    ms: f64,
    mut timings: burin_map_core::Timings,
    geo: &JsValue,
    mm_per_px: Option<f32>,
) -> Drawing {
    let (width, height) = (d.width as u32, d.height as u32);
    let t = now_ms();
    let (pts, ends) = pack(&d.paths);
    let svg = d.to_svg(mm_per_px);
    timings.push(("svg".into(), now_ms() - t));
    let geo = read_geo(geo);
    Drawing {
        width,
        height,
        paths: Float32Array::from(pts.as_slice()),
        ends: Uint32Array::from(ends.as_slice()),
        n_lines: d.lines.len() as u32,
        n_dashes: d.paths.len() as u32,
        ms,
        timings,
        svg,
        nw_lon: geo.nw_lon,
        nw_lat: geo.nw_lat,
        deg_per_px_lon: geo.deg_per_px_lon,
        deg_per_px_lat: geo.deg_per_px_lat,
    }
}
