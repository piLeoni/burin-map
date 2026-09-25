"""Evenly spaced streamlines (Jobard & Lefer, 1997) with variable spacing.

Lines are traced one at a time along the direction field. Each finished line
is stamped into two occupancy rasters:

- `occ_test` (radius sep × test_ratio): a line being traced stops on entering it;
- `occ_seed` (radius ~sep): new seeds must lie outside it.

New seeds are placed on both sides of every line, so the drawing grows
outwards from each line like ripples; a shuffled grid picks up regions the
ripples never reach.

Plain Jobard–Lefer seeds at exactly sep and needs ~sep free beyond it, so
gaps between 1 and 2 sep are never filled and read as bands. Here each seed
first measures the gap along the normal to the next line: wide gaps get a
seed at sep, gaps of at least 2 × fill_ratio × sep get one in the middle.
"""

from __future__ import annotations

import math
import random
from collections import deque

import numpy as np

from .fields import Fields
from .style import Style


def _bilinear(f: np.ndarray, x: float, y: float) -> float:
    x0, y0 = int(x), int(y)
    fx, fy = x - x0, y - y0
    r0, r1 = f[y0], f[y0 + 1]
    return ((r0[x0] * (1 - fx) + r0[x0 + 1] * fx) * (1 - fy)
            + (r1[x0] * (1 - fx) + r1[x0 + 1] * fx) * fy)


def _disk(r: float) -> tuple[np.ndarray, np.ndarray]:
    ri = int(math.ceil(r))
    yy, xx = np.mgrid[-ri:ri + 1, -ri:ri + 1]
    m = xx * xx + yy * yy <= r * r
    return yy[m], xx[m]


