#!/usr/bin/env python3
"""Render the figures for docs/guide and the /docs site.

    cargo build --release -p burin-map
    python3 web/scripts/docs-figures/figures.py            # everything
    python3 web/scripts/docs-figures/figures.py examples   # or: figures

For each comparison: a labelled PNG row (docs/guide/img, read on GitHub) and
one cropped SVG per variant (web/public/docs/svg) for the zoomable viewer on
the site. For each preset example: the whole map as a simplified SVG. Everything the site needs is listed in
web/src/docs/figures.json.
"""
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

sys.path.insert(0, str(Path(__file__).parent))
from svgcrop import crop_svg  # noqa: E402

ROOT = Path(__file__).resolve().parents[3]
BIN = ROOT / "target/release/burin-map"
WORK = Path(tempfile.gettempdir()) / "burin-docs-figures"
(WORK / "ex").mkdir(parents=True, exist_ok=True)
OUT = ROOT / "docs/guide/img"
SVG_OUT = ROOT / "web/public/docs/svg"
MANIFEST = ROOT / "web/src/docs/figures.json"
for d in (OUT, SVG_OUT):
    d.mkdir(parents=True, exist_ok=True)

FIGURES = {}
EXAMPLES = {}

AREA = '37.73811,-122.36351'
SIZE_KM = [6, 4]


def _font(size):
    try:
        return ImageFont.truetype("/System/Library/Fonts/Supplemental/Arial.ttf", size)
    except OSError:
        return ImageFont.load_default(size)


FONT = _font(20)
FONT_SMALL = _font(16)

# Tight crops picked by hand against a plain render of the test area.
LAND_BOX = (380, 1160, 820, 1600)     # one swirling hillside
SHORE_BOX = (1100, 1050, 1580, 1500)   # bay mouth: land meeting open water


def render(doc_js: str, name: str) -> tuple[Path, Path]:
    path = WORK / f"{name}.js"
    path.write_text(doc_js)
    svg = WORK / f"{name}.svg"
    r = subprocess.run([str(BIN), "--script", str(path), "--out-file", str(svg), "--out-format", "both"],
                       capture_output=True, text=True)
    if r.returncode != 0:
        raise SystemExit(f"{name}: {r.stdout}\n{r.stderr}")
    return WORK / f"{name}.png", svg


def crop(img: Image.Image, box, scale: int = 1) -> Image.Image:
    assert box[2] <= img.width and box[3] <= img.height, f"{box} is outside the {img.size} render"
    im = img.convert("L").crop(box)
    if scale != 1:
        im = im.resize((im.width * scale, im.height * scale), Image.LANCZOS)
    return im


def save_png(im: Image.Image, path: Path) -> None:
    """16 greys are plenty for black ink on white, and halve the file."""
    im.convert("L").quantize(16).save(path, optimize=True)


def label_strip(im: Image.Image, text: str) -> Image.Image:
    bar = 34
    canvas = Image.new("L", (im.width, im.height + bar), 255)
    canvas.paste(im, (0, 0))
    d = ImageDraw.Draw(canvas)
    d.line([0, im.height, im.width, im.height], fill=0, width=2)
    font = FONT if d.textlength(text, font=FONT) <= im.width - 12 else FONT_SMALL
    d.text((8, im.height + 7), text, fill=0, font=font)
    return canvas


def row(crops_labels, gap=14) -> Image.Image:
    tiles = [label_strip(c, t) for c, t in crops_labels]
    w = sum(t.width for t in tiles) + gap * (len(tiles) - 1)
    h = max(t.height for t in tiles)
    canvas = Image.new("L", (w, h), 255)
    x = 0
    d = ImageDraw.Draw(canvas)
    for t in tiles:
        canvas.paste(t, (x, 0))
        d.rectangle([x, 0, x + t.width - 1, t.height - 1], outline=180, width=1)
        x += t.width + gap
    return canvas


def js(v) -> str:
    """A style dict as a JS object literal, the way a document writes it."""
    if isinstance(v, dict):
        if not v:
            return "{}"
        return "{ " + ", ".join(f"{k}: {js(x)}" for k, x in v.items()) + " }"
    if isinstance(v, str):
        return json.dumps(v)
    return f"{v:g}" if isinstance(v, float) else str(v)


def style_code(style: dict) -> str:
    return f"style: {js(style)}" + ("  // all defaults" if not style else "")


