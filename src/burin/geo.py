"""Where to draw: addresses and coordinates to bounds."""

from __future__ import annotations

import json
import math
import re
import urllib.parse
import urllib.request

from .dem import Bounds

NOMINATIM = "https://nominatim.openstreetmap.org/search"
_LATLON = re.compile(r"^\s*(-?\d+(?:\.\d+)?)\s*[, ]\s*(-?\d+(?:\.\d+)?)\s*$")


def locate(query: str) -> tuple[float, float, str]:
    """(lat, lon, label) for "37.77, -122.42" or any address/place name.
    Names go to OpenStreetMap's Nominatim (one request, no key needed)."""
    m = _LATLON.match(query)
    if m:
        lat, lon = float(m[1]), float(m[2])
        return lat, lon, f"{lat:.5f}, {lon:.5f}"
    url = NOMINATIM + "?" + urllib.parse.urlencode({"q": query, "format": "json", "limit": 1})
    req = urllib.request.Request(url, headers={"User-Agent": "burin (terrain drawing tool)"})
    with urllib.request.urlopen(req, timeout=30) as r:
        hits = json.load(r)
    if not hits:
        raise ValueError(f"nothing found for {query!r}")
    return float(hits[0]["lat"]), float(hits[0]["lon"]), hits[0]["display_name"]


def bounds_around(lat: float, lon: float, width_km: float, height_km: float) -> Bounds:
    """Lon/lat box of roughly width × height km centred on (lat, lon)."""
    dlat = height_km / 2 / 111.32
    dlon = width_km / 2 / (111.32 * math.cos(math.radians(lat)))
    return (lon - dlon, lat - dlat, lon + dlon, lat + dlat)
