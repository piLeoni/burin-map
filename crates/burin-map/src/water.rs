//! Water from OpenStreetMap vector tiles (OpenFreeMap by default), through
//! terramask, rasterised onto the elevation's grid.

use crate::dem::{projector, Bounds, Dem};
use crate::{Error, Progress, WaterDoc};

/// Fraction of each pixel of `dem`'s grid that is water.
pub fn load_water(
    b: &Bounds,
    dem: &Dem,
    doc: &WaterDoc,
    progress: &(dyn Fn(&Progress) + Sync),
) -> Result<Vec<f32>, Error> {
    let t0 = std::time::Instant::now();
    let filter = match &doc.classes {
        Some(classes) => terramask::Filter::water(classes, &[] as &[&str]),
        None => terramask::Filter::water(terramask::Filter::DEFAULT_AREAS, &[] as &[&str]),
    };
    let grid = terramask::Grid::new(widen(b), dem.width, dem.height);
    let fetcher = match &doc.source {
        Some(s) => terramask::Fetcher::with_source(s.clone()),
        None => terramask::Fetcher::new(),
    };
    let water = fetcher
        .features_for(
            &grid,
            &filter,
            &terramask::ZoomLimits::default(),
            |done, total| progress(&Progress::WaterTile { done, total }),
        )
        .map_err(|e| tm_error("water", e))?;
    let mask = water.mask_with(
        dem.width,
        dem.height,
        &terramask::MaskOptions::default(),
        projector(b, dem.width, dem.height),
    );
    progress(&Progress::Water {
        share: mask.fraction(),
        secs: t0.elapsed().as_secs_f64(),
    });
    Ok(mask.coverage)
}

/// The lon/lat box around a transverse Mercator rectangle, which is a
/// little wider than `b` at its corners.
pub(crate) fn widen(b: &Bounds) -> Bounds {
    let (dw, dh) = ((b[2] - b[0]) * 0.05, (b[3] - b[1]) * 0.05);
    [
        b[0] - dw,
        (b[1] - dh).max(-85.0),
        b[2] + dw,
        (b[3] + dh).min(85.0),
    ]
}

pub(crate) fn tm_error(what: &str, e: terramask::Error) -> Error {
    match e {
        terramask::Error::Net(m) => Error::Net(format!("{what}: {m}")),
        terramask::Error::Input(m) => Error::Input(format!("{what}: {m}")),
        terramask::Error::Data(m) => Error::Data(format!("{what}: {m}")),
    }
}
