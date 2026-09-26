//! burin-map: pen-and-ink terrain maps from SRTM elevation.
//!
//! The drawing engine is `burin-map-core` (pure computation, also built for WASM);
//! this crate adds everything around it: geocoding, elevation download and
//! reprojection, PNG output and frame sizing.
//!
//! ```no_run
//! let (lat, lon, _) = burin_map::locate("Twin Peaks, San Francisco").unwrap();
//! let area = burin_map::bounds_around(lat, lon, 20.0, 20.0);
//! let map = burin_map::draw(&area, &burin_map::Style::default(), &burin_map::Frame::default(), |p| eprintln!("{p}")).unwrap();
//! std::fs::write("sf.svg", map.to_svg()).unwrap();
//! ```

pub mod cli;
pub mod dem;
pub mod geo;
pub mod progress;
pub mod raster;
pub mod tmerc;

pub use burin_map_core::{Batch, Drawing, Line, Style};
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

/// The rectangle of the drawing. With `width_mm` it has a physical size: the
/// pen takes the place of `style.batch_stroke`, so every proportion of the
/// style is kept and the pixel width follows from frame and pen. With
/// `height_mm` too, the drawing shrinks to fit inside both.
#[derive(Clone, Debug)]
pub struct Frame {
    /// Pixel width when no `width_mm` is given.
    pub px: usize,
    pub width_mm: Option<f64>,
    pub height_mm: Option<f64>,
    pub pen_mm: f64,
}

impl Default for Frame {
    fn default() -> Self {
        Frame { px: 2400, width_mm: None, height_mm: None, pen_mm: 0.3 }
    }
}

pub struct Map {
    pub drawing: Drawing,
    pub style: Style,
    /// Metres per pixel on the ground.
    pub px_m: f64,
    /// Physical size, when the frame has one.
    pub mm_per_px: Option<f64>,
}

impl Map {
    pub fn to_svg(&self) -> String {
        self.drawing.to_svg(self.style.batch_stroke, self.mm_per_px.map(|k| k as f32))
    }

    pub fn to_png(&self) -> Result<Vec<u8>, Error> {
        let d = &self.drawing;
        raster::to_png(&d.paths, d.width, d.height, self.style.batch_stroke)
    }
}

/// Download SRTM elevation for `bounds` and draw it with a single batch.
pub fn draw(
    bounds: &Bounds,
    style: &Style,
    frame: &Frame,
    progress: impl Fn(&Progress) + Sync,
) -> Result<Map, Error> {
    draw_batches(bounds, &[Batch::along(style.clone())], frame, progress)
}

/// Download SRTM and draw with one or more batch passes (see `Batch`).
pub fn draw_batches(
    bounds: &Bounds,
    batches: &[Batch],
    frame: &Frame,
    progress: impl Fn(&Progress) + Sync,
) -> Result<Map, Error> {
    use std::time::Instant;
    assert!(!batches.is_empty());
    let style = &batches[0].style;
    let mut width = frame.px;
    let mut mm_per_px = None;
    if let Some(fw) = frame.width_mm {
        let k = frame.pen_mm / style.batch_stroke as f64;
        let mut width_mm = fw;
        if let Some(fh) = frame.height_mm {
            width_mm = width_mm.min(fh / projected_aspect(bounds));
        }
        width = (width_mm / k).round() as usize;
        mm_per_px = Some(k);
    }
    let dem = dem::load_dem_with(bounds, width, &progress)?;
    let (w, h) = (dem.width, dem.height);
    if let Some(k) = mm_per_px {
        progress(&Progress::Frame {
            width_mm: w as f64 * k,
            height_mm: h as f64 * k,
            scale: dem.px_m * 1000.0 / k,
            pen_mm: frame.pen_mm,
            pitch_mm: style.space as f64 * k,
        });
    }

    let t = Instant::now();
    let drawing = burin_map_core::draw_batches(&dem.elev, w, h, dem.px_m as f32, batches);
    let secs = t.elapsed().as_secs_f64();
    progress(&Progress::Relief { secs: secs * 0.3 });
    progress(&Progress::Lines { count: drawing.lines.len(), secs: secs * 0.5 });
    progress(&Progress::Dashes { count: drawing.paths.len(), secs: secs * 0.2 });
    Ok(Map {
        drawing,
        style: style.clone(),
        px_m: dem.px_m,
        mm_per_px,
    })
}
