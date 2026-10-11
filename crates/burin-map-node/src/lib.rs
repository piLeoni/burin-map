//! Node.js bindings (N-API): the native addon behind the `burin-map` npm package.
//! The public JS API is the generated `index.js` plus a thin `cli.js` for `npx`.

use burin_map::{Error as BurinError, Style};
use burin_map_core::Line;
use napi::bindgen_prelude::*;
use napi_derive::napi;

fn js_err(e: BurinError) -> Error {
    Error::from_reason(e.to_string())
}

fn style_from(obj: Option<Object>) -> Result<Style> {
    let mut st = Style::default();
    let Some(obj) = obj else {
        return Ok(st);
    };
    for p in Style::PARAMS {
        if let Ok(Some(n)) = obj.get::<f64>(p.name) {
            st.set(p.name, n);
        }
    }
    Ok(st)
}

fn pack(lines: &[Line]) -> (Float32Array, Uint32Array) {
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
    (Float32Array::new(pts), Uint32Array::new(ends))
}

/// A finished drawing.
#[napi]
pub struct Drawing {
    inner: burin_map::Map,
}

#[napi]
impl Drawing {
    #[napi(getter)]
    pub fn width(&self) -> u32 {
        self.inner.drawing.width as u32
    }

    #[napi(getter)]
    pub fn height(&self) -> u32 {
        self.inner.drawing.height as u32
    }

    #[napi(getter)]
    pub fn px_m(&self) -> f64 {
        self.inner.px_m
    }

    #[napi(getter)]
    pub fn mm_per_px(&self) -> Option<f64> {
        self.inner.mm_per_px
    }

    /// Packed polylines: interleaved x,y floats and end offsets (in points).
    #[napi]
    pub fn paths(&self) -> (Float32Array, Uint32Array) {
        pack(&self.inner.drawing.paths)
    }

    #[napi]
    pub fn lines(&self) -> (Float32Array, Uint32Array) {
        pack(&self.inner.drawing.lines)
    }

    /// SVG, sized in mm when the area has a paper size (or `mmPerPx` is given).
    #[napi]
    pub fn svg(&self, mm_per_px: Option<f64>) -> String {
        let k = mm_per_px.or(self.inner.mm_per_px);
        self.inner.drawing.to_svg(k.map(|k| k as f32))
    }

    #[napi]
    pub fn png(&self) -> Result<Buffer> {
        Ok(Buffer::from(self.inner.to_png().map_err(js_err)?))
    }

    #[napi]
    pub fn pen_up_travel(&self) -> f64 {
        burin_map_core::pen_up_travel(&self.inner.drawing.paths) as f64
    }
}

/// `(name, default, help, group)` for every style parameter.
#[napi]
pub fn style_params() -> Vec<(String, f64, String, String)> {
    let d = Style::default();
    Style::PARAMS
        .iter()
        .map(|p| {
            (
                p.name.to_string(),
                d.get(p.name).unwrap() as f32 as f64,
                p.help.to_string(),
                p.group.to_string(),
            )
        })
        .collect()
}

/// `(id, blurb)` for each style group.
#[napi]
pub fn style_groups() -> Vec<(String, String)> {
    burin_map_core::GROUPS
        .iter()
        .map(|&(id, blurb)| (id.to_string(), blurb.to_string()))
        .collect()
}

/// `(lat, lon, label)` for coordinates or a place name (Nominatim).
#[napi]
pub fn locate(query: String) -> Result<(f64, f64, String)> {
    burin_map::locate(&query).map_err(js_err)
}

/// Lon/lat box of roughly `width_km` × `height_km` centred on `(lat, lon)`.
#[napi]
pub fn bounds_around(lat: f64, lon: f64, width_km: f64, height_km: f64) -> (f64, f64, f64, f64) {
    let [w, s, e, n] = burin_map::bounds_around(lat, lon, width_km, height_km);
    (w, s, e, n)
}

#[napi]
pub fn projected_size(west: f64, south: f64, east: f64, north: f64) -> (f64, f64) {
    burin_map::projected_size(&[west, south, east, north])
}

/// Deserialise batches from a JSON string with the same schema as --style-json.
#[napi]
pub fn style_from_json(json: String) -> Result<String> {
    let batches = burin_map_core::from_json_batches(&json)
        .map_err(|e| Error::from_reason(format!("invalid JSON: {e}")))?;
    Ok(serde_json::to_string(&batches).unwrap_or_default())
}

