//! Water from OpenStreetMap vector tiles (OpenFreeMap by default), through
//! watermask, rasterised onto the elevation's grid.

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
    let mut filter = watermask::Filter {
        lines: Vec::new(),
        ..Default::default()
    };
    if let Some(classes) = &doc.classes {
        filter.areas = classes.clone();
    }
    // The grid is a transverse Mercator rectangle, a little wider than the
    // lon/lat box at its corners.
    let (dw, dh) = ((b[2] - b[0]) * 0.05, (b[3] - b[1]) * 0.05);
    let wide = [
        b[0] - dw,
        (b[1] - dh).max(-85.0),
        b[2] + dw,
        (b[3] + dh).min(85.0),
    ];
    let grid = watermask::Grid::new(wide, dem.width, dem.height);
    let fetcher = match &doc.source {
        Some(s) => watermask::Fetcher::with_source(s.clone()),
        None => watermask::Fetcher::new(),
    };
    let water = fetcher
        .water_for(
            &grid,
            &filter,
            &watermask::ZoomLimits::default(),
            |done, total| progress(&Progress::WaterTile { done, total }),
        )
        .map_err(|e| match e {
            watermask::Error::Net(m) => Error::Net(format!("water: {m}")),
            watermask::Error::Input(m) => Error::Input(format!("water: {m}")),
            watermask::Error::Data(m) => Error::Data(format!("water: {m}")),
        })?;
    let mask = water.mask_with(
        dem.width,
        dem.height,
        &watermask::MaskOptions::default(),
        projector(b, dem.width, dem.height),
    );
    progress(&Progress::Water {
        share: mask.water_fraction(),
        secs: t0.elapsed().as_secs_f64(),
    });
    Ok(mask.coverage)
}
