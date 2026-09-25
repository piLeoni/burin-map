"""All tunable parameters of a drawing.

Lengths are in output pixels, not metres, so the same style gives the same
look whatever the area and scale of the map.
"""

from __future__ import annotations

from dataclasses import dataclass, field, fields


def _p(default, help: str):
    return field(default=default, metadata={"help": help})


@dataclass
class Style:
    # Canvas
    width: int = _p(2400, "output width in px (height follows the area's aspect)")

    # Line layout
    spacing: float = _p(4.8, "distance between neighbouring lines, px")
    spacing_steep: float = _p(0.6, "spacing multiplier on the steepest ground (1 = constant)")
    spacing_dark: float = _p(0.7, "spacing multiplier in the deepest shadow (1 = constant)")
    spacing_gamma: float = _p(1.0, "curve of the steepness → spacing mapping")
    test_ratio: float = _p(0.5, "a line stops when closer than spacing × this to another")
    fill_ratio: float = _p(0.8, "gaps wider than 2 × this × spacing get a line in the middle (1 = off)")
    step: float = _p(1.0, "integration step, px")
    min_len: float = _p(4.0, "drop lines shorter than this, px")

    # Smoothing of the terrain (gaussian sigma, px)
    flow_blur: float = _p(10.0, "smoothing of the surface lines follow: higher = broader curves")
    shade_blur: float = _p(1.8, "smoothing for detail shading")
    broad_blur: float = _p(11.5, "smoothing for massif-scale shading")
    broad_weight: float = _p(0.5, "mix of massif-scale shading (0 = detail only, 1 = massif only)")
    wobble: float = _p(20.0, "random swing of line direction on flat ground, degrees")
    wobble_size: float = _p(12.0, "size of the wobble's bends, px")

    # Light
    azimuth: float = _p(315.0, "light direction, degrees clockwise from north")
    altitude: float = _p(45.0, "light elevation above the horizon, degrees")
    z_exag: float = _p(4.0, "vertical exaggeration used for shading")

    # Tone: 0 = nothing drawn, 1 = solid line
    flat_level: float = _p(0.25, "tone of flat ground; keeps a minimum stipple everywhere")
    shade_gain: float = _p(3.0, "how quickly slopes saturate to solid (shadow) or white (light)")
    dark_gamma: float = _p(0.7, "<1 pushes shadowed slopes towards solid")
    lit_gain: float = _p(1.5, ">1 makes lit slopes reach full light sooner")
    lit_floor: float = _p(0.06, "tone kept in full light: a few scattered dashes hint at the form")

    # Dashes
    period: float = _p(2.2, "dash + gap length, in multiples of spacing")
    duty_min: float = _p(0.15, "dash fraction of the period at tone 0")
    duty_max: float = _p(1.0, "dash fraction of the period at tone 1")
    cutoff: float = _p(0.03, "below this tone nothing is drawn")
    jitter: float = _p(0.45, "irregularity of dash length and pitch (0 = mechanical); fades in solid shadow")
    breaks: float = _p(0.15, "chance per period of a small random break in otherwise solid line")

    # Output
    stroke: float = _p(1.2, "stroke width, px")
    seed: int = _p(1, "random seed (seed order, dash jitter)")


def style_fields():
    """(name, default, help) for every Style parameter; used to build the CLI."""
    return [(f.name, f.default, f.metadata.get("help", "")) for f in fields(Style)]
