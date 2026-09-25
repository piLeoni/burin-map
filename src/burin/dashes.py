"""Break streamlines into dashes whose length follows the tone.

Each line is cut into periods of roughly `period × spacing` px, with a
random phase so neighbouring lines don't line up. In each period the dash
covers a fraction of it between duty_min and duty_max, set by the tone at the
period's midpoint: in shadow the dashes merge into a solid line, on flat
ground they become short stipple, on lit slopes they vanish.

Below the flat tone, dashes don't get any shorter (a very short dash still
reads as a dot, so lit slopes would stay as busy as the plain); instead they
keep the flat length and are dropped at random, leaving lit slopes mostly
white with a few scattered strokes.

Pitch and dash length are randomised by `jitter` so the stipple doesn't read
as a machine-made grid. The dash length factor is gamma-distributed (mean 1,
std = jitter): mostly near average, sometimes a dot, sometimes a long stroke.
Jitter fades out as tone approaches 1 so solid shadows stay solid; there
`breaks` opens an occasional small gap so long lines aren't unbroken wire.
"""

from __future__ import annotations

import numpy as np

from .style import Style


def dash_lines(lines: list[np.ndarray], tone: np.ndarray, st: Style) -> list[np.ndarray]:
    rng = np.random.default_rng(st.seed)
    H, W = tone.shape
    base = st.period * st.spacing
    out: list[np.ndarray] = []
    for line in lines:
        s = np.concatenate([[0.0], np.cumsum(np.hypot(*np.diff(line, axis=0).T))])
        total = s[-1]
        cur = -rng.random() * base
        while cur < total:
            period = base * rng.uniform(1.0 - st.jitter, 1.0 + st.jitter)
            period = max(period, 0.3 * base)
            mid = min(cur + period * 0.5, total)
            mx, my = np.interp(mid, s, line[:, 0]), np.interp(mid, s, line[:, 1])
            t = tone[min(int(my), H - 1), min(int(mx), W - 1)]
            if st.cutoff < t < st.flat_level:
                t = st.flat_level if rng.random() * st.flat_level < t else 0.0
            if t > st.cutoff:
                duty = st.duty_min + (st.duty_max - st.duty_min) * t
                j = st.jitter * (1.0 - t)
                spread = rng.gamma(1.0 / j ** 2, j ** 2) if j > 1e-3 else 1.0
                a = max(cur, 0.0)
                b = cur + period * duty * spread
                if b >= cur + period and rng.random() < st.breaks:
                    # must beat the round caps (stroke/2 each side) to show
                    b = cur + period - st.stroke * rng.uniform(1.5, 3.0)
                b = min(b, total)
                if b - a > 0.6:
                    ss = np.concatenate([[a], s[(s > a) & (s < b)], [b]])
                    out.append(np.column_stack([np.interp(ss, s, line[:, 0]),
                                                np.interp(ss, s, line[:, 1])]))
            cur += period
    return out
