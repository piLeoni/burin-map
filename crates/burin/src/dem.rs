//! Elevation data: SRTM tiles from AWS Terrain Tiles, reprojected to a
//! transverse Mercator centred on the area (conformal, scale exact at the
//! centre).
//!
//! Tiles are 1°×1°, 1 arc-second (~30 m), fetched on demand and cached
//! decimated to roughly the output resolution, in the same `.npy` files the
//! Python package uses. Set BURIN_CACHE to move the cache (default:
//! ~/.cache/burin).

use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::progress::{Progress, TileSource};
use crate::tmerc::TMerc;
use crate::Error;

const SKADI: &str = "https://s3.amazonaws.com/elevation-tiles-prod/skadi";
/// Metres per arc-second along a meridian.
const SRTM_PX_M: f64 = 30.9;
const TILE: usize = 3600;

/// West, south, east, north in degrees.
pub type Bounds = [f64; 4];

pub struct Dem {
    /// Metres, row 0 = north; sea and missing tiles are 0.
    pub elev: Vec<f32>,
    pub width: usize,
    pub height: usize,
    /// Metres per pixel.
    pub px_m: f64,
}

pub fn cache_dir() -> PathBuf {
    let d = std::env::var_os("BURIN_CACHE").map(PathBuf::from).unwrap_or_else(|| {
        let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")).unwrap_or_default();
        PathBuf::from(home).join(".cache").join("burin")
    });
    let _ = std::fs::create_dir_all(&d);
    d
}

/// Largest decimation that divides the tile and stays at or above output resolution.
fn stride_for(px_m: f64) -> usize {
    let want = ((px_m / SRTM_PX_M) as usize).max(1);
    (1..=want).rev().find(|d| TILE % d == 0).unwrap_or(1)
}

fn tile_name(lat: i32, lon: i32) -> (String, String) {
    let ns = if lat >= 0 { 'N' } else { 'S' };
    let ew = if lon >= 0 { 'E' } else { 'W' };
    (format!("{ns}{:02}", lat.abs()), format!("{ns}{:02}{ew}{:03}", lat.abs(), lon.abs()))
}

/// One SRTM tile decimated by `stride`, without its shared last row/col
/// (side = 3600 / stride). None over open sea.
fn tile(lat: i32, lon: i32, stride: usize) -> Result<(Option<Vec<f32>>, TileSource), Error> {
    let (dir, name) = tile_name(lat, lon);
    let cache = cache_dir();
    let cached = cache.join(format!("{name}_s{stride}.npy"));
    let missing = cache.join(format!("{name}.missing"));
    if let Ok(bytes) = std::fs::read(&cached) {
        if let Some(v) = npy::read_f32(&bytes, TILE / stride) {
            return Ok((Some(v), TileSource::Cache));
        }
    }
    if missing.exists() {
        return Ok((None, TileSource::Sea));
    }
    let url = format!("{SKADI}/{dir}/{name}.hgt.gz");
    let resp = match ureq::get(&url).set("User-Agent", "burin").call() {
        Ok(r) => r,
        Err(ureq::Error::Status(403 | 404, _)) => {
            let _ = std::fs::write(&missing, b"");
            return Ok((None, TileSource::Sea));
        }
        Err(e) => return Err(Error::Net(format!("{url}: {e}"))),
    };
    let mut gz = Vec::new();
    resp.into_reader().read_to_end(&mut gz).map_err(|e| Error::Net(format!("{url}: {e}")))?;
    let mut raw = Vec::new();
    flate2::read::GzDecoder::new(&gz[..]).read_to_end(&mut raw).map_err(|e| Error::Data(format!("{name}: {e}")))?;
    let n = ((raw.len() / 2) as f64).sqrt() as usize;
    if n * n * 2 != raw.len() || n < TILE {
        return Err(Error::Data(format!("{name}: unexpected size {} bytes", raw.len())));
    }
    // 1″ tiles are 3601², sharing edges with their neighbours.
    let side = TILE / stride;
    let step = (n - 1) / side;
    let mut out = Vec::with_capacity(side * side);
    for r in 0..side {
        for c in 0..side {
            let i = 2 * (r * step * n + c * step);
            let v = i16::from_be_bytes([raw[i], raw[i + 1]]) as f32;
            out.push(if v < -1000.0 { 0.0 } else { v }); // voids
        }
    }
    let _ = std::fs::write(&cached, npy::write_f32(&out, side));
    Ok((Some(out), TileSource::Download(gz.len() as u64)))
}

