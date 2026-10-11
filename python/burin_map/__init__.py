"""Pen-and-ink terrain maps from real elevation data.

A drawing is a document: the same calls a document script makes
(`presets/*.js`, the web editor, `burin-map --script`), here with Python's
keywords. Build it up, then draw:

    >>> import burin_map as burin
    >>> burin.area(at="37.66,-122.45", size_km=[36, 25])   # size_mm=[420, 594] for paper
    >>> burin.batch(angle=0, style={"space_light": 6})
    >>> burin.water("lines", follow="shore")
    >>> burin.shore(lines=2)
    >>> d = burin.draw()                                   # and a new, empty document
    >>> d.save_svg("sf.svg"); d.save_png("sf.png")

or run a document script as it is: `burin.draw_script(open("contours.js").read())`.

The engine is written in Rust (the `burin_map._burin` extension); this module
only adapts it to Python: numpy arrays, dataclasses, files.
"""

from __future__ import annotations

import dataclasses
import json as _json
import math
from dataclasses import field
from pathlib import Path
from typing import Callable

import numpy as np

from . import _burin

# `json`, `abs`, `round`, `min` and `max` stay out of `import *`: they would
# hide the module and the builtins.
__all__ = ["Drawing", "Style", "area", "batch", "bounds_around", "ceil", "clamp",
           "depth", "dist", "draw", "draw_elevation", "draw_script", "elev", "engine",
           "evaluate", "fields", "fill", "floor", "force_cpu", "lerp", "locate", "mix",
           "parameters", "projected_size", "rgb", "shore", "smoothstep", "step",
           "style_groups", "style_help", "tone", "water"]
__version__ = _burin.__version__

Bounds = tuple[float, float, float, float]  # west, south, east, north (degrees)



def engine() -> str:
    """Where the terrain fields are built. The GPU when the machine has one,
    the CPU when it does not: the drawing is the same shape either way."""
    return _burin.engine()


def force_cpu() -> None:
    """Build the fields on the CPU from now on. Slower, but bit-for-bit
    reproducible — the GPU works in f32 and lands every dash in a slightly
    different place, differently on each GPU."""
    if not hasattr(_burin, "force_cpu"):
        return
    _burin.force_cpu()

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
                "`burin_map.style_help()` lists them with their meaning. Passed as a " \
                "batch's `style`, it writes every value out."


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

    def __init__(self, native, style: Style | None = None, mm_per_px: float | None = None):
        self._native = native
        self.style = style
        self.width: int = native.width
        self.height: int = native.height
        self.px_m: float = native.px_m        # metres on the ground per pixel
        self.mm_per_px = mm_per_px            # physical size, when the area has paper
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


# ---- the document ----------------------------------------------------------
# The calls of a document script, one for one, with the same checks: the
# library a script runs is `crates/burin-map-script/src/library.js`.

_items: list[dict] = []
_area: dict | None = None


def _object(v, what: str) -> dict:
    if v is None:
        return {}
    if not isinstance(v, dict):
        raise ValueError(f"{what} must be a dict")
    return v


def _number(v, what: str):
    if isinstance(v, bool) or not isinstance(v, (int, float)) or not math.isfinite(v):
        raise ValueError(f"{what} must be a number")
    return v


def _color(v, what: str):
    if isinstance(v, str):
        return v
    if isinstance(v, (list, tuple)):
        return list(v)
    raise ValueError(f"{what} must be a colour string or a tree")


def _scalar(v, what: str):
    if isinstance(v, (list, tuple)):
        return list(v)
    return _number(v, what)


def _style_params(v) -> dict:
    """A batch's style parameters: numbers only, by their short names too.
    A `Style` writes every value out."""
    if dataclasses.is_dataclass(v) and not isinstance(v, type):
        v = dataclasses.asdict(v)
    return {k: _number(x, f"style {k}") for k, x in _object(v, "style").items() if x is not None}


