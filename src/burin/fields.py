"""Raster fields derived from the elevation: line direction, tone, spacing."""

from __future__ import annotations

import math
from dataclasses import dataclass

import numpy as np
from scipy import ndimage

from .style import Style


@dataclass
class Fields:
    tx: np.ndarray        # unit vector along the contours of the smoothed surface
    ty: np.ndarray
    tone: np.ndarray      # 0 = draw nothing … 1 = solid line
    walkable: np.ndarray  # where lines may go (land with a defined direction)
    sep: np.ndarray       # local line spacing, px


def _percentile_rank(v: np.ndarray, mask: np.ndarray) -> np.ndarray:
    """Rank of each value among `mask` pixels, in [0, 1]; 0 outside the mask."""
    out = np.zeros_like(v)
    out[mask] = np.argsort(np.argsort(v[mask])) / max(mask.sum() - 1, 1)
    return out


def _relative_shade(elev: np.ndarray, blur: float, land: np.ndarray, st: Style) -> np.ndarray:
    """Lambert shading relative to flat ground: 0 flat, +1 deep shadow, -1 fully lit.

    Scaled by the 98th percentile over land so the result doesn't depend on
    how rugged the area is.
    """
    surf = ndimage.gaussian_filter(elev, blur)
    sy, sx = np.gradient(surf)
    alt, az = math.radians(st.altitude), math.radians(st.azimuth)
    # Image y points south, so north is -y.
    lx, ly, lz = math.cos(alt) * math.sin(az), -math.cos(alt) * math.cos(az), math.sin(alt)
    nx, ny = -sx * st.z_exag, -sy * st.z_exag
    lit = (nx * lx + ny * ly + lz) / np.sqrt(nx * nx + ny * ny + 1.0)
    rel = lz - lit
    return np.clip(rel / max(np.percentile(np.abs(rel[land]), 98), 1e-9), -1.0, 1.0)


def build_fields(elev: np.ndarray, px_m: float, st: Style) -> Fields:
    land = elev > 0.5

    # Direction: gradient of a heavily smoothed surface, turned 90°. Lines that
    # follow it are contour-like but broad and evenly curved.
    h = ndimage.gaussian_filter(elev, st.flow_blur)
    gy, gx = np.gradient(h, px_m)
    mag = np.hypot(gx, gy)
    inv = np.where(mag > 1e-9, 1.0 / np.maximum(mag, 1e-12), 0.0)
    tx, ty = -gy * inv, gx * inv
    steep = _percentile_rank(mag, land)

    # On near-flat ground the field is almost uniform and lines come out
    # combed-parallel. A smooth random rotation there, fading to none on
    # slopes, makes them wander like a hand would.
    if st.wobble > 0:
        rng = np.random.default_rng(st.seed)
        noise = ndimage.gaussian_filter(rng.standard_normal(elev.shape), st.wobble_size)
        noise /= max(noise.std(), 1e-12)
        a = np.radians(st.wobble) * noise * (1.0 - steep) ** 2
        ca, sa = np.cos(a), np.sin(a)
        tx, ty = tx * ca - ty * sa, tx * sa + ty * ca

    # Shading at two scales: detail keeps ridges and valleys readable up close,
    # the massif scale gives whole mountain sides one tone from afar.
    elev_px = elev / px_m  # heights in pixel units so gradients are true slopes
    s = _relative_shade(elev_px, st.shade_blur, land, st)
    if st.broad_weight > 0:
        s = (1 - st.broad_weight) * s + st.broad_weight * _relative_shade(
            elev_px, st.broad_blur, land, st)
    s = np.clip(s * st.shade_gain, -1.0, 1.0)

    # Flat ground sits at flat_level; shadow raises it towards solid, light
    # lowers it towards white.
    shadow = np.clip(s, 0.0, 1.0) ** st.dark_gamma
    light = np.clip(-s * st.lit_gain, 0.0, 1.0)
    tone = np.where(s >= 0, st.flat_level + (1.0 - st.flat_level) * shadow,
                    st.lit_floor + (st.flat_level - st.lit_floor) * (1.0 - light))
    coast = ndimage.gaussian_filter(land.astype(np.float64), 2.0)
    tone *= np.clip(coast * 1.4 - 0.2, 0.0, 1.0)  # fade out at the shoreline

    # Spacing: tighter on steep ground and in shadow, smoothed so neighbouring
    # lines stay roughly parallel.
    keep = (1.0 - (1.0 - st.spacing_steep) * steep ** st.spacing_gamma) \
         * (1.0 - (1.0 - st.spacing_dark) * np.where(s >= 0, shadow, 0.0))
    sep = st.spacing * (1.0 - ndimage.gaussian_filter(1.0 - keep, 3.0))

    return Fields(tx, ty, tone, land & (mag > 1e-7), sep)
