//! Filled areas: the land, the sea, and bands of height or depth on either,
//! as closed rings for an even-odd fill. They are cut from the same water
//! coverage as the shoreline, so they meet it exactly.

use serde::{Deserialize, Serialize};

use crate::style::Ground;
use crate::{Line, Pen};

/// A filled area in a drawing: its pen (`fill` set), its rings, and how many
/// of the drawing's stroke runs lie beneath it.
#[derive(Clone, Debug, PartialEq)]
pub struct Fill {
    pub pen: Pen,
    pub rings: Vec<Line>,
    pub above: usize,
}

/// Which heights a fill covers, in metres; depths are metres below the
/// surface. Unset limits are open.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Band {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elev_min: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub elev_max: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth_min: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub depth_max: Option<f32>,
}

impl Band {
    pub fn is_open(&self) -> bool {
        *self == Band::default()
    }

    /// The band as heights: `(lowest, highest)`.
    fn heights(&self) -> (f32, f32) {
        let lo = [self.elev_min, self.depth_max.map(|d| -d)]
            .into_iter()
            .flatten()
            .fold(f32::NEG_INFINITY, f32::max);
        let hi = [self.elev_max, self.depth_min.map(|d| -d)]
            .into_iter()
            .flatten()
            .fold(f32::INFINITY, f32::min);
        (lo, hi)
    }
}

/// The rings of `ground` within `band`. `water` is the water coverage, 0…1;
/// without it the sea is wherever `elev` is at or below 0.5 m. `blur` softens
/// the band's edges, px; the coast is never blurred.
pub fn rings(
    ground: Ground,
    band: &Band,
    elev: &[f32],
    water: Option<&[f32]>,
    w: usize,
    h: usize,
    blur: f32,
) -> Vec<Line> {
    // Signed: above 0 inside. The coast term stays within ±0.5: past the
    // coast it alone decides, and short of it it is smaller than the band's
    // metres almost everywhere, so the coast lands where the shoreline does.
    let side = |i: usize| -> f32 {
        let land = match water {
            Some(c) => 0.5 - c[i],
            None => (elev[i] - 0.5).clamp(-0.5, 0.5),
        };
        if ground.is_land() {
            land
        } else {
            -land
        }
    };
    let (lo, hi) = band.heights();
    let soft;
    let e: &[f32] = if band.is_open() || blur <= 0.0 {
        elev
    } else {
        soft = crate::grid::blur(elev, w, h, blur);
        &soft
    };
    let inside: Vec<f32> = (0..w * h)
        .map(|i| match side(i) {
            s if s <= 0.0 => s,
            s => s.min(e[i] - lo).min(hi - e[i]),
        })
        .collect();
    closed_outlines(&inside, w, h)
}

