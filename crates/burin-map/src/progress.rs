//! What `draw` reports while it works. The command line shows these in
//! colour; `Display` gives plain one-line text (used by the Python `log`).

use std::fmt;

#[derive(Clone, Debug)]
pub enum Progress {
    /// One terrain tile ready; `done` of `total`.
    Tile {
        done: usize,
        total: usize,
    },
    /// Terrain tiles of one zoom reprojected onto the output grid.
    Elevation {
        tiles: usize,
        zoom: u8,
        width: usize,
        height: usize,
        px_m: f64,
        highest_m: f32,
        /// The sea floor's lowest point, below 0; 0 without sea.
        deepest_m: f32,
        centre: (f64, f64),
        secs: f64,
    },
    /// One water tile ready; `done` of `total`.
    WaterTile {
        done: usize,
        total: usize,
    },
    /// Water rasterised onto the grid; `share` of it is water.
    Water {
        share: f64,
        secs: f64,
    },
    /// Frame with a physical size.
    Frame {
        width_mm: f64,
        height_mm: f64,
        scale: f64,
        pen_width: f64,
        pitch_mm: f64,
    },
    /// Shading, slope and line directions computed.
    Relief {
        secs: f64,
    },
    Lines {
        count: usize,
        secs: f64,
    },
    Dashes {
        count: usize,
        secs: f64,
    },
    /// A step of the drawing reached a stage, as the engine tells it.
    Step {
        step: burin_map_core::Step,
        stage: burin_map_core::Stage,
    },
}

pub use burin_map_core::panel::{latlon, relief, scale, thousands};

impl fmt::Display for Progress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Progress::Tile { done, total } => write!(f, "elevation tile {done}/{total}"),
            Progress::Elevation { tiles, zoom, width, height, px_m, highest_m, deepest_m, centre, secs } => {
                write!(f, "elevation: {tiles} terrain tile{} at zoom {zoom}", if tiles == 1 { "" } else { "s" })?;
                write!(f, ", projected around {} onto {width} × {height} px, {px_m:.0} m per px, {} ({secs:.1} s)",
                    latlon(centre.1, centre.0), relief(highest_m, deepest_m))
            }
            Progress::WaterTile { done, total } => write!(f, "water tile {done}/{total}"),
            Progress::Water { share, secs } => {
                write!(f, "water: OpenStreetMap, {:.0}% of the area ({secs:.1} s)", share * 100.0)
            }
            Progress::Frame { width_mm, height_mm, scale, pen_width, pitch_mm } => write!(
                f, "frame: {width_mm:.0} × {height_mm:.0} mm at {}, stroke {pen_width} mm, lines {pitch_mm:.2} mm apart",
                self::scale(scale)),
            Progress::Relief { secs } => write!(f, "relief: shading and line directions ({secs:.1} s)"),
            Progress::Lines { count, secs } => write!(f, "lines: {} traced ({secs:.1} s)", thousands(count as u64)),
            Progress::Dashes { count, secs } => {
                write!(f, "dashes: {} ({secs:.1} s)", thousands(count as u64))
            }
            Progress::Step { step, stage } => write!(f, "{step:?}: {stage:?}"),
        }
    }
}
