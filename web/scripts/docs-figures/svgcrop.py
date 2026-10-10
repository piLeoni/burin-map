"""Cut a burin-map SVG down to a box: drop paths outside it, simplify, shift and round the rest."""
import re

TOK = re.compile(r"[MLZ]|-?\d+(?:\.\d+)?")
D = re.compile(r'd="([^"]*)"')


def _subpaths(d: str):
    """[(points, closed)] from an absolute M/L/Z path."""
    out, pts, nums = [], None, []
    for tok in TOK.findall(d):
        if tok == "M":
            if pts:
                out.append((pts, False))
            pts = []
        elif tok == "L":
            pass
        elif tok == "Z":
            if pts:
                out.append((pts, True))
            pts = None
        else:
            nums.append(float(tok))
            if len(nums) == 2:
                if pts is None:
                    pts = []
                pts.append((nums[0], nums[1]))
                nums = []
    if pts:
        out.append((pts, False))
    return out


def _dp(pts, tol):
    if len(pts) < 3:
        return pts
    keep = [False] * len(pts)
    keep[0] = keep[-1] = True
    stack = [(0, len(pts) - 1)]
    tol2 = tol * tol
    while stack:
        a, b = stack.pop()
        ax, ay = pts[a]
        bx, by = pts[b]
        dx, dy = bx - ax, by - ay
        L2 = dx * dx + dy * dy
        best, bi = -1.0, -1
        for i in range(a + 1, b):
            px, py = pts[i]
            if L2 == 0:
                d2 = (px - ax) ** 2 + (py - ay) ** 2
            else:
                c = (px - ax) * dy - (py - ay) * dx
                d2 = c * c / L2
            if d2 > best:
                best, bi = d2, i
        if best > tol2:
            keep[bi] = True
            stack.append((a, bi))
            stack.append((bi, b))
    return [p for p, k in zip(pts, keep) if k]


def _fmt(v: float) -> str:
    s = f"{v:.1f}"
    if s.endswith(".0"):
        s = s[:-2]
    return "0" if s == "-0" else s


def _touches(subs, box, margin):
    x0, y0, x1, y1 = box
    for pts, _ in subs:
        xs = [p[0] for p in pts]
        ys = [p[1] for p in pts]
        if max(xs) >= x0 - margin and min(xs) <= x1 + margin and max(ys) >= y0 - margin and min(ys) <= y1 + margin:
            if len(pts) > 50:
                return True
            for x, y in pts:
                if x0 - margin <= x <= x1 + margin and y0 - margin <= y <= y1 + margin:
                    return True
    return False


def _path(subs, x0, y0, tol):
    parts = []
    for pts, closed in subs:
        pts = _dp(pts, tol)
        coords = " ".join(f"{_fmt(x - x0)} {_fmt(y - y0)}" for x, y in pts)
        parts.append("M" + coords + ("Z" if closed else ""))
    return "".join(parts)


def crop_svg(src: str, box, dst: str, margin: float = 12.0, tol: float = 0.2) -> int:
    x0, y0, x1, y1 = box
    w, h = x1 - x0, y1 - y0
    lines = []
    with open(src) as f:
        for line in f:
            s = line.strip()
            if s.startswith("<svg"):
                lines.append(
                    f'<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" '
                    f'viewBox="0 0 {w} {h}" fill="none" stroke-linecap="round" stroke-linejoin="round">'
                )
                lines.append(f'<rect width="{w}" height="{h}" fill="#fff"/>')
            elif s.startswith("<g"):
                attrs = re.findall(r'(stroke|stroke-width|fill|fill-rule)="([^"]*)"', s)
                lines.append("<g " + " ".join(f'{k}="{v}"' for k, v in attrs) + ">")
            elif s.startswith("</g"):
                if lines[-1].startswith("<g "):
                    lines.pop()
                else:
                    lines.append("</g>")
            elif s.startswith("<path"):
                m = D.search(s)
                if not m:
                    continue
                subs = _subpaths(m.group(1))
                if subs and _touches(subs, box, margin):
                    lines.append(f'<path d="{_path(subs, x0, y0, tol)}"/>')
            elif s.startswith("</svg"):
                lines.append("</svg>")
    text = "\n".join(lines) + "\n"
    with open(dst, "w") as f:
        f.write(text)
    return len(text)
