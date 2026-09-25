//! burin-map: pen-and-ink terrain maps from SRTM elevation.
//!
//! The drawing engine is `burin-map-core` (pure computation, also built for WASM);
//! this crate adds everything around it: geocoding, elevation download and
//! reprojection, PNG output and page sizing.
//!
//! ```no_run
//! let (lat, lon, _) = burin_map::locate("Twin Peaks, San Francisco").unwrap();
//! let area = burin_map::bounds_around(lat, lon, 20.0, 20.0);
//! let map = burin_map::draw(&area, &burin_map::Style::default(), &burin_map::Page::default(), |p| eprintln!("{p}")).unwrap();
//! std::fs::write("sf.svg", map.to_svg()).unwrap();
//! ```

pub mod cli;
pub mod dem;
pub mod geo;
pub mod progress;
pub mod raster;
pub mod tmerc;

pub use burin_map_core::{Drawing, Line, Style};
pub use dem::{load_dem, projected_aspect, projected_size, Bounds, Dem};
pub use geo::{bounds_around, locate, parse_length_km, parse_scale};
pub use progress::{Progress, TileSource};

#[derive(Debug)]
pub enum Error {
    /// Bad arguments.
    Input(String),
    /// Download or geocoding failed.
    Net(String),
    /// Unexpected content in downloaded or cached data.
    Data(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Error::Input(m) | Error::Net(m) | Error::Data(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for Error {}

/// Output size. With `page_width_mm` the drawing is sized for paper: the pen
/// takes the place of `style.stroke`, so every proportion of the style is
/// kept and the pixel width follows from page and pen. With `page_height_mm`
/// too, the drawing shrinks to fit inside both.
#[derive(Clone, Debug)]
pub struct Page {
    /// Pixel width when no page is given.
    pub width: usize,
    pub page_width_mm: Option<f64>,
    pub page_height_mm: Option<f64>,
    pub pen_mm: f64,
}

impl Default for Page {
    fn default() -> Self {
        Page { width: 2400, page_width_mm: None, page_height_mm: None, pen_mm: 0.3 }
    }
}

pub struct Map {
    pub drawing: Drawing,
    pub style: Style,
    /// Metres per pixel on the ground.
    pub px_m: f64,
    /// Physical size, when drawn for a page.
    pub mm_per_px: Option<f64>,
}

impl Map {
    pub fn to_svg(&self) -> String {
        self.drawing.to_svg(self.style.stroke, self.mm_per_px.map(|k| k as f32))
    }

    pub fn to_png(&self) -> Result<Vec<u8>, Error> {
        let d = &self.drawing;
        raster::to_png(&d.paths, d.width, d.height, self.style.stroke)
    }
}

/// Download SRTM elevation for `bounds` (west, south, east, north in degrees)
/// and draw it, reporting each step to `progress`.
pub fn draw(bounds: &Bounds, style: &Style, page: &Page, progress: impl Fn(&Progress) + Sync)
    -> Result<Map, Error> {
    use std::time::Instant;
    let mut width = page.width;
    let mut mm_per_px = None;
    if let Some(pw) = page.page_width_mm {
        let k = page.pen_mm / style.stroke as f64;
        let mut width_mm = pw;
        if let Some(ph) = page.page_height_mm {
            width_mm = width_mm.min(ph / projected_aspect(bounds));
        }
        width = (width_mm / k).round() as usize;
        mm_per_px = Some(k);
    }
    let dem = dem::load_dem_with(bounds, width, &progress)?;
    let (w, h) = (dem.width, dem.height);
    if let Some(k) = mm_per_px {
        progress(&Progress::Page {
            width_mm: w as f64 * k,
            height_mm: h as f64 * k,
            scale: dem.px_m * 1000.0 / k,
            pen_mm: page.pen_mm,
            pitch_mm: style.spacing as f64 * k,
        });
    }

    let t = Instant::now();
    let f = burin_map_core::build_fields(&dem.elev, w, h, dem.px_m as f32, style);
    progress(&Progress::Relief { secs: t.elapsed().as_secs_f64() });
    let t = Instant::now();
    let lines = burin_map_core::trace_streamlines(&f, style);
    progress(&Progress::Lines { count: lines.len(), secs: t.elapsed().as_secs_f64() });
    let t = Instant::now();
    let lines = burin_map_core::plot_order(lines);
    let paths = burin_map_core::dash_lines(&lines, &f.tone, w, h, style);
    progress(&Progress::Dashes { count: paths.len(), secs: t.elapsed().as_secs_f64() });
    let drawing = Drawing { width: w, height: h, paths, lines };
    Ok(Map { drawing, style: style.clone(), px_m: dem.px_m, mm_per_px })
}