def _band(v, what: str, water: bool = False) -> dict:
    o = _object(v, what)
    keys = ("depthMin", "depthMax", "depthFade") if water else \
        ("elevMin", "elevMax", "depthMin", "depthMax")
    out = {k: _number(o[k], f"{what}.{k}") for k in keys if o.get(k) is not None}
    if out.get("elevMin", -math.inf) > out.get("elevMax", math.inf):
        raise ValueError(f"{what}: elevMin is above elevMax")
    if out.get("depthMin", -math.inf) > out.get("depthMax", math.inf):
        raise ValueError(f"{what}: depthMin is above depthMax")
    return out


def _keep(kind: str, entry: dict) -> dict:
    _burin.check(kind, _json.dumps(entry))
    _items.append(entry)
    return entry


def area(**options) -> dict:
    """Where to draw: `place`, `at`, `bounds`, `size_km`, `size_mm`, `scale`,
    `pen_mm` — the keys of `burin.area({...})`."""
    global _area
    _burin.check("area", _json.dumps(options))
    _area = dict(options)
    return _area


def _names(v, what: str) -> list[str]:
    names = [v] if isinstance(v, str) else list(v) if isinstance(v, (list, tuple)) else None
    if names is None or not all(isinstance(n, str) for n in names):
        raise ValueError(f"{what} must be a region name or a list of them")
    return names


def _regions(regions) -> dict | None:
    """Regions by name as GeoJSON: dicts, or anything with `__geo_interface__`
    (Shapely, GeoPandas, …)."""
    if regions is None:
        return None
    out = {}
    for name, geo in _object(regions, "regions").items():
        geo = getattr(geo, "__geo_interface__", geo)
        if not isinstance(geo, dict):
            raise ValueError(f"region {name!r} must be GeoJSON or have __geo_interface__")
        out[str(name)] = geo
    return out


def batch(*, name=None, angle=None, draw=None, ground=None, color=None, shade=None,
          width=None, cap=None, where=None, within=None, outside=None, style=None,
          **params) -> dict:
    """A batch of lines, stippling or a fill, as `burin.batch({...})`. Style
    parameters go in `style` (a dict, or a `Style` with every value) or by name.
    `within` / `outside` name regions handed to `draw(regions=...)`: lines and
    dots keep to them, or give way to them, easing at the edge (`mask_*`)."""
    if draw not in (None, "lines", "dots", "fill"):
        raise ValueError(f'batch draw must be "lines", "dots" or "fill", not {draw!r}')
    if ground not in (None, "land", "sea"):
        raise ValueError(f'batch ground must be "land" or "sea", not {ground!r}')
    out: dict = {}
    if name is not None:
        out["name"] = str(name)
    if angle is not None:
        out["angle"] = _number(angle, "batch angle")
    if cap is not None:
        out["cap"] = str(cap)
    if ground is not None:
        out["ground"] = ground
    if draw is not None:
        out["draw"] = draw
    if color is not None:
        out["color"] = _color(color, "batch color")
    if shade is not None:
        out["shade"] = _color(shade, "batch shade")
    if width is not None:
        out["width"] = _scalar(width, "batch width")
    if where is not None:
        out["where"] = _band(where, "batch where")
    if within is not None:
        out["within"] = _names(within, "batch within")
    if outside is not None:
        out["outside"] = _names(outside, "batch outside")
    out["style"] = {**_style_params(style), **_style_params(params)}
    return _keep("batch", out)


def fill(**options) -> dict:
    """A fill of a band: what a contour belt is."""
    return batch(**{**options, "draw": "fill"})


def water(kind: str = "lines", *, name=None, angle=None, follow=None, color=None, shade=None,
          width=None, cap=None, where=None, style=None, **params) -> dict:
    """A pass over the water: `water("lines" | "ripples" | "dots", ...)`."""
    if kind not in ("lines", "ripples", "dots"):
        raise ValueError(f"water draws lines, ripples or dots, not {kind!r}")
    out: dict = {"draw": kind, "kind": "water"}
    if name is not None:
        out["name"] = str(name)
    if angle is not None:
        out["angle"] = _number(angle, "water angle")
    if cap is not None:
        out["cap"] = str(cap)
    if follow is not None:
        out["follow"] = str(follow)
    if color is not None:
        out["color"] = _color(color, "water color")
    if shade is not None:
        out["shade"] = _color(shade, "water shade")
    if width is not None:
        out["width"] = _scalar(width, "water width")
    if where is not None:
        out["where"] = _band(where, "water where", water=True)
    out["style"] = {**_style_params(style), **_style_params(params)}
    return _keep("water", out)


