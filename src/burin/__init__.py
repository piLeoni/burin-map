"""Pen-and-ink terrain maps from real elevation data.

    >>> import burin
    >>> lat, lon, _ = burin.locate("Twin Peaks, San Francisco")  # or "37.75,-122.45"
    >>> sf = burin.bounds_around(lat, lon, 20, 20)                # km
    >>> bay = (-122.75, 37.45, -122.05, 38.05)                 # west, south, east, north
    >>> d = burin.draw(bay, page_width_mm=420, pen_mm=0.3)  # plotter / print
    >>> d.save_svg("bay.svg"); d.save_png("bay.png")

Pipeline: DEM → contour-direction field + tone → evenly spaced streamlines
→ plotting order → tone-driven dashes → strokes.
"""

from __future__ import annotations

from dataclasses import dataclass, field, replace
from pathlib import Path
from typing import Callable

import numpy as np

from .dashes import dash_lines
from .dem import DEM, Bounds, load_dem, projected_aspect
from .fields import Fields, build_fields
from .geo import bounds_around, locate
from .render import pen_up_travel, plot_order, to_image, write_svg
from .streamlines import trace_streamlines
from .style import Style

__all__ = ["Drawing", "Style", "bounds_around", "draw", "draw_elevation", "load_dem", "locate"]
__version__ = "0.1.0"


@dataclass
class Drawing:
    paths: list[np.ndarray]  # dashes in drawing order, each an N×2 array of x, y in px
    width: int
    height: int
    style: Style
    mm_per_px: float | None = None  # physical size, when drawn for a page
    lines: list[np.ndarray] = field(repr=False, default_factory=list)  # undashed streamlines

    def image(self, supersample: int = 2):
        return to_image(self.paths, self.width, self.height, self.style.stroke, supersample)

    def save_png(self, path: str | Path, supersample: int = 2) -> None:
        self.image(supersample).save(path)

    def save_svg(self, path: str | Path) -> None:
        write_svg(self.paths, self.width, self.height, self.style.stroke, path, self.mm_per_px)


def draw_elevation(elev: np.ndarray, px_m: float, style: Style | None = None,
                   log: Callable[[str], None] = lambda _: None) -> Drawing:
    """Draw any elevation grid (metres, square pixels of `px_m` metres, row 0 = north).
    Values ≤ 0.5 m are treated as sea and left blank."""
    st = style or Style()
    f: Fields = build_fields(elev, px_m, st)
    lines = plot_order(trace_streamlines(f, st))
    log(f"{len(lines)} streamlines")
    paths = dash_lines(lines, f.tone, st)
    log(f"{len(paths)} dashes")
    H, W = elev.shape
    return Drawing(paths, W, H, st, lines=lines)


def draw(bounds: Bounds, style: Style | None = None, *,
         page_width_mm: float | None = None, page_height_mm: float | None = None,
         pen_mm: float = 0.3, log: Callable[[str], None] = lambda _: None) -> Drawing:
    """Download SRTM elevation for `bounds` (west, south, east, north in degrees) and draw it.

    With `page_width_mm` the drawing is sized for paper: the pen takes the place
    of `style.stroke`, so every proportion of the style is kept and the pixel
    width follows from page and pen. Height follows the area's aspect; with
    `page_height_mm` too, the drawing is shrunk to fit inside both.
    """
    st = style or Style()
    mm_per_px = None
    if page_width_mm:
        mm_per_px = pen_mm / st.stroke
        width_mm = page_width_mm
        if page_height_mm:
            width_mm = min(width_mm, page_height_mm / projected_aspect(bounds))
        st = replace(st, width=int(round(width_mm / mm_per_px)))
    dem: DEM = load_dem(bounds, st.width)
    H, W = dem.elev.shape
    log(f"DEM {W}x{H} px, {dem.px_m:.0f} m/px, max {dem.elev.max():.0f} m")
    if mm_per_px:
        log(f"page {W * mm_per_px:.0f}×{H * mm_per_px:.0f} mm, pen {pen_mm} mm, "
            f"lines every {st.spacing * mm_per_px:.2f} mm, scale 1:{dem.px_m * 1000 / mm_per_px:,.0f}")
    d = draw_elevation(dem.elev, dem.px_m, st, log)
    d.mm_per_px = mm_per_px
    if mm_per_px:
        log(f"pen-up travel {pen_up_travel(d.paths) * mm_per_px / 1000:.1f} m")
    return d
