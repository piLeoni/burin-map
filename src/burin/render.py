"""Output: raster PNG and vector SVG, black round-capped strokes on white."""

from __future__ import annotations

from pathlib import Path

import numpy as np
from PIL import Image, ImageDraw
from scipy.spatial import cKDTree


def plot_order(lines: list[np.ndarray]) -> list[np.ndarray]:
    """Greedy nearest-neighbour ordering, reversing lines where that's shorter,
    to cut pen-up travel on a plotter. Dashes cut from the result come out in
    drawing order."""
    n = len(lines)
    if n < 2:
        return list(lines)
    ends = np.array([l[0] for l in lines] + [l[-1] for l in lines])  # i: start, n+i: end
    tree = cKDTree(ends)
    used = np.zeros(n, bool)
    out: list[np.ndarray] = []
    cur = np.zeros(2)
    for _ in range(n):
        k = 8
        while True:
            _, idx = tree.query(cur, k=min(k, 2 * n))
            pick = next((i for i in np.atleast_1d(idx) if not used[i % n]), None)
            if pick is not None or k >= 2 * n:
                break
            k *= 4
        li = pick % n
        used[li] = True
        line = lines[li] if pick < n else lines[li][::-1]
        out.append(line)
        cur = line[-1]
    return out


def pen_up_travel(paths: list[np.ndarray]) -> float:
    """Total distance between the end of each path and the start of the next, px."""
    if len(paths) < 2:
        return 0.0
    a = np.array([p[-1] for p in paths[:-1]])
    b = np.array([p[0] for p in paths[1:]])
    return float(np.hypot(*(b - a).T).sum())


def to_image(paths: list[np.ndarray], width: int, height: int, stroke: float,
             supersample: int = 2) -> Image.Image:
    """Greyscale image; drawn at `supersample`× then downscaled for antialiasing."""
    k = supersample
    img = Image.new("L", (width * k, height * k), 255)
    dr = ImageDraw.Draw(img)
    sw = stroke * k
    r = sw / 2
    for p in paths:
        pts = [(float(x) * k, float(y) * k) for x, y in p]
        dr.line(pts, fill=0, width=max(1, int(round(sw))), joint="curve")
        for px, py in (pts[0], pts[-1]):  # PIL has no round caps
            dr.ellipse((px - r, py - r, px + r, py + r), fill=0)
    return img.resize((width, height), Image.Resampling.LANCZOS)


def write_svg(paths: list[np.ndarray], width: int, height: int, stroke: float,
              path: str | Path, mm_per_px: float | None = None) -> None:
    """With `mm_per_px` the document has a physical size in mm (for plotters and
    print layout); coordinates stay in px and the viewBox does the scaling."""
    if mm_per_px:
        size = f'width="{width * mm_per_px:.2f}mm" height="{height * mm_per_px:.2f}mm"'
    else:
        size = f'width="{width}" height="{height}"'
    with Path(path).open("w") as f:
        f.write(f'<svg xmlns="http://www.w3.org/2000/svg" {size} '
                f'viewBox="0 0 {width} {height}">'
                f'<g fill="none" stroke="#000" stroke-width="{stroke}" '
                f'stroke-linecap="round" stroke-linejoin="round">\n')
        for p in paths:
            f.write('<path d="M' + " L".join(f"{x:.2f} {y:.2f}" for x, y in p) + '"/>\n')
        f.write("</g></svg>\n")
