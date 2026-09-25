"""Command line: `burin --at "Twin Peaks, San Francisco" --size 20 -o sf.png`."""

from __future__ import annotations

import argparse
from pathlib import Path

from . import Style, __version__, draw
from .geo import bounds_around, locate
from .style import style_fields

PLACES = {
    "bern": (7.10, 46.70, 7.80, 47.20),
    "matterhorn": (7.40, 45.80, 8.00, 46.20),
    "san-francisco": (-122.75, 37.45, -122.05, 38.05),
}


def main(argv: list[str] | None = None) -> None:
    ap = argparse.ArgumentParser(
        prog="burin",
        description="Pen-and-ink terrain map from SRTM elevation data.",
        formatter_class=argparse.ArgumentDefaultsHelpFormatter)
    ap.add_argument("--version", action="version", version=__version__)
    where = ap.add_mutually_exclusive_group()
    where.add_argument("--place", choices=sorted(PLACES), default="san-francisco", help="named area")
    where.add_argument("--bounds", nargs=4, type=float, metavar=("W", "S", "E", "N"),
                       help="area in degrees: west south east north")
    where.add_argument("--at", metavar="WHERE",
                       help='centre: an address or place name, or "lat,lon"')
    ap.add_argument("--size", type=float, default=30.0, metavar="KM",
                    help="with --at: area width in km (height follows the page, else square)")
    ap.add_argument("-o", "--output", default="burin.png", help="PNG path")
    ap.add_argument("--svg", action="store_true", help="also write an SVG next to the PNG")
    page = ap.add_argument_group("page (plotter / print); overrides --width and sizes the SVG in mm")
    page.add_argument("--page-width", type=float, metavar="MM", help="drawing width on paper, mm")
    page.add_argument("--page-height", type=float, metavar="MM",
                      help="max drawing height, mm; the drawing shrinks to fit")
    page.add_argument("--pen", type=float, default=0.3, metavar="MM", help="pen tip width, mm")

    style = ap.add_argument_group("style")
    for name, default, help in style_fields():
        style.add_argument(f"--{name.replace('_', '-')}", type=type(default),
                           default=default, help=help)
    args = ap.parse_args(argv)

    st = Style(**{name: getattr(args, name) for name, _, _ in style_fields()})
    if args.at:
        lat, lon, label = locate(args.at)
        aspect = (args.page_height / args.page_width
                  if args.page_width and args.page_height else 1.0)
        bounds = bounds_around(lat, lon, args.size, args.size * aspect)
        print(f"{label}\n  {args.size:g} × {args.size * aspect:g} km")
    elif args.bounds:
        bounds = tuple(args.bounds)
    else:
        bounds = PLACES[args.place]
    d = draw(bounds, st, page_width_mm=args.page_width,
             page_height_mm=args.page_height, pen_mm=args.pen, log=print)

    out = Path(args.output)
    d.save_png(out)
    print(f"wrote {out}")
    if args.svg:
        d.save_svg(out.with_suffix(".svg"))
        print(f"wrote {out.with_suffix('.svg')}")


if __name__ == "__main__":
    main()
