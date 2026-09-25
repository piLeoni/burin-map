//! Plotting order: greedy nearest neighbour over line ends, reversing lines
//! where that's shorter, to cut pen-up travel. Uses a uniform grid instead
//! of a k-d tree.

use crate::streamlines::Line;

pub fn plot_order(lines: Vec<Line>) -> Vec<Line> {
    let n = lines.len();
    if n < 2 {
        return lines;
    }
    // Endpoint e: line e % n, start if e < n else end.
    let ends: Vec<[f32; 2]> = lines.iter().map(|l| l[0])
        .chain(lines.iter().map(|l| *l.last().unwrap()))
        .collect();
    let (mut xmax, mut ymax) = (1.0f32, 1.0f32);
    for p in &ends {
        xmax = xmax.max(p[0]);
        ymax = ymax.max(p[1]);
    }
    let cell = ((xmax * ymax / n as f32).sqrt()).max(4.0);
    let (gw, gh) = ((xmax / cell) as usize + 1, (ymax / cell) as usize + 1);
    let mut buckets: Vec<Vec<u32>> = vec![Vec::new(); gw * gh];
    let cell_of = |p: [f32; 2]| ((p[0].max(0.0) / cell) as usize).min(gw - 1) + gw * ((p[1].max(0.0) / cell) as usize).min(gh - 1);
    for (e, &p) in ends.iter().enumerate() {
        buckets[cell_of(p)].push(e as u32);
    }

    let mut used = vec![false; n];
    let mut order: Vec<(usize, bool)> = Vec::with_capacity(n);
    let mut cur = [0.0f32, 0.0f32];
    for _ in 0..n {
        let (cx, cy) = (((cur[0].max(0.0) / cell) as isize).min(gw as isize - 1),
                        ((cur[1].max(0.0) / cell) as isize).min(gh as isize - 1));
        let mut best: Option<(f32, usize)> = None;
        for ring in 0..(gw.max(gh) as isize) {
            // Anything in a farther ring is at least (ring - 1) cells away.
            if let Some((d, _)) = best {
                if ((ring - 1).max(0) as f32 * cell).powi(2) > d {
                    break;
                }
            }
            for gy in cy - ring..=cy + ring {
                for gx in cx - ring..=cx + ring {
                    let on_ring = (gy - cy).abs() == ring || (gx - cx).abs() == ring;
                    if !on_ring || gx < 0 || gy < 0 || gx >= gw as isize || gy >= gh as isize {
                        continue;
                    }
                    let b = &mut buckets[gy as usize * gw + gx as usize];
                    b.retain(|&e| !used[e as usize % n]);
                    for &e in b.iter() {
                        let p = ends[e as usize];
                        let d = (p[0] - cur[0]).powi(2) + (p[1] - cur[1]).powi(2);
                        if best.map_or(true, |(bd, _)| d < bd) {
                            best = Some((d, e as usize));
                        }
                    }
                }
            }
        }
        let Some((_, e)) = best else { break };
        let li = e % n;
        used[li] = true;
        let reversed = e >= n;
        order.push((li, reversed));
        cur = if reversed { lines[li][0] } else { *lines[li].last().unwrap() };
    }

    let mut slots: Vec<Option<Line>> = lines.into_iter().map(Some).collect();
    order.into_iter().map(|(li, rev)| {
        let mut l = slots[li].take().unwrap();
        if rev {
            l.reverse();
        }
        l
    }).collect()
}

/// Total distance between the end of each path and the start of the next, px.
pub fn pen_up_travel(paths: &[Line]) -> f64 {
    paths.windows(2).map(|p| {
        let (a, b) = (p[0].last().unwrap(), p[1][0]);
        ((b[0] - a[0]) as f64).hypot((b[1] - a[1]) as f64)
    }).sum()
}