/// Where `v` crosses 0 (inside above it), as closed rings in px; rings that
/// reach the edge are closed along it.
fn closed_outlines(v: &[f32], w: usize, h: usize) -> Vec<Line> {
    use std::collections::HashMap;
    let (pw, ph) = (w + 2, h + 2);
    // One pixel more all round, mirrored about 0 where inside so the
    // crossing falls on the frame, and outside where outside: every ring
    // then closes within the padded grid.
    let mut f = vec![0.0f32; pw * ph];
    for y in 0..ph {
        for x in 0..pw {
            let s = v[y.saturating_sub(1).min(h - 1) * w + x.saturating_sub(1).min(w - 1)];
            let edge = x == 0 || y == 0 || x == pw - 1 || y == ph - 1;
            f[y * pw + x] = if edge { -s.abs() } else { s };
        }
    }
    let inside = |i: usize| f[i] > 0.0;
    // Grid edges by id: 2·i joins sample i to the one right of it, 2·i + 1
    // to the one below. Pixel centres sit at +0.5, less the padding.
    let point = |id: usize| -> [f32; 2] {
        let (i, across) = (id / 2, id.is_multiple_of(2));
        let j = if across { i + 1 } else { i + pw };
        let t = f[i] / (f[i] - f[j]);
        let (x, y) = ((i % pw) as f32 - 0.5, (i / pw) as f32 - 0.5);
        let p = if across { [x + t, y] } else { [x, y + t] };
        [p[0].clamp(0.0, w as f32), p[1].clamp(0.0, h as f32)]
    };
    // Marching squares, undirected: each crossed edge is joined to the
    // crossed edges of the two cells either side of it, so the joins form
    // cycles whatever the values, and an even-odd fill needs no winding.
    let mut links: HashMap<usize, [usize; 2]> = HashMap::new();
    let mut join = |a: usize, b: usize| {
        for (from, to) in [(a, b), (b, a)] {
            let l = links.entry(from).or_insert([usize::MAX; 2]);
            l[(l[0] != usize::MAX) as usize] = to;
        }
    };
    for y in 0..ph - 1 {
        for x in 0..pw - 1 {
            let i = y * pw + x;
            let corners = [i, i + 1, i + 1 + pw, i + pw];
            let s = corners.map(inside);
            // Edge k runs between corners k and k + 1.
            let edges = [2 * i, 2 * (i + 1) + 1, 2 * (i + pw), 2 * i + 1];
            let crossed: Vec<usize> = (0..4).filter(|&k| s[k] != s[(k + 1) % 4]).collect();
            match crossed[..] {
                [a, b] => join(edges[a], edges[b]),
                [_, _, _, _] => {
                    let centre = corners.iter().map(|&c| f[c]).sum::<f32>() > 0.0;
                    // Cut off the corners unlike the centre.
                    for k in (0..4).filter(|&k| s[k] != centre) {
                        join(edges[(k + 3) % 4], edges[k]);
                    }
                }
                _ => {}
            }
        }
    }
    let mut rings = Vec::new();
    let mut done = std::collections::HashSet::new();
    let mut starts: Vec<usize> = links.keys().copied().collect();
    starts.sort_unstable();
    for start in starts {
        if !done.insert(start) {
            continue;
        }
        let mut ring = vec![point(start)];
        let (mut prev, mut cur) = (start, links[&start][0]);
        while cur != start && done.insert(cur) {
            ring.push(point(cur));
            let [a, b] = links[&cur];
            (prev, cur) = (cur, if a == prev { b } else { a });
        }
        ring.push(ring[0]);
        if ring.len() >= 4 {
            rings.push(ring);
        }
    }
    rings
}

/// Twice the signed area of a ring; its sign is its winding.
fn area2(r: &Line) -> f32 {
    r.iter()
        .zip(r.iter().cycle().skip(1))
        .map(|(a, b)| a[0] * b[1] - b[0] * a[1])
        .sum()
}

fn contains(r: &Line, p: [f32; 2]) -> bool {
    let mut inside = false;
    for (a, b) in r.iter().zip(r.iter().cycle().skip(1)) {
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < a[0] + (p[1] - a[1]) / (b[1] - a[1]) * (b[0] - a[0])
        {
            inside = !inside;
        }
    }
    inside
}