def compare(fig: str, variants, make_doc, box, scale=1, columns=None):
    """variants: [(arg, label, code)] — code is shown under the tile; None: the style."""
    pngs, tiles = [], []
    for i, (arg, label, code) in enumerate(variants):
        code = code if code is not None else style_code(arg)
        png, svg = render(make_doc(arg), f"{fig}-{i}")
        pngs.append((crop(Image.open(png), box, scale), label))
        name = f"{fig}-{i}.svg"
        crop_svg(str(svg), box, str(SVG_OUT / name))
        tiles.append({"src": f"/docs/svg/{name}", "label": label, "code": code})
    save_png(row(pngs), OUT / f"{fig}.png")
    FIGURES[fig] = {"w": box[2] - box[0], "h": box[3] - box[1], "tiles": tiles}
    if columns:
        FIGURES[fig]["columns"] = columns


def land_doc(draw="lines"):
    def make(style):
        batch = {"angle": 0, "draw": draw, "style": style}
        return "\n".join([
            f'burin.area({{ at: "{AREA}", size_km: {SIZE_KM} }});',
            f"burin.batch({json.dumps(batch)});",
            "burin.draw();",
        ])
    return make


def water_doc(kind, follow="shore", angle=0):
    def make(style):
        opts = {"follow": follow, "angle": angle, "style": style}
        return (
            f'burin.area({{ at: "{AREA}", size_km: {SIZE_KM} }});\n'
            f'burin.water("{kind}", {json.dumps(opts)});\n'
            'burin.batch({ angle: 0, style: {} });\n'
            'burin.draw();'
        )
    return make


def shore_doc(shore):
    return (
        f'burin.area({{ at: "{AREA}", size_km: {SIZE_KM} }});\n'
        f'burin.shore({json.dumps(shore)});\n'
        'burin.batch({ angle: 0, style: {} });\n'
        'burin.draw();'
    )


def figures():
    # The defaults draw by hand (jitter, flat lottery, cutoff); the ink figures
    # turn that off so each shows one rule on its own.
    plain = {"mod_jitter": 0, "mod_flat": 0, "dash_cutoff": 0}
    half = {"dash_light": 0.5, "dash_dark": 1, **plain}
    compare("land-lines-spacing", [
        ({"space_light": 5, "space_dark": 5}, "uniform 5 px", None),
        ({}, "default (6 / 7)", None),
        ({"space_light": 11, "space_dark": 2}, "exaggerated (11 / 2)", None),
    ], land_doc(), LAND_BOX, 2)
    compare("land-lines-ink", [
        ({"dash_light": 1, "dash_dark": 1, **plain}, "solid everywhere", None),
        (half, "even 50% dashes in light", None),
        ({**half, "mod_jitter": 1}, "same ink, by hand", None),
    ], land_doc(), LAND_BOX, 2)
    compare("land-lines-modifiers", [
        (half, "modifiers off", None),
        ({**half, "mod_flat": 1}, "mod_flat: 1", None),
        ({**half, "mod_breaks": 1}, "mod_breaks: 1", None),
    ], land_doc(), LAND_BOX, 2)
    compare("land-lines-cutoff", [
        ({"dash_cutoff": 0}, "no cutoff", None),
        ({}, "default: 0.15, soft", None),
        ({"dash_cutoff_soft": 0}, "0.15, hard edge", None),
    ], land_doc(), LAND_BOX, 2)
    compare("land-lines-levels", [
        (plain, "smooth", None),
        ({"dash_levels": 3, "dash_light": 0.1, "dash_mid": 0.5, "dash_dark": 1, **plain},
         "three levels", None),
    ], land_doc(), LAND_BOX, 2)
    compare("land-dots", [
        ({}, "default", None),
        ({"water_patches": 1}, "water_patches: 1", None),
        ({"mod_jitter": 1}, "mod_jitter: 1 (dot size)", None),
    ], land_doc("dots"), LAND_BOX, 2)
    compare("water-lines", [
        ({}, "default (tight near shore)", None),
        ({"space_light": 2, "space_dark": 2}, "uniform spacing", None),
        ({"space_dark": 1.5, "space_light": 10}, "exaggerated gradient", None),
    ], water_doc("lines"), SHORE_BOX)
    compare("water-ripples", [
        ({}, "default (swell + wave)", None),
        ({"mod_wobble": 0, "mod_wave": 0}, "straight rows", None),
        ({"water_patches": 1}, "water_patches: 1", None),
    ], water_doc("ripples", follow="map", angle=90), SHORE_BOX)
    compare("water-dots", [
        ({}, "default (dense near shore)", None),
        ({"space_dark": 6, "space_light": 6}, "uniform spacing", None),
        ({"water_patches": 1}, "water_patches: 1", None),
    ], water_doc("dots"), SHORE_BOX)
    compare("shoreline", [
        ({"lines": 1}, "one solid line (default)", "burin.shore({ lines: 1 })  // the default"),
        ({"lines": 3, "style": {"space_light": 4}}, "three lines, 4 px apart", "burin.shore({ lines: 3, style: { space_light: 4 } })"),
        ({"lines": 1, "style": {"dash_dark": 0.4, "mod_wobble": 15}}, "dashed and wobbly",
         "burin.shore({ lines: 1, style: { dash_dark: 0.4, mod_wobble: 15 } })"),
    ], shore_doc, SHORE_BOX)


