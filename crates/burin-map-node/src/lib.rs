//! Node.js bindings (N-API): the native addon behind the `burin-map` npm package.
//! The public JS API is the generated `index.js` plus a thin `cli.js` for `npx`.

use burin_map::{Error as BurinError, Page, Style};
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
    for &(name, _) in Style::PARAMS {
        if let Ok(v) = obj.get::<f64>(name) {
            if let Some(n) = v {
                if !st.set(name, n) {
                    return Err(Error::from_reason(format!("unknown style parameter {name:?}")));
                }
            }
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

    #[napi]
    pub fn svg(&self, mm_per_px: Option<f64>) -> String {
        self.inner
            .drawing
            .to_svg(self.inner.style.stroke, mm_per_px.map(|k| k as f32))
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

/// `(name, default, help)` for every style parameter.
#[napi]
pub fn style_params() -> Vec<(String, f64, String)> {
    let d = Style::default();
    Style::PARAMS
        .iter()
        .map(|&(n, h)| (n.to_string(), d.get(n).unwrap() as f32 as f64, h.to_string()))
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

/// Download SRTM for the lon/lat box and draw it.
#[napi]
pub fn draw(
    west: f64,
    south: f64,
    east: f64,
    north: f64,
    style: Option<Object>,
    width: Option<u32>,
    page_width_mm: Option<f64>,
    page_height_mm: Option<f64>,
    pen_mm: Option<f64>,
) -> Result<Drawing> {
    let st = style_from(style)?;
    let page = Page {
        width: width.unwrap_or(2400) as usize,
        page_width_mm,
        page_height_mm,
        pen_mm: pen_mm.unwrap_or(0.3),
    };
    let b = [west, south, east, north];
    let inner = burin_map::draw(&b, &st, &page, |_| {}).map_err(js_err)?;
    Ok(Drawing { inner })
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

/// Run the `burin-map` CLI (`argv[0]` is the program name).
#[napi]
pub fn run_cli(argv: Vec<String>) -> Result<()> {
    burin_map::cli::run(argv)
        .map_err(|e| Error::from_reason(e.to_string()))
}

#[napi]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
