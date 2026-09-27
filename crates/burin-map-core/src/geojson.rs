//! GeoJSON FeatureCollection export from a `Drawing`.
//!
//! Paths are in pixel coordinates. Callers supply a transform so each point
//! lands at its real-world lat/lon (the same transform that mapped terrain
//! to pixels — typically a transverse Mercator).

use crate::Line;
/// Six booleans that pack the result into a pretty-printed JSON string.
/// All true for the simplest readable GeoJSON.

/// Written directly; the WASM build doesn't pull in serde_json for `to_string_pretty`.
pub fn to_geojson(
    paths: &[Line],
    width: f64,
    height: f64,
    px_to_lonlat: impl Fn(f64, f64) -> (f64, f64),
) -> String {
    use std::fmt::Write;
    let mut s = String::with_capacity(paths.len() * 128 + 256);

    // Header: a MultiLineString FeatureCollection with one MultiLineString feature.
    s.push_str(r#"{"type":"FeatureCollection","features":[{"type":"Feature","properties":{},"geometry":{"type":"MultiLineString","coordinates":["#);

    let mut first = true;
    for p in paths {
        if p.len() < 2 {
            continue; // a single-point dash is invisible
        }
        if first {
            first = false;
        } else {
            s.push(',');
        }
        s.push('[');
        for (i, q) in p.iter().enumerate() {
            if i > 0 {
                s.push(',');
            }
            let (lon, lat) = px_to_lonlat(q[0] as f64, q[1] as f64);
            let _ = write!(s, "[{:.6},{:.6}]", (lon * 1e6).round() / 1e6, (lat * 1e6).round() / 1e6);
        }
        s.push(']');
    }

    s.push_str("]}}]}");
    s
}