/// Projected extent (xmin, ymin, xmax, ymax) of the lon/lat box, densified
/// along its edges since they project as curves.
fn projected_extent(b: &Bounds, t: &TMerc) -> [f64; 4] {
    let [w, s, e, n] = *b;
    let mut ext = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
    const K: usize = 21;
    for i in 0..=K {
        let f = i as f64 / K as f64;
        let (lon, lat) = (w + (e - w) * f, s + (n - s) * f);
        for (x, y) in [t.forward(lon, s), t.forward(lon, n), t.forward(w, lat), t.forward(e, lat)] {
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
pub fn load_dem(b: &Bounds, width: usize) -> Result<Dem, Error> {
    load_dem_with(b, width, &|_| {})
}

/// `load_dem`, reporting each tile and the result to `progress`.
pub fn load_dem_with(b: &Bounds, width: usize, progress: &(dyn Fn(&Progress) + Sync)) -> Result<Dem, Error> {
    let t0 = std::time::Instant::now();
    let [w, s, e, n] = *b;
    if !(w < e && s < n && s >= -90.0 && n <= 90.0) {
        return Err(Error::Input(format!("bad bounds {b:?}")));
    }
    let t = centred(b);
    let [xmin, ymin, xmax, ymax] = projected_extent(b, &t);
    let px_m = (xmax - xmin) / width as f64;
    let height = ((ymax - ymin) / px_m).round().max(1.0) as usize;
    let stride = stride_for(px_m);

    // Mosaic of the tiles covering the box, north-west corner first.
    let lats: Vec<i32> = (s.floor() as i32..=n.floor() as i32).rev().collect();
    let lons: Vec<i32> = (w.floor() as i32..=e.floor() as i32).collect();
    let jobs: Vec<(i32, i32)> = lats.iter().flat_map(|&la| lons.iter().map(move |&lo| (la, lo))).collect();
    let tiles = fetch_all(&jobs, stride, progress)?;
    let side = TILE / stride;
    let (mw, mh) = (lons.len() * side, lats.len() * side);
    let mut mosaic = vec![0.0f32; mw * mh];
    for (k, (t, _)) in tiles.iter().enumerate() {
        if let Some(t) = t {
            let (ty, tx) = (k / lons.len(), k % lons.len());
            for r in 0..side {
                let dst = (ty * side + r) * mw + tx * side;
                mosaic[dst..dst + side].copy_from_slice(&t[r * side..(r + 1) * side]);
            }
        }
    }
    let (west, north) = (lons[0] as f64, lats[0] as f64 + 1.0);
    let deg_px = 1.0 / side as f64;

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
    for yy in 0..height {
        let (gy, fy) = (yy / G, (yy % G) as f64 / G as f64);
        for xx in 0..width {
            let (gx, fx) = (xx / G, (xx % G) as f64 / G as f64);
            let lerp = |a: (f64, f64), b: (f64, f64), f: f64| (a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f);
            let top = lerp(ll[gy * gw + gx], ll[gy * gw + gx + 1], fx);
            let bot = lerp(ll[(gy + 1) * gw + gx], ll[(gy + 1) * gw + gx + 1], fx);
            let (lon, lat) = lerp(top, bot, fy);
            // Mosaic pixel centres sit at +0.5.
            let c = (lon - west) / deg_px - 0.5;
            let r = (north - lat) / deg_px - 0.5;
            elev[yy * width + xx] = bilinear(&mosaic, mw, mh, c, r);
        }
    }
    let downloads: Vec<u64> = tiles.iter()
        .filter_map(|(_, s)| if let TileSource::Download(b) = s { Some(*b) } else { None })
        .collect();
    progress(&Progress::Elevation {
        tiles: tiles.len(),
        downloaded: downloads.len(),
        bytes: downloads.iter().sum(),
        width,
        height,
        px_m,
        highest_m: elev.iter().copied().fold(0.0, f32::max),
        centre: ((w + e) / 2.0, (s + n) / 2.0),
        secs: t0.elapsed().as_secs_f64(),
    });
    Ok(Dem { elev, width, height, px_m })
}

fn bilinear(g: &[f32], w: usize, h: usize, x: f64, y: f64) -> f32 {
    if x < -0.5 || y < -0.5 || x > w as f64 - 0.5 || y > h as f64 - 0.5 {
        return 0.0;
    }
    let (x, y) = (x.clamp(0.0, (w - 1) as f64), y.clamp(0.0, (h - 1) as f64));
    let (x0, y0) = (x as usize, y as usize);
    let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
    let (fx, fy) = ((x - x0 as f64) as f32, (y - y0 as f64) as f32);
    let top = g[y0 * w + x0] * (1.0 - fx) + g[y0 * w + x1] * fx;
    let bot = g[y1 * w + x0] * (1.0 - fx) + g[y1 * w + x1] * fx;
    top * (1.0 - fy) + bot * fy
}

type TileResult = Result<(Option<Vec<f32>>, TileSource), Error>;

/// Fetch tiles with a few parallel downloads, in `jobs` order.
fn fetch_all(jobs: &[(i32, i32)], stride: usize, progress: &(dyn Fn(&Progress) + Sync))
    -> Result<Vec<(Option<Vec<f32>>, TileSource)>, Error> {
    let next = AtomicUsize::new(0);
    let done = std::sync::Mutex::new(0usize);
    let mut results: Vec<Option<TileResult>> = (0..jobs.len()).map(|_| None).collect();
    let slots: Vec<std::sync::Mutex<&mut Option<_>>> = results.iter_mut().map(std::sync::Mutex::new).collect();
    std::thread::scope(|sc| {
        for _ in 0..jobs.len().min(8) {
            sc.spawn(|| loop {
                let k = next.fetch_add(1, Ordering::Relaxed);
                if k >= jobs.len() {
                    break;
                }
                let r = tile(jobs[k].0, jobs[k].1, stride);
                if let Ok((_, source)) = &r {
                    // Under the lock so reports arrive in count order.
                    let mut d = done.lock().unwrap();
                    *d += 1;
                    let name = tile_name(jobs[k].0, jobs[k].1).1;
                    progress(&Progress::Tile { name: &name, source: *source, done: *d, total: jobs.len() });
                }
                **slots[k].lock().unwrap() = Some(r);
            });
        }
    });
    drop(slots);
    results.into_iter().map(|r| r.expect("every job ran")).collect()
}

/// Minimal .npy reader/writer for square little-endian f32 arrays.
mod npy {
    pub fn read_f32(b: &[u8], side: usize) -> Option<Vec<f32>> {
        if b.len() < 10 || &b[..6] != b"\x93NUMPY" {
            return None;
        }
        let (hlen, start) = match b[6] {
            1 => (u16::from_le_bytes([b[8], b[9]]) as usize, 10),
            _ => (u32::from_le_bytes([b[8], b[9], b[10], b[11]]) as usize, 12),
        };
        let header = std::str::from_utf8(b.get(start..start + hlen)?).ok()?;
        let shape = format!("({side}, {side})");
        if !header.contains("'<f4'") || !header.contains("'fortran_order': False") || !header.contains(&shape) {
            return None;
        }
        let data = b.get(start + hlen..)?;
        if data.len() != side * side * 4 {
            return None;
        }
        Some(data.chunks_exact(4).map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect())
    }

    pub fn write_f32(v: &[f32], side: usize) -> Vec<u8> {
        let mut header = format!("{{'descr': '<f4', 'fortran_order': False, 'shape': ({side}, {side}), }}");
        // Pad so the data starts on a 64-byte boundary, header ends in '\n'.
        while (10 + header.len() + 1) % 64 != 0 {
            header.push(' ');
        }
        header.push('\n');
        let mut out = Vec::with_capacity(10 + header.len() + v.len() * 4);
        out.extend_from_slice(b"\x93NUMPY\x01\x00");
        out.extend_from_slice(&(header.len() as u16).to_le_bytes());
        out.extend_from_slice(header.as_bytes());
        for x in v {
            out.extend_from_slice(&x.to_le_bytes());
        }
        out
    }

    #[cfg(test)]
    #[test]
    fn round_trip() {
        let v: Vec<f32> = (0..16).map(|i| i as f32 * 1.5).collect();
        assert_eq!(read_f32(&write_f32(&v, 4), 4), Some(v));
    }
}
