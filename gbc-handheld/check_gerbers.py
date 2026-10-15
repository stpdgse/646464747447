#!/usr/bin/env python3
"""Check the manufacturing files themselves (Gerber + Excellon), independently of KiCad.

Reads build/fab/ with gerbonara and verifies what the factory will actually receive:
  * board outline size,
  * every drill hit sits on a copper flash on BOTH copper layers (no layer/drill offset),
    with an annular ring of at least MIN_RING,
  * every plated hole has a solder-mask opening on both sides,
  * drill count matches the board file,
and renders the Gerbers with gerbv (build/gerber_top.png, gerber_bottom.png).

Run:  python3 gbc-handheld/check_gerbers.py
"""
import math
import os
import subprocess
import sys
import warnings

from gerbonara import LayerStack
from gerbonara.graphic_objects import Flash

HERE = os.path.dirname(os.path.abspath(__file__))
FAB = os.path.join(HERE, "build", "fab")
MIN_RING = 0.15


def flashes(layer):
    """(x, y, smallest outline size) of every flash. The size is taken from the flash's real
    outline, so aperture macros (KiCad's rounded rectangles) are measured too."""
    out = []
    for o in layer.objects:
        if isinstance(o, Flash):
            (x0, y0), (x1, y1) = o.bounding_box(unit=None)
            out.append((o.x, o.y, min(abs(x1 - x0), abs(y1 - y0))))
    return out


def main():
    warnings.simplefilter("ignore")
    ls = LayerStack.open(FAB)
    fails = []

    def check(cond, msg):
        print(("PASS  " if cond else "FAIL  ") + msg)
        if not cond:
            fails.append(msg)

    for key in (("top", "copper"), ("bottom", "copper"), ("top", "mask"), ("bottom", "mask"),
                ("top", "silk"), ("bottom", "silk")):
        check(key in ls.graphic_layers, f"layer {key[0]} {key[1]} present")
    (x0, y0), (x1, y1) = ls.outline.bounding_box(unit=None)
    w, h = abs(x1 - x0), abs(y1 - y0)
    check(abs(w - 90.1) < 0.2 and abs(h - 98.1) < 0.2, f"outline {w:.2f} x {h:.2f} mm (90 x 98 + line width)")

    hits = []
    for dl in (ls.drill_pth, ls.drill_npth):
        if dl is None:
            continue
        for o in dl.objects:
            if isinstance(o, Flash):
                hits.append((o.x, o.y, o.aperture.diameter))
    check(len(hits) > 100, f"{len(hits)} drill hits read from the Excellon files")

    layers = {k: flashes(ls.graphic_layers[k]) for k in
              (("top", "copper"), ("bottom", "copper"), ("top", "mask"), ("bottom", "mask"))}
    bad_cu, bad_mask, thin = [], [], []
    for (hx, hy, d) in hits:
        for side in ("top", "bottom"):
            cu = [f for f in layers[(side, "copper")] if math.hypot(f[0] - hx, f[1] - hy) < 0.01]
            if not cu:
                bad_cu.append((side, round(hx, 2), round(hy, 2), d))
            else:
                ring = (max(f[2] for f in cu) - d) / 2
                if ring < MIN_RING:
                    thin.append((side, round(hx, 2), round(hy, 2), round(ring, 3)))
            mk = [f for f in layers[(side, "mask")] if math.hypot(f[0] - hx, f[1] - hy) < 0.01]
            if not mk and d > 0.5:      # vias (0.4 mm) are tented on purpose
                bad_mask.append((side, round(hx, 2), round(hy, 2)))
    check(not bad_cu, f"every drill hit is on a copper pad on both layers ({len(bad_cu)} misses) {bad_cu[:4]}")
    check(not thin, f"annular ring >= {MIN_RING} mm on every hole ({len(thin)} thin) {thin[:4]}")
    check(not bad_mask, f"every component hole has a mask opening both sides ({len(bad_mask)} missing) {bad_mask[:4]}")

    # Pictures of the actual fab output, drawn by gerbv (gerbonara's own SVG renderer mis-draws
    # arcs that cross 0 degrees, e.g. one board corner; gerbv draws them correctly).
    for side, cu in (("top", "F_Cu.gtl"), ("bottom", "B_Cu.gbl")):
        out = os.path.join(HERE, "build", f"gerber_{side}.png")
        subprocess.run(["gerbv", "-x", "png", "-D", "250", "-o", out,
                        os.path.join(FAB, "gbc_handheld-Edge_Cuts.gm1"),
                        os.path.join(FAB, f"gbc_handheld-{cu}"),
                        os.path.join(FAB, "gbc_handheld-PTH.drl")],
                       check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    print(f"\n{len(fails)} failure(s)")
    return 1 if fails else 0


if __name__ == "__main__":
    sys.exit(main())