def shore(*, color=None, shade=None, width=None, lines=None, style=None, **params) -> dict:
    """The shoreline, the zero contour; or its own style."""
    out: dict = {"kind": "shore"}
    if color is not None:
        out["color"] = _color(color, "shore color")
    if shade is not None:
        out["shade"] = _color(shade, "shore shade")
    if width is not None:
        out["width"] = _scalar(width, "shore width")
    if lines is not None:
        out["lines"] = _number(lines, "shore lines")
    out["style"] = {**_style_params(style), **_style_params(params)}
    return _keep("shore", out)


def draw(*, regions=None, log: Callable[[str], None] | None = None) -> Drawing:
    """Draw what has been built — downloading the elevation, and the water when
    there are water passes — and start a new, empty document. `regions` are
    the polygons batches name in `within` / `outside`, by name, in lon/lat.
    `log` gets a line per step."""
    global _area
    if not _items:
        raise ValueError("the document has nothing to draw: add a batch with batch(...)")
    payload = {"atlas": _items, "area": _area, "regions": _regions(regions)}
    native = _burin.draw_document(_json.dumps(payload), log)
    _items.clear()
    _area = None
    return Drawing(native, mm_per_px=native.mm_per_px)


def json() -> list[dict]:
    """The document built so far, without drawing it."""
    return [dict(i) for i in _items]


def parameters() -> list[dict]:
    """The style parameters, by name, with their help."""
    return _json.loads(_burin.parameters())


# The fields a tree may read where a dash sits.
elev = "elev"
tone = "tone"
dist = "dist"
depth = "depth"


def lerp(a, b, t):
    """A colour between two others, by `t`."""
    return ["lerp", _color(a, "lerp"), _color(b, "lerp"), t]


def rgb(r, g, b):
    """An rgb colour from three trees."""
    return ["rgb", r, g, b]


def clamp(x, lo, hi):
    return ["clamp", x, lo, hi]


def smoothstep(edge0, edge1, x):
    return ["smoothstep", edge0, edge1, x]


def step(edge, x):
    """0 below `edge`, 1 above."""
    return ["step", edge, x]


def abs(x):  # noqa: A001
    return ["abs", x]


def floor(x):
    return ["floor", x]


def ceil(x):
    return ["ceil", x]


def round(x):  # noqa: A001
    return ["round", x]


def min(a, b):  # noqa: A001
    return ["min", a, b]


def max(a, b):  # noqa: A001
    return ["max", a, b]


def mix(a, b, t):
    """Two colours mixed, `t` decided once."""
    return ["mix", _color(a, "mix"), _color(b, "mix"), _number(t, "mix t")]


# ---- documents as files ----------------------------------------------------

def evaluate(script: str) -> dict:
    """Run a document script and return what it built.

        >>> doc = burin_map.evaluate(open("contours.js").read())
        >>> doc["atlas"][0]["kind"] if doc["atlas"] else "batch"
        'batch'

    Returns `{"atlas": [...], "area": {...} | None}`: the batches and water
    passes the engine draws, in order, and the area the script asked for.
    """
    return _json.loads(_burin.evaluate(script))


def draw_script(script: str, *, regions=None,
                log: Callable[[str], None] | None = None) -> Drawing:
    """Run a document script and draw it, as `burin-map --script` does;
    `regions` as `draw` takes them."""
    payload = {**_json.loads(_burin.evaluate(script)), "regions": _regions(regions)}
    native = _burin.draw_document(_json.dumps(payload), log)
    return Drawing(native, mm_per_px=native.mm_per_px)


# ---- your own elevation ----------------------------------------------------

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
