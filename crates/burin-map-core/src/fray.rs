//! Lines breaking up towards an edge: shorter and shorter dashes, then dots,
//! where `whole` falls from 1 to 0. Water uses it near the shore and at depth
//! limits; land batches at the edge of the regions they keep to.

use rayon::prelude::*;

use crate::rng::Rng;
use crate::style::Style;
use crate::water::resample;
use crate::Line;

/// `paths` cut into dashes wherever `whole(p)` is below 1: about one dash per
/// 1.5 × the spacing, as long as `whole` allows, down to single dots, and none
/// at 0 or below. `salt` keeps the draws of different callers apart.
pub(crate) fn fray_along(
    paths: Vec<Line>,
    st: &Style,
    salt: u64,
    whole: impl Fn([f32; 2]) -> f32 + Send + Sync,
) -> Vec<Line> {
    let period = (st.space_base() * 1.5).max(3.0);
    paths
        .into_par_iter()
        .enumerate()
        .flat_map_iter(|(i, l)| {
            if l.len() < 2 || l.iter().all(|&p| whole(p) >= 1.0) {
                return vec![l];
            }
            let mut rng = Rng::new(st.batch_seed ^ salt ^ (i as u64).wrapping_mul(0x9E37_79B9));
            let p = resample(&l, 1.0);
            let mut out = Vec::new();
            let mut piece: Line = Vec::new();
            let (mut s, mut cell, mut ink) = (0.0f32, usize::MAX, (0.0f32, f32::MAX));
            for (k, &q) in p.iter().enumerate() {
                if k > 0 {
                    s += (q[0] - p[k - 1][0]).hypot(q[1] - p[k - 1][1]);
                }
                let c = (s / period) as usize;
                if c != cell {
                    cell = c;
                    let t = whole(q);
                    ink = if t >= 1.0 {
                        (0.0, f32::MAX)
                    } else {
                        let t = t.max(0.0);
                        let mut len = period * t.powf(0.8) * rng.uniform(0.6, 1.4) as f32;
                        if len < 1.0 {
                            len = if rng.uniform(0.0, 1.0) < (t * 1.5) as f64 {
                                1.0
                            } else {
                                0.0
                            };
                        }
                        let len = len.min(period);
                        let at = c as f32 * period + rng.uniform(0.0, (period - len) as f64) as f32;
                        (at, at + len)
                    };
                }
                if s >= ink.0 && s <= ink.1 {
                    piece.push(q);
                } else if !piece.is_empty() {
                    if piece.len() == 1 {
                        piece.push(q);
                    }
                    out.push(std::mem::take(&mut piece));
                }
            }
            if piece.len() >= 2 {
                out.push(piece);
            }
            out
        })
        .collect()
}
