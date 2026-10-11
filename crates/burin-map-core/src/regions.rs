//! Regions: areas the program drawing the map hands in — parks, towns, a
//! land-use raster — that batches keep to (`within`) or give way to
//! (`outside`). A batch only names them; the areas come with the draw, on the
//! elevation grid's pixels. Near a region's edge the batch's shading eases
//! out, so its lines thin and open up the way the ink rules thin them on lit
//! ground, and with `mask_fray` they break into dashes and dots along a
//! ragged edge, as water lines do near the shore.

use std::collections::HashMap;

use rayon::prelude::*;

use crate::fray::fray_along;
use crate::rng::{noise_field, Rng};
use crate::style::{Batch, Style};
use crate::Line;

pub use terramask::Ring;

/// An area on the elevation grid's pixels (x right, y down, row 0 north), as
/// the program drawing the map has it.
#[derive(Clone, Debug, PartialEq)]
pub enum Region {
    /// Share of each pixel inside, 0…1, `width * height` values. Soft edges
    /// carry through.
    Coverage(Vec<f32>),
    /// Polygons in pixels: exterior rings and holes, filled as the water is.
    Rings(Vec<Ring>),
    /// Signed distance to the edge in px, positive inside, `width * height`
    /// values: for programs with exact geometry of their own.
    Distance(Vec<f32>),
}

/// The regions some batch names, resolved for one drawing: the signed
/// distance to the edge of each set of names a batch keeps to or gives way to.
pub(crate) struct Masks {
    width: usize,
    height: usize,
    px_m: f32,
    sets: HashMap<Vec<String>, Vec<f32>>,
}

/// One batch's view of its regions.
pub(crate) struct Mask<'a> {
    width: usize,
    height: usize,
    within: Option<&'a [f32]>,
    outside: Option<&'a [f32]>,
    start: f32,
    reach: f32,
    fade: f32,
    fray: f32,
}

/// A set of names as one key: sorted, each once.
fn key(names: &[String]) -> Vec<String> {
    let mut k = names.to_vec();
    k.sort();
    k.dedup();
    k
}

impl Masks {
    pub(crate) fn none() -> Masks {
        Masks {
            width: 0,
            height: 0,
            px_m: 1.0,
            sets: HashMap::new(),
        }
    }

    /// Check `regions` against what `batches` name, and work out the distance
    /// for every set of names in use. Regions named together are one area:
    /// their coverage is merged before the distance is taken, so where two
    /// parks touch there is no edge between them.
    pub(crate) fn resolve(
        regions: &[(&str, Region)],
        batches: &[Batch],
        width: usize,
        height: usize,
        px_m: f32,
    ) -> Result<Masks, String> {
        let n = width * height;
        for (i, (name, r)) in regions.iter().enumerate() {
            if regions[..i].iter().any(|(m, _)| m == name) {
                return Err(format!("region {name:?} is given twice"));
            }
            let len = match r {
                Region::Coverage(v) | Region::Distance(v) => Some(v.len()),
                Region::Rings(_) => None,
            };
            if let Some(len) = len.filter(|&l| l != n) {
                return Err(format!(
                    "region {name:?} has {len} values; the grid is {width} × {height} = {n}"
                ));
            }
        }
        let mut sets = HashMap::new();
        for (i, b) in batches.iter().enumerate() {
            for (names, how) in [(&b.within, "keeps to"), (&b.outside, "gives way to")] {
                if names.is_empty() {
                    continue;
                }
                let k = key(names);
                if sets.contains_key(&k) {
                    continue;
                }
                let mut given = Vec::with_capacity(k.len());
                for name in &k {
                    match regions.iter().find(|(m, _)| m == name) {
                        Some((_, r)) => given.push(r),
                        None => {
                            return Err(format!(
                                "batch {:?} {how} region {name:?}, which wasn't given",
                                b.label(i)
                            ))
                        }
                    }
                }
                sets.insert(k, distance(&given, width, height));
            }
        }
        Ok(Masks {
            width,
            height,
            px_m,
            sets,
        })
    }

    /// The regions batch `b` keeps to and gives way to, with its style's
    /// distances in px; `None` when it names none.
    pub(crate) fn of(&self, b: &Batch, st: &Style) -> Option<Mask<'_>> {
        if !b.is_masked() {
            return None;
        }
        let set = |names: &[String]| {
            (!names.is_empty()).then(|| self.sets[&key(names)].as_slice())
        };
        let px = |px: f32, m: f32| if m > 0.0 && self.px_m > 0.0 { m / self.px_m } else { px };
        Some(Mask {
            width: self.width,
            height: self.height,
            within: set(&b.within),
            outside: set(&b.outside),
            start: px(st.mask_start, st.mask_start_m),
            reach: px(st.mask_reach, st.mask_reach_m),
            fade: st.mask_fade,
            fray: px(st.mask_fray, st.mask_fray_m),
        })
    }
}

