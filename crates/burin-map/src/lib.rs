//! burin-map: pen-and-ink terrain maps from terrain tiles.
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

use serde::{Deserialize, Serialize};

pub mod cli;
pub mod dem;
pub mod document;
pub mod geo;
mod panel;
pub mod progress;
pub mod raster;
pub mod tmerc;
pub mod water;

pub use burin_map_core::{
    Batch, Draw, Drawing, Follow, GeoTransform, Line, Shoreline, Style, WaterBatch,
};
pub use dem::{load_dem, projected_aspect, projected_size, Bounds, Dem};
pub use document::{AreaDoc, Document, LandDoc, WaterDoc};
pub use geo::{bounds_around, locate, parse_length_km, parse_scale};
pub use progress::Progress;

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

/// The rectangle of the drawing. With `size_mm` it is a sheet of paper: the
/// pen takes the place of `style.batch_stroke`, so every proportion of the
/// style is kept and the pixel width follows from paper and the pen width.
/// When the ground and the paper have different proportions, the drawing
/// shrinks to fit inside the paper.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Frame {
    /// Grid width in px when there is no paper size. Not a user setting.
    pub px: usize,
    /// Paper `[width, height]` in mm.
    pub size_mm: Option<[f64; 2]>,
    /// Pen width on paper, mm. With `size_mm` it sets the scale: a finer pen
    /// means more pixels, so more lines on the same sheet.
    pub pen_width: f64,
}

impl Default for Frame {
    fn default() -> Self {
        Frame {
            px: 2400,
            size_mm: None,
            pen_width: 0.3,
        }
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
        self.drawing.to_svg(self.mm_per_px.map(|k| k as f32))
    }

    pub fn to_png(&self) -> Result<Vec<u8>, Error> {
        raster::to_png(&self.drawing)
    }

    /// GeoJSON with geographic coordinates (the caller must supply the bounds).
    pub fn to_geojson(&self, bounds: &Bounds) -> String {
        let pix_w = self.drawing.width as f64;
        let pix_h = self.drawing.height as f64;
        let deg_per_px_lon = (bounds[2] - bounds[0]) / pix_w;
        let deg_per_px_lat = (bounds[3] - bounds[1]) / pix_h;
        let tr = burin_map_core::GeoTransform {
            nw_lon: bounds[0] + deg_per_px_lon * 0.5,
            nw_lat: bounds[3] - deg_per_px_lat * 0.5,
            deg_per_px_lon,
            deg_per_px_lat,
        };
        burin_map_core::geojson::to_geojson_with_fills(
            &self.drawing.paths,
            &self.drawing.fills,
            pix_w,
            pix_h,
            |x, y| tr.to_lonlat(x, y),
        )
    }
}

#[cfg(feature = "gpu")]
pub use burin_map_core::engine;

/// Download elevation for `bounds` and draw it with a single batch.
pub fn draw(
    bounds: &Bounds,
    style: &Style,
    frame: &Frame,
    progress: impl Fn(&Progress) + Sync,
) -> Result<Map, Error> {
    draw_batches(bounds, &[Batch::along(style.clone())], frame, progress)
}

/// Download elevation and draw with one or more batch passes (see `Batch`).
pub fn draw_batches(
    bounds: &Bounds,
    batches: &[Batch],
    frame: &Frame,
    progress: impl Fn(&Progress) + Sync,
) -> Result<Map, Error> {
    draw_with_water(bounds, batches, None, frame, progress)
}

/// [`draw_batches`], and with `water` the water from OpenStreetMap too: it
/// decides where the land is, and its batches draw on it.
pub fn draw_with_water(
    bounds: &Bounds,
    batches: &[Batch],
    water: Option<&WaterDoc>,
    frame: &Frame,
    progress: impl Fn(&Progress) + Sync,
) -> Result<Map, Error> {
    use std::time::Instant;
    assert!(!batches.is_empty());
    let style = &batches[0].style;
    let mut width = frame.px;
    let mut mm_per_px = None;
    let mut px_per_mm = 1.0; // Default: 1 px/mm
    if let Some([fw, fh]) = frame.size_mm {
        let k = frame.pen_width / style.batch_stroke as f64;
        let width_mm = fw.min(fh / projected_aspect(bounds));
        width = (width_mm / k).round() as usize;
        mm_per_px = Some(k);
        px_per_mm = 1.0 / k;
    }
    let dem = dem::load_dem_with(bounds, width, &progress)?;
    let (w, h) = (dem.width, dem.height);
    if let Some(k) = mm_per_px {
        progress(&Progress::Frame {
            width_mm: w as f64 * k,
            height_mm: h as f64 * k,
            scale: dem.px_m * 1000.0 / k,
            pen_width: frame.pen_width,
            pitch_mm: style.space as f64 * k,
        });
    }

    let coverage = match water {
        Some(doc) => Some(water::load_water(bounds, &dem, doc, &progress)?),
        None => None,
    };
    let layer = water
        .zip(coverage.as_deref())
        .map(|(doc, c)| burin_map_core::WaterLayer {
            coverage: c,
            depth: None,
            depth_override: None,
            shoreline: doc.shoreline.as_ref(),
            batches: &doc.batches,
        });

    struct Steps<'a>(&'a (dyn Fn(&Progress) + Sync));
    impl burin_map_core::Progress for Steps<'_> {
        fn reach(&self, step: burin_map_core::Step, stage: burin_map_core::Stage) {
            (self.0)(&Progress::Step { step, stage });
        }
    }
    let t = Instant::now();
    let drawing = burin_map_core::draw_batches_auto_told(
        &dem.elev,
        w,
        h,
        dem.px_m as f32,
        batches,
        layer.as_ref(),
        &Steps(&progress),
        px_per_mm as f32,
    );
    let secs = t.elapsed().as_secs_f64();
    progress(&Progress::Relief { secs: secs * 0.3 });
    progress(&Progress::Lines {
        count: drawing.lines.len(),
        secs: secs * 0.5,
    });
    progress(&Progress::Dashes {
        count: drawing.paths.len(),
        secs: secs * 0.2,
    });
    Ok(Map {
        drawing,
        style: style.clone(),
        px_m: dem.px_m,
        mm_per_px,
    })
}
