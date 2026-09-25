//! Where to draw: addresses and coordinates to bounds.

use crate::dem::Bounds;
use crate::Error;

const NOMINATIM: &str = "https://nominatim.openstreetmap.org/search";

/// Parse "lat,lon" or "lat lon".
fn parse_latlon(q: &str) -> Option<(f64, f64)> {
    let mut it = q.split(|c: char| c == ',' || c.is_whitespace()).filter(|s| !s.is_empty());
    let (a, b) = (it.next()?, it.next()?);
    if it.next().is_some() {
        return None;
    }
    let (lat, lon) = (a.parse::<f64>().ok()?, b.parse::<f64>().ok()?);
    ((-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon)).then_some((lat, lon))
}

/// (lat, lon, label) for "37.77, -122.42" or any address or place name.
/// Names go to OpenStreetMap's Nominatim (one request, no key needed).
pub fn locate(query: &str) -> Result<(f64, f64, String), Error> {
    if let Some((lat, lon)) = parse_latlon(query) {
        return Ok((lat, lon, format!("{lat:.5}, {lon:.5}")));
    }
    let hits: serde_json::Value = ureq::get(NOMINATIM)
        .query("q", query)
        .query("format", "json")
        .query("limit", "1")
        .set("User-Agent", "burin (terrain drawing tool)")
        .call()
        .map_err(|e| Error::Net(format!("geocoding: {e}")))?
        .into_json()
        .map_err(|e| Error::Net(format!("geocoding: {e}")))?;
    let hit = hits.get(0).ok_or_else(|| Error::Input(format!("nothing found for {query:?}")))?;
    let num = |k: &str| hit[k].as_str().and_then(|s| s.parse::<f64>().ok());
    match (num("lat"), num("lon")) {
        (Some(lat), Some(lon)) => Ok((lat, lon, hit["display_name"].as_str().unwrap_or(query).to_string())),
        _ => Err(Error::Data("geocoding: malformed reply".into())),
    }
}

/// Lon/lat box of roughly width × height km centred on (lat, lon).
pub fn bounds_around(lat: f64, lon: f64, width_km: f64, height_km: f64) -> Bounds {
    let dlat = height_km / 2.0 / 111.32;
    let dlon = width_km / 2.0 / (111.32 * lat.to_radians().cos());
    [lon - dlon, lat - dlat, lon + dlon, lat + dlat]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinates_parse() {
        assert_eq!(parse_latlon("37.75,-122.45"), Some((37.75, -122.45)));
        assert_eq!(parse_latlon(" -33.9 151.2 "), Some((-33.9, 151.2)));
        assert_eq!(parse_latlon("Twin Peaks"), None);
        assert_eq!(parse_latlon("91,0"), None);
    }
}