/// Rings grouped into polygons for GeoJSON: each outer ring followed by its
/// holes, nesting decided by containment as the even-odd fill reads it.
pub fn polygons(rings: &[Line]) -> Vec<Vec<&Line>> {
    let n = rings.len();
    // Largest first, so a ring's parent is already placed.
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by(|&a, &b| area2(&rings[b]).abs().total_cmp(&area2(&rings[a]).abs()));
    let mut depth = vec![0usize; n];
    let mut parent = vec![None; n];
    for (k, &i) in order.iter().enumerate() {
        let p = rings[i][0];
        if let Some(&j) = order[..k].iter().rev().find(|&&j| contains(&rings[j], p)) {
            depth[i] = depth[j] + 1;
            parent[i] = Some(j);
        }
    }
    let mut out: Vec<(usize, Vec<&Line>)> = Vec::new();
    for &i in &order {
        if depth[i].is_multiple_of(2) {
            out.push((i, vec![&rings[i]]));
        } else if let Some(poly) = parent[i].and_then(|j| out.iter_mut().find(|(o, _)| *o == j)) {
            poly.1.push(&rings[i]);
        }
    }
    out.into_iter().map(|(_, p)| p).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A square island 10…30 px in a 40 px sea, with a lake 15…20 px.
    fn island() -> (Vec<f32>, Vec<f32>) {
        let (w, h) = (40, 40);
        let mut water = vec![1.0f32; w * h];
        let mut elev = vec![-100.0f32; w * h];
        for y in 10..30 {
            for x in 10..30 {
                water[y * w + x] = 0.0;
                elev[y * w + x] = (x as f32 - 10.0) * 10.0;
            }
        }
        for y in 15..20 {
            for x in 15..20 {
                water[y * w + x] = 1.0;
            }
        }
        (water, elev)
    }

    fn total_area(rings: &[Line]) -> f32 {
        polygons(rings)
            .iter()
            .map(|p| {
                let outer = area2(p[0]).abs();
                outer - p[1..].iter().map(|r| area2(r).abs()).sum::<f32>()
            })
            .sum::<f32>()
            / 2.0
    }

    #[test]
    fn land_and_sea_share_the_coast() {
        let (water, elev) = island();
        let land = rings(
            Ground::Land,
            &Band::default(),
            &elev,
            Some(&water),
            40,
            40,
            0.0,
        );
        let sea = rings(
            Ground::Sea,
            &Band::default(),
            &elev,
            Some(&water),
            40,
            40,
            0.0,
        );
        let (a_land, a_sea) = (total_area(&land), total_area(&sea));
        assert!((a_land - (400.0 - 25.0)).abs() < 4.0, "land {a_land}");
        assert!(
            (a_land + a_sea - 1600.0).abs() < 1.0,
            "land {a_land} + sea {a_sea}"
        );
        let sea_polys = polygons(&sea);
        assert!(
            sea_polys.iter().any(|p| p.len() == 2),
            "the sea has the island as a hole"
        );
        assert!(
            sea.iter().flatten().any(|p| p[0] == 0.0) && sea.iter().flatten().any(|p| p[0] == 40.0)
        );
    }

    #[test]
    fn every_ring_closes() {
        let (w, h) = (37, 23);
        let mut seed = 7u64;
        let mut rand = || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 40) as f32 / (1u64 << 24) as f32
        };
        // Coarse noise, with plenty of exact halves as anti-aliased edges give.
        let water: Vec<f32> = (0..w * h)
            .map(|_| match rand() {
                r if r < 0.3 => 0.5,
                r => (r - 0.3) / 0.7,
            })
            .collect();
        let elev: Vec<f32> = (0..w * h).map(|i| (i % w) as f32 * 30.0 - 300.0).collect();
        for ground in [Ground::Land, Ground::Sea] {
            for band in [
                Band::default(),
                Band {
                    elev_min: Some(-100.0),
                    elev_max: Some(200.0),
                    ..Band::default()
                },
            ] {
                let r = rings(ground, &band, &elev, Some(&water), w, h, 0.0);
                assert!(!r.is_empty());
                for ring in &r {
                    assert_eq!(
                        ring.first(),
                        ring.last(),
                        "{ground:?} {band:?}: an open ring"
                    );
                }
            }
        }
    }

    #[test]
    fn a_band_keeps_to_its_heights() {
        let (water, elev) = island();
        let high = Band {
            elev_min: Some(100.0),
            ..Band::default()
        };
        let r = rings(Ground::Land, &high, &elev, Some(&water), 40, 40, 0.0);
        let a = total_area(&r);
        // From the 100 m pixel centre (x 20.5) to the coast (x 30), 20 rows.
        assert!((a - 190.0).abs() < 2.0, "{a}");
        assert!(r.iter().flatten().all(|p| p[0] >= 20.4));
        assert!(
            r.iter().flatten().any(|p| (p[0] - 30.0).abs() < 1e-3),
            "reaches the coast"
        );
    }
}
