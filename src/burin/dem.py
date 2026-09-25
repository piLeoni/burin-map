"""Elevation data: SRTM tiles from AWS Terrain Tiles, reprojected to a
transverse Mercator centred on the area (conformal, scale exact at the centre).

Tiles are 1°×1°, 1 arc-second (~30 m), fetched on demand and cached
decimated to roughly the output resolution. Set BURIN_CACHE to move the
cache (default: ~/.cache/burin).
"""

from __future__ import annotations

import gzip
import math
import os
import urllib.error
import urllib.request
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from pathlib import Path

import numpy as np
from rasterio.transform import from_bounds
from rasterio.warp import Resampling, reproject, transform_bounds

SKADI = "https://s3.amazonaws.com/elevation-tiles-prod/skadi"
SRTM_PX_M = 30.9  # metres per arc-second along a meridian

Bounds = tuple[float, float, float, float]  # west, south, east, north (degrees)


@dataclass
class DEM:
    elev: np.ndarray  # metres, row 0 = north; sea and missing tiles are 0
    px_m: float       # metres per pixel
    crs: str          # PROJ definition of the grid's projection


def cache_dir() -> Path:
    d = Path(os.environ.get("BURIN_CACHE", Path.home() / ".cache" / "burin"))
    d.mkdir(parents=True, exist_ok=True)
    return d


def centred_tmerc(lon: float, lat: float) -> str:
    return f"+proj=tmerc +lat_0={lat} +lon_0={lon} +k=1 +x_0=0 +y_0=0 +datum=WGS84 +units=m"


def _stride_for(px_m: float, side: int = 3600) -> int:
    """Largest decimation that divides the tile and stays at or above output resolution."""
    want = max(1, int(px_m / SRTM_PX_M))
    return max(d for d in range(1, want + 1) if side % d == 0)


def _tile(lat: int, lon: int, stride: int) -> np.ndarray | None:
    """One SRTM tile decimated by `stride`, without its shared last row/col. None over sea."""
    ns, ew = ("N" if lat >= 0 else "S"), ("E" if lon >= 0 else "W")
    name = f"{ns}{abs(lat):02d}{ew}{abs(lon):03d}"
    cached = cache_dir() / f"{name}_s{stride}.npy"
    missing = cache_dir() / f"{name}.missing"
    if cached.exists():
        return np.load(cached)
    if missing.exists():
        return None
    url = f"{SKADI}/{ns}{abs(lat):02d}/{name}.hgt.gz"
    try:
        req = urllib.request.Request(url, headers={"User-Agent": "burin"})
        with urllib.request.urlopen(req, timeout=120) as r:
            raw = gzip.decompress(r.read())
    except urllib.error.HTTPError as e:
        if e.code in (403, 404):  # no tile = open ocean
            missing.touch()
            return None
        raise
    n = math.isqrt(len(raw) // 2)
    arr = np.frombuffer(raw, dtype=">i2").reshape(n, n)[:-1:stride, :-1:stride]
    arr = arr.astype(np.float32)
    arr[arr < -1000] = 0.0  # voids
    np.save(cached, arr)
    return arr


def _projected_extent(bounds: Bounds) -> tuple[str, tuple[float, float, float, float]]:
    w, s, e, n = bounds
    crs = centred_tmerc((w + e) / 2, (s + n) / 2)
    return crs, transform_bounds("EPSG:4326", crs, w, s, e, n)


def projected_aspect(bounds: Bounds) -> float:
    """Height / width of the area once projected."""
    _, (xmin, ymin, xmax, ymax) = _projected_extent(bounds)
    return (ymax - ymin) / (xmax - xmin)


def load_dem(bounds: Bounds, width: int) -> DEM:
    """Elevation for `bounds` on a grid `width` px wide with square metric pixels."""
    w, s, e, n = bounds
    crs, (xmin, ymin, xmax, ymax) = _projected_extent(bounds)
    px_m = (xmax - xmin) / width
    height = int(round((ymax - ymin) / px_m))
    stride = _stride_for(px_m)

    lats = range(math.floor(n), math.floor(s) - 1, -1)  # north to south
    lons = range(math.floor(w), math.floor(e) + 1)
    jobs = [(la, lo) for la in lats for lo in lons]
    with ThreadPoolExecutor(8) as ex:
        tiles = dict(zip(jobs, ex.map(lambda j: _tile(*j, stride), jobs)))

    side = 3600 // stride
    blank = np.zeros((side, side), np.float32)
    mosaic = np.concatenate([
        np.concatenate([tiles[(la, lo)] if tiles[(la, lo)] is not None else blank
                        for lo in lons], axis=1)
        for la in lats], axis=0)
    src_tr = from_bounds(lons[0], lats[-1], lons[-1] + 1, lats[0] + 1,
                         mosaic.shape[1], mosaic.shape[0])

    out = np.zeros((height, width), np.float32)
    reproject(mosaic, out, src_transform=src_tr, src_crs="EPSG:4326",
              dst_transform=from_bounds(xmin, ymin, xmax, ymax, width, height),
              dst_crs=crs, resampling=Resampling.bilinear)
    return DEM(out.astype(np.float64), px_m, crs)
