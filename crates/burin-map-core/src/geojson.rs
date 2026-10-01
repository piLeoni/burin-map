//! GeoJSON FeatureCollection export from a `Drawing`.
//!
//! Paths are in pixel coordinates. Callers supply a transform so each point
//! lands at its real-world lat/lon (the same transform that mapped terrain
//! to pixels — typically a transverse Mercator).

use crate::{Fill, Line};

/// Six booleans that pack the result into a pretty-printed JSON string.
/// All true for the simplest readable GeoJSON.
///
/// Written directly; the WASM build doesn't pull in serde_json for `to_string_pretty`.
pub fn to_geojson(
    paths: &[Line],
    width: f64,
    height: f64,
    px_to_lonlat: impl Fn(f64, f64) -> (f64, f64),
) -> String {
    to_geojson_with_fills(paths, &[], width, height, px_to_lonlat)
}

/// [`to_geojson`] with each fill after the lines as a MultiPolygon feature,
/// its colour and layer in the properties.
pub fn to_geojson_with_fills(
    paths: &[Line],
    fills: &[Fill],
    _width: f64,
    _height: f64,
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
            let _ = write!(
                s,
                "[{:.6},{:.6}]",
                (lon * 1e6).round() / 1e6,
                (lat * 1e6).round() / 1e6
            );
        }
        s.push(']');
    }

    s.push_str("]}}");
    for f in fills {
        let layer = f.pen.layer.as_deref().unwrap_or("");
        let _ = write!(
            s,
            r#",{{"type":"Feature","properties":{{"fill":{:?},"layer":{:?}}},"geometry":{{"type":"MultiPolygon","coordinates":["#,
            f.pen.color, layer
        );
        for (k, poly) in crate::fills::polygons(&f.rings).iter().enumerate() {
            if k > 0 {
                s.push(',');
            }
            s.push('[');
            for (j, r) in poly.iter().enumerate() {
                if j > 0 {
                    s.push(',');
                }
                let mut ll: Vec<(f64, f64)> = r
                    .iter()
                    .map(|q| {
                        let (lon, lat) = px_to_lonlat(q[0] as f64, q[1] as f64);
                        ((lon * 1e6).round() / 1e6, (lat * 1e6).round() / 1e6)
                    })
                    .collect();
                if ll.first() != ll.last() {
                    ll.push(ll[0]);
                }
                // Outer rings counter-clockwise, holes clockwise (RFC 7946).
                let area: f64 = ll
                    .windows(2)
                    .map(|w| w[0].0 * w[1].1 - w[1].0 * w[0].1)
                    .sum();
                if (area > 0.0) != (j == 0) {
                    ll.reverse();
                }
                s.push('[');
                for (i, (lon, lat)) in ll.iter().enumerate() {
                    if i > 0 {
                        s.push(',');
                    }
                    let _ = write!(s, "[{lon:.6},{lat:.6}]");
                }
                s.push(']');
            }
            s.push(']');
        }
        s.push_str("]}}");
    }
    s.push_str("]}");
    s
}