/// Signed distance to the edge of `regions` taken as one area, + inside.
fn distance(regions: &[&Region], w: usize, h: usize) -> Vec<f32> {
    let coverages: Vec<Vec<f32>> = regions
        .iter()
        .filter_map(|r| match r {
            Region::Coverage(c) => Some(c.clone()),
            Region::Rings(rings) => Some(terramask::Mask::from_rings(w, h, rings, 4).coverage),
            Region::Distance(_) => None,
        })
        .collect();
    let mut d = match coverages.split_first() {
        Some((first, rest)) => {
            let mut c = first.clone();
            for other in rest {
                c.par_iter_mut().zip(other).for_each(|(a, &b)| *a = a.max(b));
            }
            terramask::Mask {
                width: w,
                height: h,
                coverage: c,
            }
            .distance()
        }
        None => vec![f32::NEG_INFINITY; w * h],
    };
    for r in regions {
        if let Region::Distance(e) = r {
            d.par_iter_mut().zip(e).for_each(|(a, &b)| *a = a.max(b));
        }
    }
    d
}

impl Mask<'_> {
    /// Distance in px to the nearer edge the batch respects, positive where
    /// its lines may go: inside what it keeps to, away from what it gives way to.
    fn edge(&self, i: usize) -> f32 {
        let inside = self.within.map_or(f32::INFINITY, |d| d[i]);
        let away = self.outside.map_or(f32::INFINITY, |d| -d[i]);
        inside.min(away)
    }

    fn edge_at(&self, p: [f32; 2]) -> f32 {
        let x = (p[0].max(0.0) as usize).min(self.width - 1);
        let y = (p[1].max(0.0) as usize).min(self.height - 1);
        self.edge(y * self.width + x)
    }

    /// How much of the batch's shading is kept at pixel `i`: 0 nearer the
    /// edge than `mask_start`, rising over `mask_reach` to 1.
    pub(crate) fn weight(&self, i: usize) -> f32 {
        ease(self.edge(i), self.start, self.reach, self.fade)
    }

    /// `paths` breaking up over `mask_fray` px past `mask_start`, along an
    /// edge that wanders so they don't stop on a clean line. Unchanged with
    /// `mask_fray` 0.
    pub(crate) fn fray(&self, paths: Vec<Line>, st: &Style) -> Vec<Line> {
        let band = self.fray;
        if band <= 0.0 || self.width == 0 || self.height == 0 {
            return paths;
        }
        let mut rng = Rng::new(st.batch_seed ^ 0x5EA1);
        let wander = noise_field(self.width, self.height, band.max(8.0), &mut rng);
        let whole = |p: [f32; 2]| (self.edge_at(p) - self.start) / band + (wander(p) - 0.5) * 0.8;
        fray_along(paths, st, 0x5EA1, whole)
    }
}

