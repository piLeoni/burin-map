//! What `draw` reports while it works. The command line shows these in
//! colour; `Display` gives plain one-line text (used by the Python `log`).

use std::fmt;

#[derive(Clone, Copy, Debug)]
pub enum TileSource {
    Cache,
    /// Downloaded, compressed size in bytes.
    Download(u64),
    /// No tile exists there: open sea.
    Sea,
}

#[derive(Clone, Debug)]
pub enum Progress<'a> {
    /// One elevation tile ready; `done` of `total`.
    Tile { name: &'a str, source: TileSource, done: usize, total: usize },
    /// Elevation mosaicked and reprojected onto the output grid.
    Elevation {
        tiles: usize,
        downloaded: usize,
        bytes: u64,
        width: usize,
        height: usize,
        px_m: f64,
        highest_m: f32,
        centre: (f64, f64),
        secs: f64,
    },
    /// Sized for paper.
    Page { width_mm: f64, height_mm: f64, scale: f64, pen_mm: f64, pitch_mm: f64 },
    /// Shading, slope and line directions computed.
    Relief { secs: f64 },
    Lines { count: usize, secs: f64 },
    Dashes { count: usize, secs: f64 },
    /// Plotter job: pen-down length, pen-up moves, both in mm.
    Plot { strokes: usize, ink_mm: f64, travel_mm: f64 },
}

/// "1,234,567".
pub fn thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}

/// "1:100,000": three significant figures, since whole pixels make the exact
/// value drift a little from the one asked for (1:100,021).
pub fn scale(n: f64) -> String {
    let mag = 10f64.powi(n.log10().floor() as i32 - 2).max(1.0);
    format!("1:{}", thousands(((n / mag).round() * mag) as u64))
}

/// "850 m", "12.4 km".
pub fn distance(mm: f64) -> String {
    let m = mm / 1000.0;
    if m < 1000.0 { format!("{m:.0} m") } else { format!("{:.1} km", m / 1000.0) }
}

pub fn megabytes(bytes: u64) -> String {
    format!("{:.1} MB", bytes as f64 / 1e6)
}

/// "46.95°N 7.45°E".
pub fn latlon(lat: f64, lon: f64) -> String {
    format!("{:.2}°{} {:.2}°{}", lat.abs(), if lat >= 0.0 { 'N' } else { 'S' },
            lon.abs(), if lon >= 0.0 { 'E' } else { 'W' })
}

impl fmt::Display for Progress<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Progress::Tile { name, source, done, total } => {
                let what = match source {
                    TileSource::Cache => "from cache".to_string(),
                    TileSource::Download(b) => format!("downloaded, {}", megabytes(b)),
                    TileSource::Sea => "open sea, none".to_string(),
                };
                write!(f, "elevation tile {name} {what} ({done}/{total})")
            }
            Progress::Elevation { tiles, downloaded, bytes, width, height, px_m, highest_m, centre, secs } => {
                write!(f, "elevation: {tiles} SRTM tile{}", if tiles == 1 { "" } else { "s" })?;
                if downloaded > 0 {
                    write!(f, " ({downloaded} downloaded, {})", megabytes(bytes))?;
                }
                write!(f, ", projected around {} onto {width} × {height} px, {px_m:.0} m per px, \
                           highest point {highest_m:.0} m ({secs:.1} s)", latlon(centre.1, centre.0))
            }
            Progress::Page { width_mm, height_mm, scale, pen_mm, pitch_mm } => write!(
                f, "page: {width_mm:.0} × {height_mm:.0} mm at {}, {pen_mm} mm pen, lines {pitch_mm:.2} mm apart",
                self::scale(scale)),
            Progress::Relief { secs } => write!(f, "relief: shading and line directions ({secs:.1} s)"),
            Progress::Lines { count, secs } => write!(f, "lines: {} traced ({secs:.1} s)", thousands(count as u64)),
            Progress::Dashes { count, secs } => {
                write!(f, "strokes: {} dashes, ordered for the pen ({secs:.1} s)", thousands(count as u64))
            }
            Progress::Plot { strokes, ink_mm, travel_mm } => write!(
                f, "plotter: {} strokes, {} of drawn line, {} of pen-up moves between strokes",
                thousands(strokes as u64), distance(ink_mm), distance(travel_mm)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats() {
        assert_eq!(thousands(1234567), "1,234,567");
        assert_eq!(thousands(999), "999");
        assert_eq!(distance(200_200.0), "200 m");
        assert_eq!(distance(12_400_000.0), "12.4 km");
        assert_eq!(latlon(46.95, -7.45), "46.95°N 7.45°W");
        assert_eq!(scale(100_021.0), "1:100,000");
        assert_eq!(scale(67_540.0), "1:67,500");
        assert_eq!(scale(250.0), "1:250");
    }
}
