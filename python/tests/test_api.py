"""Offline tests of the Python API on a synthetic island: no downloads.
The engine itself is tested in Rust (`cargo test`)."""

import dataclasses

import numpy as np
import pytest

import burin
from burin import Style, draw_elevation

PX_M = 200.0


@pytest.fixture(scope="module")
def island():
    y, x = np.mgrid[0:300, 0:400].astype(float)
    bump = lambda cx, cy, r, h: h * np.exp(-((x - cx) ** 2 + (y - cy) ** 2) / (2 * r * r))
    elev = bump(150, 150, 60, 1500) + bump(270, 170, 40, 900) - 150
    return np.clip(elev, 0, None)


@pytest.fixture(scope="module")
def drawing(island):
    return draw_elevation(island, PX_M)


def test_style_mirrors_the_engine():
    st = Style()
    assert st.spacing == pytest.approx(4.8) and st.seed == 1
    assert dataclasses.replace(st, spacing=6).spacing == 6
    assert "spacing" in burin.style_help()
    with pytest.raises(TypeError):
        Style(nope=1)


def test_produces_lines_and_dashes(drawing):
    assert (drawing.width, drawing.height) == (400, 300)
    assert len(drawing.lines) > 20
    assert len(drawing.paths) > len(drawing.lines)
    for p in drawing.paths:
        assert p.ndim == 2 and p.shape[1] == 2
        assert (p[:, 0] >= 0).all() and (p[:, 0] < drawing.width).all()
        assert (p[:, 1] >= 0).all() and (p[:, 1] < drawing.height).all()


def test_nothing_drawn_at_sea(drawing, island):
    pts = np.concatenate(drawing.paths).astype(int)
    assert (island[pts[:, 1], pts[:, 0]] > 0).mean() > 0.99


def test_style_changes_the_drawing(island):
    sparse = draw_elevation(island, PX_M, Style(spacing=8))
    dense = draw_elevation(island, PX_M, Style(spacing=4))
    assert len(dense.lines) > 1.5 * len(sparse.lines)


def test_fields(island):
    f = burin.fields(island, PX_M)
    assert f.tone.shape == island.shape
    # Light from the north-west: south-east flank in shadow, north-west flank lit.
    assert f.tone[190, 190] > f.tone[110, 110]
    assert np.allclose(np.hypot(f.tx, f.ty)[island > 100], 1, atol=1e-3)


def test_svg_and_png(drawing, tmp_path):
    drawing.save_svg(tmp_path / "a.svg")
    drawing.save_png(tmp_path / "a.png")
    assert (tmp_path / "a.svg").read_text().count("<path") == len(drawing.paths)
    assert (tmp_path / "a.png").read_bytes()[:4] == b"\x89PNG"


def test_svg_physical_size(drawing):
    drawing.mm_per_px = 0.25
    try:
        assert f'width="{drawing.width * 0.25:.2f}mm"' in drawing.svg()[:200]
    finally:
        drawing.mm_per_px = None


def test_coordinates_and_bounds():
    lat, lon, _ = burin.locate("37.7544, -122.4477")
    assert (lat, lon) == (37.7544, -122.4477)
    w, s, e, n = burin.bounds_around(lat, lon, 20, 10)
    assert (n - s) * 111.32 == pytest.approx(10)
    assert (e - w) * 111.32 * np.cos(np.radians(lat)) == pytest.approx(20)
    pw, ph = burin.projected_size((w, s, e, n))
    assert pw == pytest.approx(20_000, rel=0.01) and ph == pytest.approx(10_000, rel=0.01)


def test_bad_input_raises(island):
    with pytest.raises(ValueError):
        draw_elevation(island[0], PX_M)
