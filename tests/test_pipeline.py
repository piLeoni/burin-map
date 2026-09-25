"""Offline tests on a synthetic island: no downloads."""

import numpy as np
import pytest

from burin import Style, draw_elevation
from burin.fields import build_fields

PX_M = 200.0


@pytest.fixture(scope="module")
def island():
    y, x = np.mgrid[0:300, 0:400].astype(float)
    bump = lambda cx, cy, r, h: h * np.exp(-((x - cx) ** 2 + (y - cy) ** 2) / (2 * r * r))
    elev = bump(150, 150, 60, 1500) + bump(270, 170, 40, 900) - 150
    return np.clip(elev, 0, None)


@pytest.fixture(scope="module")
def drawing(island):
    return draw_elevation(island, PX_M, Style(width=400))


def test_tone_is_flat_level_on_flat_ground():
    elev = np.full((80, 80), 100.0)
    elev[40, 40] = 101.0  # something for the percentile scaling to work with
    f = build_fields(elev, PX_M, Style())
    assert np.median(f.tone[20:60, 20:60]) == pytest.approx(Style().flat_level, abs=0.02)


def test_shadow_darker_than_light(island):
    f = build_fields(island, PX_M, Style())
    # Light from the north-west: south-east flank in shadow, north-west flank lit.
    assert f.tone[190, 190] > f.tone[110, 110]


def test_produces_lines_and_dashes(drawing):
    assert len(drawing.lines) > 20
    assert len(drawing.paths) > len(drawing.lines)
    for p in drawing.paths:
        assert p.ndim == 2 and p.shape[1] == 2
        assert (p[:, 0] >= 0).all() and (p[:, 0] < drawing.width).all()
        assert (p[:, 1] >= 0).all() and (p[:, 1] < drawing.height).all()


def test_nothing_drawn_at_sea(drawing, island):
    pts = np.concatenate(drawing.paths).astype(int)
    assert (island[pts[:, 1], pts[:, 0]] > 0).mean() > 0.99


def test_lines_do_not_crowd(drawing):
    """Almost no point comes closer than half the minimum stop distance to a point of
    another line, or of the same line far along it (a spiral)."""
    from scipy.spatial import cKDTree

    st = drawing.style
    min_d = st.spacing * st.spacing_steep * st.spacing_dark * st.test_ratio * 0.5
    pts = np.concatenate(drawing.lines)
    owner = np.concatenate([np.full(len(l), i) for i, l in enumerate(drawing.lines)])
    arc = np.concatenate([np.arange(len(l)) for l in drawing.lines])
    i, j = cKDTree(pts).query_pairs(min_d, output_type="ndarray").T
    crowded = (owner[i] != owner[j]) | (np.abs(arc[i] - arc[j]) > 4 * st.spacing / st.step)
    assert crowded.sum() < 0.005 * len(pts)


def test_plot_order_cuts_travel(drawing):
    from burin.render import pen_up_travel, plot_order

    rng = np.random.default_rng(0)
    shuffled = [drawing.lines[i] for i in rng.permutation(len(drawing.lines))]
    assert pen_up_travel(plot_order(shuffled)) < 0.5 * pen_up_travel(shuffled)


def test_svg_physical_size(drawing, tmp_path):
    from dataclasses import replace

    d = replace(drawing, mm_per_px=0.25)
    d.save_svg(tmp_path / "p.svg")
    head = (tmp_path / "p.svg").read_text()[:200]
    assert f'width="{drawing.width * 0.25:.2f}mm"' in head


def test_coordinates_and_bounds():
    from burin import bounds_around, locate

    lat, lon, _ = locate("37.7544, -122.4477")
    assert (lat, lon) == (37.7544, -122.4477)
    w, s, e, n = bounds_around(lat, lon, 20, 10)
    assert (n - s) * 111.32 == pytest.approx(10)
    assert (e - w) * 111.32 * np.cos(np.radians(lat)) == pytest.approx(20)


def test_svg_and_png(drawing, tmp_path):
    drawing.save_svg(tmp_path / "a.svg")
    drawing.save_png(tmp_path / "a.png")
    assert (tmp_path / "a.svg").read_text().count("<path") == len(drawing.paths)
    assert (tmp_path / "a.png").stat().st_size > 0