def trace_streamlines(f: Fields, st: Style) -> list[np.ndarray]:
    """Polylines (N×2 arrays of x, y in px). Closed loops repeat their first point."""
    tx, ty, walkable, sep = f.tx, f.ty, f.walkable, f.sep
    H, W = tx.shape
    occ_test = np.zeros((H, W), bool)
    occ_seed = np.zeros((H, W), bool)
    line_id = np.full((H, W), -1, np.int32)  # which line passes through each pixel
    disks: dict[float, tuple[np.ndarray, np.ndarray]] = {}
    max_steps = int(4 * (W + H) / st.step)

    def free(x: float, y: float, occ: np.ndarray) -> bool:
        if not (1 <= x < W - 2 and 1 <= y < H - 2):
            return False
        xi, yi = int(x + 0.5), int(y + 0.5)
        return walkable[yi, xi] and not occ[yi, xi]

    def direction(x: float, y: float, pu: float, pv: float):
        """Unit field direction at (x, y), flipped to agree with (pu, pv).
        The field is a line field (no preferred sign), hence the flip."""
        u, v = _bilinear(tx, x, y), _bilinear(ty, x, y)
        n = math.hypot(u, v)
        if n < 1e-6:
            return None
        u, v = u / n, v / n
        return (-u, -v) if u * pu + v * pv < 0 else (u, v)

    # The line being traced must keep its distance from itself too, or it
    # spirals tightly around peaks. Its points go in a coarse spatial hash,
    # indexed by signed arc position (backward half < 0) so that the points
    # just behind the tip don't count as collisions.
    cell = max(1.0, float(sep.max()))
    lag = int(math.ceil(2.0 * math.pi * sep.max() * st.test_ratio / st.step)) + 2

    def hits_itself(own: dict, x: float, y: float, k: int) -> bool:
        r2 = (sep[int(y), int(x)] * st.test_ratio) ** 2
        cx, cy = int(x // cell), int(y // cell)
        for gx in (cx - 1, cx, cx + 1):
            for gy in (cy - 1, cy, cy + 1):
                for j, px, py in own.get((gx, gy), ()):
                    if abs(k - j) > lag and (px - x) ** 2 + (py - y) ** 2 < r2:
                        return True
        return False

    def trace(x0: float, y0: float, sign: float, own: dict):
        """Follow the field from a seed in one direction (RK2).
        Returns (points, closed) where closed means it came back to the seed."""
        d = direction(x0, y0, 0.0, 0.0)
        if d is None:
            return [], False
        pu, pv = d[0] * sign, d[1] * sign
        x, y = x0, y0
        r_close = max(sep[int(y0), int(x0)] * st.test_ratio, 1.5 * st.step)
        pts = []
        for i in range(max_steps):
            d1 = direction(x, y, pu, pv)
            if d1 is None:
                break
            xm, ym = x + 0.5 * st.step * d1[0], y + 0.5 * st.step * d1[1]
            if not (1 <= xm < W - 2 and 1 <= ym < H - 2):
                break
            d2 = direction(xm, ym, d1[0], d1[1])
            if d2 is None:
                break
            x, y = x + st.step * d2[0], y + st.step * d2[1]
            pu, pv = d2
            if not free(x, y, occ_test):
                break
            if i > lag and (x - x0) ** 2 + (y - y0) ** 2 < r_close ** 2:
                return pts, True
            k = int(sign) * (i + 1)
            if hits_itself(own, x, y, k):
                break
            own.setdefault((int(x // cell), int(y // cell)), []).append((k, x, y))
            pts.append((x, y))
        return pts, False

    def stamp(line: np.ndarray, lid: int):
        xi = np.clip(np.rint(line[:, 0]).astype(np.int64), 0, W - 1)
        yi = np.clip(np.rint(line[:, 1]).astype(np.int64), 0, H - 1)
        # 4-connected so a probe stepping across the line can't slip through
        for dx, dy in ((0, 0), (1, 0), (-1, 0), (0, 1), (0, -1)):
            line_id[np.clip(yi + dy, 0, H - 1), np.clip(xi + dx, 0, W - 1)] = lid
        local = np.round(sep[yi, xi] * 20) / 20  # few distinct radii → few disk shapes
        for r in np.unique(local):
            m = local == r
            for ratio, occ in ((st.test_ratio, occ_test), (0.95, occ_seed)):
                key = r * ratio
                if key not in disks:
                    disks[key] = _disk(key)
                dy, dx = disks[key]
                yy = np.clip(yi[m, None] + dy[None, :], 0, H - 1)
                xx = np.clip(xi[m, None] + dx[None, :], 0, W - 1)
                occ[yy, xx] = True

    def gap_seed(bx: float, by: float, nx: float, ny: float, lid: int):
        """Seed beside line `lid` at (bx, by) along normal (nx, ny), or None.
        Measured when the seed is used, since other lines may have come since."""
        d = sep[min(int(by), H - 1), min(int(bx), W - 1)]
        s = 1.0
        while s < 2.0 * d:
            xi, yi = int(bx + nx * s + 0.5), int(by + ny * s + 0.5)
            if not (0 <= xi < W and 0 <= yi < H) or not walkable[yi, xi]:
                break  # coast or edge: not a neighbour, fill up to it
            j = line_id[yi, xi]
            if j >= 0 and j != lid:
                gap = s + 0.75  # stamp is ~1 px thick
                if gap < 2.0 * st.fill_ratio * d:
                    return None
                sx, sy = bx + nx * gap / 2, by + ny * gap / 2
                return (sx, sy) if free(sx, sy, occ_test) else None
            s += 0.5
        sx, sy = bx + nx * d, by + ny * d
        return (sx, sy) if free(sx, sy, occ_seed) else None

    rng = random.Random(st.seed)
    g = max(2, int(sep.min()))
    grid = [(x + 0.5, y + 0.5) for y in range(2, H - 2, g) for x in range(2, W - 2, g)]
    rng.shuffle(grid)
    grid_i = 0
    queue: deque[tuple[float, float, float, float, int]] = deque()
    lines: list[np.ndarray] = []
    seed_every = max(1, int(round(st.spacing / st.step)))

    while True:
        if queue:
            seed = gap_seed(*queue.popleft())
            if seed is None:
                continue
            sx, sy = seed
        else:
            while grid_i < len(grid) and not free(*grid[grid_i], occ_seed):
                grid_i += 1
            if grid_i >= len(grid):
                break
            sx, sy = grid[grid_i]
            grid_i += 1

        own = {(int(sx // cell), int(sy // cell)): [(0, sx, sy)]}
        fwd, closed = trace(sx, sy, 1.0, own)
        bwd = [] if closed else trace(sx, sy, -1.0, own)[0]
        pts = bwd[::-1] + [(sx, sy)] + fwd
        if closed:
            pts.append(pts[0])
        if len(pts) * st.step < st.min_len:
            continue
        line = np.asarray(pts)
        lid = len(lines)
        stamp(line, lid)
        lines.append(line)

        # Candidate seeds on both sides; placed by gap_seed when popped.
        seg = np.diff(line, axis=0)
        tang = seg / np.maximum(np.hypot(seg[:, 0], seg[:, 1])[:, None], 1e-9)
        for k in range(0, len(tang), seed_every):
            nx, ny = -tang[k, 1], tang[k, 0]
            px, py = line[k]
            queue.append((px, py, nx, ny, lid))
            queue.append((px, py, -nx, -ny, lid))
    return lines
