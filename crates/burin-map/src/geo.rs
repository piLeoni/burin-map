//! Where to draw: addresses and coordinates to bounds.

use crate::dem::Bounds;
use crate::Error;

const NOMINATIM: &str = "https://nominatim.openstreetmap.org/search";

/// Parse "lat,lon" or "lat lon".
fn parse_latlon(q: &str) -> Option<(f64, f64)> {
    let mut it = q
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty());
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
        .set("User-Agent", "burin-map (terrain drawing tool)")
        .call()
        .map_err(|e| Error::Net(format!("geocoding: {e}")))?
        .into_json()
        .map_err(|e| Error::Net(format!("geocoding: {e}")))?;
    let hit = hits
        .get(0)
        .ok_or_else(|| Error::Input(format!("nothing found for {query:?}")))?;
    let num = |k: &str| hit[k].as_str().and_then(|s| s.parse::<f64>().ok());
    match (num("lat"), num("lon")) {
        (Some(lat), Some(lon)) => Ok((
            lat,
            lon,
            hit["display_name"].as_str().unwrap_or(query).to_string(),
        )),
        _ => Err(Error::Data("geocoding: malformed reply".into())),
    }
}

/// Ground length in km from "20km", "800 m", "1.5mi" or a bare number (km).
pub fn parse_length_km(s: &str) -> Result<f64, Error> {
    let s = s.trim().to_ascii_lowercase();
    let split = s.find(|c: char| c.is_ascii_alphabetic()).unwrap_or(s.len());
    let (num, unit) = (s[..split].trim(), s[split..].trim());
    let v: f64 = num
        .parse()
        .map_err(|_| Error::Input(format!("bad length {s:?} (try 20km or 800m)")))?;
    let km = match unit {
        "" | "km" => v,
        "m" => v / 1000.0,
        "mi" => v * 1.609_344,
        _ => {
            return Err(Error::Input(format!(
                "unknown unit {unit:?} in {s:?} (km, m or mi)"
            )))
        }
    };
    if km > 0.0 {
        Ok(km)
    } else {
        Err(Error::Input(format!("length must be positive: {s:?}")))
    }
}

/// Scale denominator from "1:25000", "1:25,000" or "25000".
pub fn parse_scale(s: &str) -> Result<f64, Error> {
    let t: String = s
        .trim()
        .chars()
        .filter(|c| *c != ',' && *c != '_' && *c != ' ')
        .collect();
    let t = t.strip_prefix("1:").unwrap_or(&t);
    match t.parse::<f64>() {
        Ok(n) if n > 0.0 => Ok(n),
        _ => Err(Error::Input(format!("bad scale {s:?} (try 1:25000)"))),
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

    #[test]
    fn lengths_and_scales_parse() {
        assert_eq!(parse_length_km("20").unwrap(), 20.0);
        assert_eq!(parse_length_km("20km").unwrap(), 20.0);
        assert_eq!(parse_length_km("800 m").unwrap(), 0.8);
        assert!((parse_length_km("1mi").unwrap() - 1.609344).abs() < 1e-9);
        assert!(parse_length_km("20ft").is_err() && parse_length_km("-3").is_err());
        assert_eq!(parse_scale("1:25000").unwrap(), 25000.0);
        assert_eq!(parse_scale("1:25,000").unwrap(), 25000.0);
        assert_eq!(parse_scale("50000").unwrap(), 50000.0);
        assert!(parse_scale("1:0").is_err());
    }
}
