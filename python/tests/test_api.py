"""Offline tests of the Python API on a synthetic island: no downloads.
The engine itself is tested in Rust (`cargo test`)."""

import dataclasses

import numpy as np
import pytest

import burin_map
from burin_map import Style, draw_elevation

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
    assert st.space_light == pytest.approx(6) and st.batch_seed == 1
    assert dataclasses.replace(st, space_light=8).space_light == 8
    assert "space_light" in burin_map.style_help()
    assert any(g[0] == "space" for g in burin_map.style_groups())
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
    sparse = draw_elevation(island, PX_M, Style(space_light=8, space_dark=8))
    dense = draw_elevation(island, PX_M, Style(space_light=4, space_dark=4))
    assert len(dense.lines) > 1.5 * len(sparse.lines)


def test_fields(island):
    f = burin_map.fields(island, PX_M)
    assert f.input.shape == island.shape
    # Light from the north-west: south-east flank in shadow, north-west flank lit.
    assert f.input[190, 190] > f.input[110, 110]
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
    lat, lon, _ = burin_map.locate("37.7544, -122.4477")
    assert (lat, lon) == (37.7544, -122.4477)
    w, s, e, n = burin_map.bounds_around(lat, lon, 20, 10)
    assert (n - s) * 111.32 == pytest.approx(10)
    assert (e - w) * 111.32 * np.cos(np.radians(lat)) == pytest.approx(20)
    pw, ph = burin_map.projected_size((w, s, e, n))
    assert pw == pytest.approx(20_000, rel=0.01) and ph == pytest.approx(10_000, rel=0.01)


def test_bad_input_raises(island):
    with pytest.raises(ValueError):
        draw_elevation(island[0], PX_M)


def test_evaluate_runs_a_document():
    """A document is a script; `evaluate` is the front end that runs it."""
    doc = burin_map.evaluate(
        "burin.area({ at: '46,7.7', size_km: [8, 6] })\n"
        "burin.batch({ angle: 0, style: { space_light: 5 } })\n"
        "burin.water('dots', { color: '#9db4c0' })\n"
        "burin.shore()\n"
        "burin.draw()\n"
    )
    assert [b.get("kind") for b in doc["atlas"]] == [None, "water", "shore"]
    assert doc["atlas"][0]["angle"] == 0
    assert doc["area"] == {"at": "46,7.7", "size_km": [8, 6]}


def test_evaluate_reports_a_mistake_with_its_position():
    with pytest.raises(ValueError, match=r"line 1 column 12"):
        burin_map.evaluate('burin.batch({ angle: "x" })\nburin.draw()')
    with pytest.raises(ValueError, match="nothing to draw"):
        burin_map.evaluate("burin.draw()")


def test_every_shipped_preset_runs():
    from pathlib import Path

    presets = Path(__file__).resolve().parents[2] / "presets"
    for name in ("contours", "contour-belts", "downhill", "layer-tints", "scripted-hills", "sea-depth"):
        doc = burin_map.evaluate((presets / f"{name}.js").read_text())
        assert isinstance(doc["atlas"], list) and doc["atlas"], f"{name} built no atlas"