/// Run a burin-map document (JavaScript) and return the atlas it built, as
/// JSON: the batches and water passes the engine draws, in order. This is what
/// the CLI's `--script` and the web page run; the interpreter is the front end
/// and the atlas is the document the engine reads.
#[napi]
pub fn evaluate(script: String) -> Result<String> {
    let document = burin_map_script::evaluate_document(&script)
        .map_err(|e| Error::from_reason(e.to_string()))?;
    let mut out = serde_json::Map::new();
    out.insert("atlas".into(), document.atlas);
    out.insert("area".into(), document.area.unwrap_or(serde_json::Value::Null));
    Ok(serde_json::Value::Object(out).to_string())
}

/// Read an atlas a script built: its list of batches and water passes, as
/// `burinBatch` / `burinWater` / `burinShore` return them. Comes back as the
/// batches a document carries, so a drawing can be saved and rebuilt.
#[napi]
pub fn atlas_from_json(json: String) -> Result<String> {
    let value: serde_json::Value = serde_json::from_str(&json)
        .map_err(|e| Error::from_reason(format!("invalid atlas JSON: {e}")))?;
    let atlas = burin_map_core::script::atlas_from(&value)
        .map_err(|e| Error::from_reason(format!("invalid atlas: {e}")))?;
    Ok(burin_map_core::script::atlas_to_json(&atlas).to_string())
}

/// The style parameters, as an editor's completion and help want them.
#[napi]
pub fn burin_parameters() -> String {
    burin_map_core::script::parameters().to_string()
}

/// Draw a document, `{ atlas, area }` as JSON: what `evaluate` returns and
/// `burin.draw()` hands over. Downloads the elevation (and the water, when the
/// atlas has water passes) for the area.
#[napi]
pub fn draw_document(document: String) -> Result<Drawing> {
    let v: serde_json::Value = serde_json::from_str(&document)
        .map_err(|e| Error::from_reason(format!("invalid document JSON: {e}")))?;
    let doc = burin_map::Document::from_atlas(&v["atlas"], v.get("area")).map_err(js_err)?;
    let inner = burin_map::draw_document(&doc, |_| {}).map_err(js_err)?;
    Ok(Drawing { inner })
}

/// Check one entry the library builds — `batch`, `water`, `shore` or
/// `area` — as a document script's own calls are checked.
#[napi]
pub fn check(kind: String, json: String) -> Result<()> {
    let v: serde_json::Value =
        serde_json::from_str(&json).map_err(|e| Error::from_reason(format!("invalid JSON: {e}")))?;
    use burin_map_core::script;
    let checked = match kind.as_str() {
        "batch" => script::batch_from(&v).map(drop),
        "water" => script::water_from(&v, v.get("draw").and_then(|d| d.as_str())).map(drop),
        "shore" => script::shore_from(&v).map(drop),
        "area" => script::area_from(&v).map(drop),
        other => Err(format!("nothing to check called {other:?}")),
    };
    checked.map_err(Error::from_reason)
}

/// The source of the `burin` library documents call, the same one the CLI and
/// the web page run.
#[napi]
pub fn library_source() -> String {
    burin_map_script::LIBRARY.to_string()
}

/// Draw any elevation grid (metres, row 0 = north). Values ≤ 0.5 m are sea.
#[napi]
pub fn draw_elevation(
    elev: Float32Array,
    width: u32,
    height: u32,
    px_m: f64,
    style: Option<Object>,
) -> Result<Drawing> {
    let (w, h) = (width as usize, height as usize);
    if elev.len() != w * h {
        return Err(Error::from_reason("elevation length ≠ width × height"));
    }
    let st = style_from(style)?;
    let drawing = burin_map_core::draw(elev.as_ref(), w, h, px_m as f32, &st);
    Ok(Drawing {
        inner: burin_map::Map {
            drawing,
            style: st,
            px_m,
            mm_per_px: None,
        },
    })
}

/// Never build the fields on the GPU: reproducible output, for tests and
/// golden files.
#[cfg(feature = "gpu")]
#[napi]
pub fn force_cpu() {
    burin_map::engine::force_cpu();
}

/// Where the fields are built: "GPU: Apple M4 (Metal)", "CPU (forced)", …
#[napi]
pub fn engine() -> String {
    #[cfg(feature = "gpu")]
    return burin_map::engine::describe();
    #[cfg(not(feature = "gpu"))]
    return "CPU (built without the gpu feature)".to_string();
}

/// Run the `burin-map` CLI (`argv[0]` is the program name).
#[napi]
pub fn run_cli(argv: Vec<String>) -> Result<()> {
    burin_map::cli::run(argv).map_err(|e| Error::from_reason(e.to_string()))
}

#[napi]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