/// 0 below `start`, then rising over `reach` to 1, shaped by `fade`.
fn ease(d: f32, start: f32, reach: f32, fade: f32) -> f32 {
    if d.is_nan() || d < start {
        return 0.0;
    }
    ((d - start) / reach.max(1e-3))
        .clamp(0.0, 1.0)
        .powf(fade.max(1e-3))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x0: f64, y0: f64, s: f64) -> Ring {
        Ring {
            exterior: true,
            points: vec![[x0, y0], [x0 + s, y0], [x0 + s, y0 + s], [x0, y0 + s]],
        }
    }

    fn batch() -> Batch {
        Batch::lines()
    }

    #[test]
    fn ease_rises_from_start_over_reach() {
        assert_eq!(ease(1.0, 2.0, 10.0, 1.0), 0.0);
        assert_eq!(ease(2.0, 2.0, 10.0, 1.0), 0.0);
        assert!((ease(7.0, 2.0, 10.0, 1.0) - 0.5).abs() < 1e-6);
        assert_eq!(ease(50.0, 2.0, 10.0, 1.0), 1.0);
        assert_eq!(ease(f32::INFINITY, 0.0, 10.0, 1.0), 1.0);
        assert_eq!(ease(f32::NEG_INFINITY, 0.0, 10.0, 1.0), 0.0);
        assert!(ease(7.0, 2.0, 10.0, 2.0) < ease(7.0, 2.0, 10.0, 1.0), "above 1 later");
    }

    #[test]
    fn within_and_outside_mirror_each_other() {
        let (w, h) = (40, 10);
        let left: Vec<f32> = (0..w * h).map(|i| ((i % w) < 20) as u8 as f32).collect();
        let regions = [("left", Region::Coverage(left))];
        let st = Style::default();
        let keep = batch().within("left");
        let away = batch().outside("left");
        let m = Masks::resolve(&regions, &[keep.clone(), away.clone()], w, h, 1.0).unwrap();
        let (a, b) = (m.of(&keep, &st).unwrap(), m.of(&away, &st).unwrap());
        let row = 5 * w;
        assert!(a.weight(row + 2) > 0.0 && b.weight(row + 2) == 0.0);
        assert!(a.weight(row + 37) == 0.0 && b.weight(row + 37) > 0.0);
        assert!(a.weight(row + 2) > a.weight(row + 18), "fading towards the edge");
    }

    #[test]
    fn metres_win_over_px() {
        let regions = [("all", Region::Coverage(vec![1.0; 16]))];
        let mut st = Style {
            mask_reach: 3.0,
            mask_reach_m: 200.0,
            ..Style::default()
        };
        let b = batch().within("all");
        let m = Masks::resolve(&regions, std::slice::from_ref(&b), 4, 4, 10.0).unwrap();
        assert_eq!(m.of(&b, &st).unwrap().reach, 20.0);
        st.mask_reach_m = 0.0;
        assert_eq!(m.of(&b, &st).unwrap().reach, 3.0);
    }

    #[test]
    fn regions_named_together_are_one_area() {
        // Two squares meeting edge to edge: no edge between them.
        let (w, h) = (40, 20);
        let regions = [
            ("a", Region::Rings(vec![square(0.0, 0.0, 20.0)])),
            ("b", Region::Rings(vec![square(20.0, 0.0, 20.0)])),
        ];
        let st = Style::default();
        let both = batch().within("b").within("a");
        let m = Masks::resolve(&regions, std::slice::from_ref(&both), w, h, 1.0).unwrap();
        let mask = m.of(&both, &st).unwrap();
        let seam = 10 * w + 20;
        assert!(mask.edge(seam) >= 9.0, "the seam is deep inside: {}", mask.edge(seam));
    }

    #[test]
    fn within_and_outside_together_take_the_nearer_edge() {
        let (w, h) = (60, 10);
        let park: Vec<f32> = vec![1.0; w * h];
        let town: Vec<f32> = (0..w * h).map(|i| ((i % w) >= 40) as u8 as f32).collect();
        let regions = [("park", Region::Coverage(park)), ("town", Region::Coverage(town))];
        let st = Style::default();
        let b = batch().within("park").outside("town");
        let m = Masks::resolve(&regions, std::slice::from_ref(&b), w, h, 1.0).unwrap();
        let mask = m.of(&b, &st).unwrap();
        let row = 5 * w;
        assert_eq!(mask.weight(row + 50), 0.0, "in town");
        assert!(mask.weight(row + 38) < mask.weight(row + 5), "fading towards town");
    }

    #[test]
    fn exact_distances_are_taken_as_given() {
        let d: Vec<f32> = (0..20).map(|x| 10.0 - x as f32).collect();
        let regions = [("ramp", Region::Distance(d))];
        let st = Style::default();
        let b = batch().within("ramp");
        let m = Masks::resolve(&regions, std::slice::from_ref(&b), 20, 1, 1.0).unwrap();
        assert_eq!(m.of(&b, &st).unwrap().edge(4), 6.0);
    }

    #[test]
    fn bad_regions_are_errors() {
        let b = batch().within("parks");
        let missing = Masks::resolve(&[], std::slice::from_ref(&b), 4, 4, 1.0);
        assert!(missing.err().unwrap().contains("\"parks\""));
        let short = [("parks", Region::Coverage(vec![1.0; 3]))];
        assert!(Masks::resolve(&short, std::slice::from_ref(&b), 4, 4, 1.0).is_err());
        let twice = [
            ("parks", Region::Coverage(vec![1.0; 16])),
            ("parks", Region::Coverage(vec![0.0; 16])),
        ];
        assert!(Masks::resolve(&twice, &[], 4, 4, 1.0).is_err());
    }

    #[test]
    fn fray_breaks_lines_near_the_edge_only() {
        let (w, h) = (200, 20);
        let left: Vec<f32> = (0..w * h).map(|i| ((i % w) < 100) as u8 as f32).collect();
        let regions = [("left", Region::Coverage(left))];
        let st = Style {
            mask_fray: 30.0,
            mask_reach: 1.0,
            ..Style::default()
        };
        let b = batch().within("left");
        let m = Masks::resolve(&regions, std::slice::from_ref(&b), w, h, 1.0).unwrap();
        let line: Line = (0..160).map(|x| [x as f32, 10.0]).collect();
        let out = m.of(&b, &st).unwrap().fray(vec![line], &st);
        assert!(out.len() > 1, "broken up");
        let reach = |l: &Line| l.iter().map(|p| p[0]).fold(0.0f32, f32::max);
        assert!(out.iter().all(|l| reach(l) < 100.0), "nothing past the edge");
        assert!(out.iter().any(|l| l[0][0] < 40.0 && l.len() > 30), "whole far inside");
    }
}
