"""Pen-and-ink terrain maps from real elevation data.

    >>> import burin_map
    >>> lat, lon, _ = burin_map.locate("Twin Peaks, San Francisco")  # or "37.75,-122.45"
    >>> sf = burin_map.bounds_around(lat, lon, 20, 20)                # km
    >>> bay = (-122.75, 37.45, -122.05, 38.05)                    # west, south, east, north
    >>> d = burin_map.draw(bay, frame_width_mm=420, frame_pen_mm=0.3)  # plotter / print
    >>> d.save_svg("bay.svg"); d.save_png("bay.png")

The engine is written in Rust (the `burin_map._burin` extension); this module
only adapts it to Python: numpy arrays, dataclasses, files.
"""

from __future__ import annotations

import dataclasses
from dataclasses import field
from pathlib import Path
from typing import Callable

import numpy as np

from . import _burin

__all__ = ["Drawing", "Style", "bounds_around", "draw", "draw_elevation", "fields",
           "locate", "projected_size", "style_groups", "style_help"]
__version__ = _burin.__version__

Bounds = tuple[float, float, float, float]  # west, south, east, north (degrees)

# Every drawing parameter, with its default and description, as declared in
# the Rust engine. Lengths are in output pixels, so the same style gives the
# same look whatever the area and scale.
Style = dataclasses.make_dataclass(
    "Style",
    [(name, int if name == "batch_seed" else float,
      field(default=int(default) if name == "batch_seed" else default,
            metadata={"help": help, "group": group}))
     for name, default, help, group in _burin.style_params()],
    module=__name__,
)
Style.__doc__ = "All tunable parameters of a drawing. `help(burin_map.Style)` or " \
                "`burin_map.style_help()` lists them with their meaning."


def style_help() -> str:
    return "\n".join(f"{f.name:20} {f.default!s:>6}  [{f.metadata['group']}]  {f.metadata['help']}"
                     for f in dataclasses.fields(Style))


def style_groups() -> list[tuple[str, str]]:
    """`(id, blurb)` for each style group."""
    return list(_burin.style_groups())


def _style_dict(style) -> dict | None:
    return None if style is None else dataclasses.asdict(style)


def _unpack(buffers) -> list[np.ndarray]:
    pts, ends = buffers
    xy = np.frombuffer(pts, "<f4").reshape(-1, 2)
    return np.split(xy, np.frombuffer(ends, "<u4")[:-1].astype(np.int64))


class Drawing:
    """A finished drawing. `paths` are the dashes in drawing order, `lines` the
    streamlines before dashing; both lists of N×2 float32 arrays (x, y in px)."""

    def __init__(self, native, style: Style, mm_per_px: float | None = None):
        self._native = native
        self.style = style
        self.width: int = native.width
        self.height: int = native.height
        self.px_m: float = native.px_m        # metres on the ground per pixel
        self.mm_per_px = mm_per_px            # physical size, when the frame has one
        self._paths = self._lines = None

    @property
    def paths(self) -> list[np.ndarray]:
        if self._paths is None:
            self._paths = _unpack(self._native.paths())
        return self._paths

    @property
    def lines(self) -> list[np.ndarray]:
        if self._lines is None:
            self._lines = _unpack(self._native.lines())
        return self._lines

    def pen_up_travel(self) -> float:
        """Distance travelled with the pen up between dashes, px."""
        return self._native.pen_up_travel()

    def svg(self) -> str:
        """SVG document; sized in mm when `mm_per_px` is set."""
        return self._native.svg(self.mm_per_px)

    def png(self) -> bytes:
        return self._native.png()

    def save_svg(self, path: str | Path) -> None:
        Path(path).write_text(self.svg())

    def save_png(self, path: str | Path) -> None:
        Path(path).write_bytes(self.png())

    def image(self):
        """As a PIL image (needs Pillow)."""
        import io

        from PIL import Image
        return Image.open(io.BytesIO(self.png()))

    def __repr__(self) -> str:
        return f"<burin_map.Drawing {self.width}×{self.height} px>"


def locate(query: str) -> tuple[float, float, str]:
    """(lat, lon, label) for "37.77, -122.42" or any address or place name
    (looked up on OpenStreetMap's Nominatim)."""
    return _burin.locate(query)


def bounds_around(lat: float, lon: float, width_km: float, height_km: float) -> Bounds:
    """Lon/lat box of roughly width × height km centred on (lat, lon)."""
    return _burin.bounds_around(lat, lon, width_km, height_km)


def projected_size(bounds: Bounds) -> tuple[float, float]:
    """Width and height in metres of `bounds` in the map's projection."""
    return _burin.projected_size(tuple(bounds))


def draw(bounds: Bounds, style: Style | None = None, *, frame_px: int = 2400,
         frame_width_mm: float | None = None, frame_height_mm: float | None = None,
         frame_pen_mm: float = 0.3, log: Callable[[str], None] | None = None) -> Drawing:
    """Download SRTM elevation for `bounds` (west, south, east, north in degrees) and draw it.

    `frame_px` is the image width in px. With `frame_width_mm` the frame has a
    physical size instead: the pen takes the place of `style.batch_stroke`, so
    every proportion of the style is kept and the pixel width follows from frame
    and pen. With `frame_height_mm` too, the drawing shrinks to fit inside both.
    """
    st = style or Style()
    native = _burin.draw(tuple(bounds), _style_dict(st), frame_px, frame_width_mm,
                         frame_height_mm, frame_pen_mm, log)
    return Drawing(native, st, native.mm_per_px)


def _elev_bytes(elev) -> tuple[bytes, int, int]:
    a = np.ascontiguousarray(elev, dtype="<f4")
    if a.ndim != 2:
        raise ValueError("elevation must be a 2-D array")
    return a.tobytes(), a.shape[1], a.shape[0]


def draw_elevation(elev, px_m: float, style: Style | None = None) -> Drawing:
    """Draw any elevation grid (metres, square pixels of `px_m` metres, row 0 = north).
    Values ≤ 0.5 m are treated as sea and left blank."""
    st = style or Style()
    buf, w, h = _elev_bytes(elev)
    return Drawing(_burin.draw_elevation(buf, w, h, px_m, _style_dict(st)), st)


@dataclasses.dataclass
class Fields:
    tx: np.ndarray    # unit vector along the contours of the smoothed surface
    ty: np.ndarray
    input: np.ndarray  # 0 = draw nothing … 1 = solid line
    space: np.ndarray  # local distance between lines, px


def fields(elev, px_m: float, style: Style | None = None) -> Fields:
    """The rasters the drawing is built from, for inspection and debugging."""
    buf, w, h = _elev_bytes(elev)
    out = _burin.fields(buf, w, h, px_m, _style_dict(style))
    return Fields(*(np.frombuffer(b, "<f4").reshape(h, w) for b in out))
