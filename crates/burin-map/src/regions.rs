//! Regions in lon/lat for a document drawn over an area: projected onto the
//! elevation grid the way the water is, then handed to the engine, which
//! eases the batches that name them out at their edge.

use serde_json::Value;

use crate::Error;
pub use burin_map_core::{Region, Ring};

/// An area a document's batches keep to or give way to, as the program
/// drawing the map has it.
#[derive(Clone, Debug, PartialEq)]
pub enum GeoRegion {
    /// Polygons in lon/lat degrees, each a list of rings: the first its
    /// outside, the rest holes, as GeoJSON orders them.
    Polygons(Vec<Vec<Vec<[f64; 2]>>>),
    /// Already on the elevation grid the document is drawn on.
    Pixels(Region),
}

impl GeoRegion {
    /// The polygons of a GeoJSON object: a Polygon or MultiPolygon, or a
    /// Feature, FeatureCollection or GeometryCollection of them. Points and
    /// lines are refused; features without a geometry are skipped.
    pub fn from_geojson(v: &Value) -> Result<GeoRegion, Error> {
        let mut out = Vec::new();
        polygons_of(v, &mut out)?;
        Ok(GeoRegion::Polygons(out))
    }

    /// On the grid, with `project` taking lon, lat to pixels.
    pub(crate) fn on_grid(&self, project: &impl Fn(f64, f64) -> [f64; 2]) -> Region {
        match self {
            GeoRegion::Pixels(r) => r.clone(),
            GeoRegion::Polygons(polys) => Region::Rings(
                polys
                    .iter()
                    .flat_map(|rings| rings.iter().enumerate())
                    .map(|(k, ring)| Ring {
                        exterior: k == 0,
                        points: ring.iter().map(|&[lon, lat]| project(lon, lat)).collect(),
                    })
                    .collect(),
            ),
        }
    }
}

/// Regions as the Python and Node packages send them: an object of GeoJSON
/// objects by name.
pub fn regions_from_json(v: &Value) -> Result<Vec<(String, GeoRegion)>, Error> {
    match v {
        Value::Null => Ok(Vec::new()),
        Value::Object(m) => m
            .iter()
            .map(|(name, g)| {
                GeoRegion::from_geojson(g)
                    .map(|r| (name.clone(), r))
                    .map_err(|e| Error::Input(format!("region {name:?}: {e}")))
            })
            .collect(),
        _ => Err(Error::Input("regions are an object of GeoJSON polygons by name".into())),
    }
}

fn polygons_of(v: &Value, out: &mut Vec<Vec<Vec<[f64; 2]>>>) -> Result<(), Error> {
    let kind = v.get("type").and_then(Value::as_str).unwrap_or("");
    let coords = || v.get("coordinates").unwrap_or(&Value::Null);
    match kind {
        "Polygon" => out.push(polygon(coords())?),
        "MultiPolygon" => {
            for p in list(coords(), "MultiPolygon coordinates")? {
                out.push(polygon(p)?);
            }
        }
        "Feature" => match v.get("geometry") {
            None | Some(Value::Null) => {}
            Some(g) => polygons_of(g, out)?,
        },
        "FeatureCollection" => {
            for f in list(v.get("features").unwrap_or(&Value::Null), "features")? {
                polygons_of(f, out)?;
            }
        }
        "GeometryCollection" => {
            for g in list(v.get("geometries").unwrap_or(&Value::Null), "geometries")? {
                polygons_of(g, out)?;
            }
        }
        "" => return Err(Error::Input("not GeoJSON: no \"type\"".into())),
        other => return Err(Error::Input(format!("a {other} has no area; regions are polygons"))),
    }
    Ok(())
}

fn list<'a>(v: &'a Value, what: &str) -> Result<&'a Vec<Value>, Error> {
    v.as_array().ok_or_else(|| Error::Input(format!("{what} should be a list")))
}

fn polygon(v: &Value) -> Result<Vec<Vec<[f64; 2]>>, Error> {
    list(v, "Polygon coordinates")?
        .iter()
        .map(|ring| {
            list(ring, "a ring")?
                .iter()
                .map(|p| match p.as_array().map(Vec::as_slice) {
                    Some([lon, lat, ..]) => lon
                        .as_f64()
                        .zip(lat.as_f64())
                        .map(|(lon, lat)| [lon, lat])
                        .ok_or_else(|| Error::Input("positions are [lon, lat] numbers".into())),
                    _ => Err(Error::Input("positions are [lon, lat] numbers".into())),
                })
                .collect()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn polygons_from_every_kind_of_geojson() {
        let square = json!([[[0, 0], [1, 0], [1, 1], [0, 1], [0, 0]], [[0.2, 0.2], [0.2, 0.8], [0.8, 0.8], [0.2, 0.2]]]);
        let poly = json!({"type": "Polygon", "coordinates": square});
        let GeoRegion::Polygons(p) = GeoRegion::from_geojson(&poly).unwrap() else { panic!() };
        assert_eq!((p.len(), p[0].len()), (1, 2), "one polygon with a hole");

        let all = json!({"type": "FeatureCollection", "features": [
            {"type": "Feature", "geometry": poly, "properties": {}},
            {"type": "Feature", "geometry": {"type": "MultiPolygon", "coordinates": [square, square]}},
            {"type": "Feature", "geometry": null},
        ]});
        let GeoRegion::Polygons(p) = GeoRegion::from_geojson(&all).unwrap() else { panic!() };
        assert_eq!(p.len(), 3);

        let line = json!({"type": "LineString", "coordinates": [[0, 0], [1, 1]]});
        assert!(GeoRegion::from_geojson(&line).is_err());
        assert!(GeoRegion::from_geojson(&json!({"coordinates": []})).is_err());
    }

    #[test]
    fn the_first_ring_is_the_outside() {
        let r = GeoRegion::Polygons(vec![vec![
            vec![[0.0, 0.0], [2.0, 0.0], [2.0, 2.0]],
            vec![[0.5, 0.5], [1.0, 0.5], [1.0, 1.0]],
        ]]);
        let Region::Rings(rings) = r.on_grid(&|lon, lat| [lon * 10.0, lat * 10.0]) else { panic!() };
        assert!(rings[0].exterior && !rings[1].exterior);
        assert_eq!(rings[0].points[1], [20.0, 0.0]);
    }

    #[test]
    fn regions_by_name() {
        let v = json!({"park": {"type": "Polygon", "coordinates": [[[0, 0], [1, 0], [1, 1]]]}});
        let r = regions_from_json(&v).unwrap();
        assert_eq!(r[0].0, "park");
        assert!(regions_from_json(&json!(null)).unwrap().is_empty());
        assert!(regions_from_json(&json!([1])).is_err());
    }
}