INK_PRESETS = ["ink-classic", "ink-solid", "ink-half", "ink-jitter", "ink-levels", "ink-flat", "ink-thresholds"]
INK_BOX = (1140, 500, 1560, 800)  # hills in the Contours area, light and shadow both


def ink_presets():
    """The presets/ink-*.js documents, cropped to the same few hills."""
    pngs, tiles = [], []
    for name in INK_PRESETS:
        src = ROOT / "presets" / f"{name}.js"
        svg = WORK / f"{name}.svg"
        r = subprocess.run([str(BIN), "--script", str(src), "--out-file", str(svg), "--out-format", "both"],
                           capture_output=True, text=True)
        if r.returncode != 0:
            raise SystemExit(f"{name}: {r.stdout}\n{r.stderr}")
        m = re.search(r"style:\s*(\{.*?\})\s*\}\);", src.read_text(), re.S)
        style = " ".join(m.group(1).split()).replace("{ ", "{ ").replace(", }", " }") if m else "{}"
        pngs.append((crop(Image.open(WORK / f"{name}.png"), INK_BOX, 2), name))
        crop_svg(str(svg), INK_BOX, str(SVG_OUT / f"{name}.svg"))
        tiles.append({"src": f"/docs/svg/{name}.svg", "label": f"presets/{name}.js", "code": f"style: {style}"})
    grid = [row(pngs[i:i + 4]) for i in range(0, len(pngs), 4)]
    out = Image.new("L", (max(g.width for g in grid), sum(g.height for g in grid) + 14 * (len(grid) - 1)), 255)
    y = 0
    for g in grid:
        out.paste(g, (0, y))
        y += g.height + 14
    save_png(out, OUT / "ink-presets.png")
    FIGURES["ink-presets"] = {"w": INK_BOX[2] - INK_BOX[0], "h": INK_BOX[3] - INK_BOX[1], "tiles": tiles, "columns": 3}


# ------------------------------------------------------------------ examples
EXAMPLE_PRESETS = ["contours", "engraved-sea", "sea-depth", "layer-tints", "contour-belts", "downhill", "scripted-hills"]


def examples():
    """Each preset whole, as a simplified SVG."""
    for name in EXAMPLE_PRESETS:
        svg = WORK / "ex" / f"{name}.svg"
        r = subprocess.run([str(BIN), "--script", str(ROOT / "presets" / f"{name}.js"),
                            "--out-file", str(svg), "--out-format", "svg"], capture_output=True, text=True)
        if r.returncode != 0:
            raise SystemExit(f"{name}: {r.stdout}\n{r.stderr}")
        w, h = (float(v) for v in re.search(r'viewBox="0 0 ([\d.]+) ([\d.]+)"', svg.read_text()[:600]).groups())
        crop_svg(str(svg), (0, 0, w, h), str(SVG_OUT / f"example-{name}.svg"), tol=0.3)
        EXAMPLES[name] = {"src": f"/docs/svg/example-{name}.svg", "w": round(w), "h": round(h)}


if __name__ == "__main__":
    only = set(sys.argv[1:])
    if not only or "figures" in only:
        figures()
    if not only or "figures" in only or "ink" in only:
        ink_presets()
    if not only or "examples" in only:
        examples()
    old = json.loads(MANIFEST.read_text()) if MANIFEST.exists() else {"figures": {}, "examples": {}}
    old["figures"].update(FIGURES)
    if EXAMPLES:
        old["examples"] = EXAMPLES
    MANIFEST.write_text(json.dumps(old, indent=2, ensure_ascii=False) + "\n")
    print("figures:", sorted(old["figures"]), "examples:", sorted(old["examples"]))
