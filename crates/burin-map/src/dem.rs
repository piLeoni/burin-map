//! Elevation data: Terrarium terrain tiles (AWS Open Data, through
//! terramask), land heights and the sea floor below zero, reprojected to a
//! transverse Mercator centred on the area (conformal, scale exact at the
//! centre).
//!
//! The zoom follows the output resolution, up to 15 (about 5 m per pixel at
//! the equator). Tiles are cached by terramask; set TERRAMASK_CACHE to move
//! the cache.

use crate::progress::Progress;
use crate::tmerc::TMerc;
use crate::water::{tm_error, widen};
use crate::Error;

/// West, south, east, north in degrees.
pub type Bounds = [f64; 4];

pub struct Dem {
    /// Metres, row 0 = north; the sea floor below 0, missing tiles 0.
    pub elev: Vec<f32>,
    pub width: usize,
    pub height: usize,
    /// Metres per pixel.
    pub px_m: f64,
}

/// Projected extent (xmin, ymin, xmax, ymax) of the lon/lat box, densified
/// along its edges since they project as curves.
fn projected_extent(b: &Bounds, t: &TMerc) -> [f64; 4] {
    let [w, s, e, n] = *b;
    let mut ext = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    const K: usize = 21;
    for i in 0..=K {
        let f = i as f64 / K as f64;
        let (lon, lat) = (w + (e - w) * f, s + (n - s) * f);
        for (x, y) in [
            t.forward(lon, s),
            t.forward(lon, n),
            t.forward(w, lat),
            t.forward(e, lat),
        ] {
            ext = [ext[0].min(x), ext[1].min(y), ext[2].max(x), ext[3].max(y)];
        }
    }
    ext
}

fn centred(b: &Bounds) -> TMerc {
    TMerc::new((b[0] + b[2]) / 2.0, (b[1] + b[3]) / 2.0)
}

/// Width and height of the area once projected, metres.
pub fn projected_size(b: &Bounds) -> (f64, f64) {
    let [x0, y0, x1, y1] = projected_extent(b, &centred(b));
    (x1 - x0, y1 - y0)
}

/// Height / width of the area once projected.
pub fn projected_aspect(b: &Bounds) -> f64 {
    let (w, h) = projected_size(b);
    h / w
}

/// Elevation for `bounds` on a grid `width` px wide with square metric pixels.
/// Lon/lat → position on the grid [`load_dem`] makes for `b` at this size,
/// x right and y down, pixel centres at +0.5.
pub fn projector(b: &Bounds, width: usize, height: usize) -> impl Fn(f64, f64) -> [f64; 2] {
    let t = centred(b);
    let [xmin, ymin, xmax, ymax] = projected_extent(b, &t);
    let (px, py) = ((xmax - xmin) / width as f64, (ymax - ymin) / height as f64);
    move |lon, lat| {
        let (x, y) = t.forward(lon, lat);
        [(x - xmin) / px, (ymax - y) / py]
    }
}

pub fn load_dem(b: &Bounds, width: usize) -> Result<Dem, Error> {
    load_dem_with(b, width, &|_| {})
}

/// `load_dem`, reporting each tile and the result to `progress`.
pub fn load_dem_with(
    b: &Bounds,
    width: usize,
    progress: &(dyn Fn(&Progress) + Sync),
) -> Result<Dem, Error> {
    let t0 = std::time::Instant::now();
    let [w, s, e, n] = *b;
    if !(w < e && s < n && s >= -90.0 && n <= 90.0) {
        return Err(Error::Input(format!("bad bounds {b:?}")));
    }
    let t = centred(b);
    let [xmin, ymin, xmax, ymax] = projected_extent(b, &t);
    let px_m = (xmax - xmin) / width as f64;
    let height = ((ymax - ymin) / px_m).round().max(1.0) as usize;

    let mut fetcher = terramask::Fetcher::new();
    fetcher.elevation_max_zoom = terramask::TERRARIUM_MAX_ZOOM;
    let terrain = fetcher
        .elevation_for(
            &terramask::Grid::new(widen(b), width, height),
            &terramask::ZoomLimits::default(),
            |done, total| progress(&Progress::Tile { done, total }),
        )
        .map_err(|e| tm_error("elevation", e))?
        .cubic(true);

    // Exact inverse projection on a coarse grid, bilinear in between: the
    // projection is smooth, so the error is far below a pixel.
    const G: usize = 16;
    let (gw, gh) = (width.div_ceil(G) + 1, height.div_ceil(G) + 1);
    let py_m = (ymax - ymin) / height as f64;
    let mut ll = vec![(0.0f64, 0.0f64); gw * gh];
    for gy in 0..gh {
        for gx in 0..gw {
            let x = xmin + ((gx * G) as f64 + 0.5) * px_m;
            let y = ymax - ((gy * G) as f64 + 0.5) * py_m;
            ll[gy * gw + gx] = t.inverse(x, y);
        }
    }

    let mut elev = vec![0.0f32; width * height];
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let rows_per = height.div_ceil(threads).max(1);
    std::thread::scope(|sc| {
        for (k, chunk) in elev.chunks_mut(rows_per * width).enumerate() {
            let (ll, terrain) = (&ll, &terrain);
            sc.spawn(move || {
                for (r, row) in chunk.chunks_mut(width).enumerate() {
                    let yy = k * rows_per + r;
                    let (gy, fy) = (yy / G, (yy % G) as f64 / G as f64);
                    for (xx, v) in row.iter_mut().enumerate() {
                        let (gx, fx) = (xx / G, (xx % G) as f64 / G as f64);
                        let lerp = |a: (f64, f64), b: (f64, f64), f: f64| {
                            (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f)
                        };
                        let top = lerp(ll[gy * gw + gx], ll[gy * gw + gx + 1], fx);
                        let bot = lerp(ll[(gy + 1) * gw + gx], ll[(gy + 1) * gw + gx + 1], fx);
                        let (lon, lat) = lerp(top, bot, fy);
                        let h = terrain.at(lon, lat);
                        *v = if h.is_finite() { h } else { 0.0 };
                    }
                }
            });
        }
    });
    progress(&Progress::Elevation {
        tiles: terrain.tile_count(),
        zoom: terrain.zoom(),
        width,
        height,
        px_m,
        highest_m: elev.iter().copied().fold(0.0, f32::max),
        deepest_m: elev.iter().copied().fold(0.0, f32::min),
        centre: ((w + e) / 2.0, (s + n) / 2.0),
        secs: t0.elapsed().as_secs_f64(),
    });
    Ok(Dem {
        elev,
        width,
        height,
        px_m,
    })
}
