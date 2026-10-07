#!/usr/bin/env python3
"""Plug-fit check: will the real ESP32-S3-DevKitC-1 plug into our underside sockets correctly?

Uses Espressif's own CAD file (DXF, board V1.1): it contains every header hole (0.64 mm circles)
and the silkscreen label next to it ("3V3", "RST", "4", ..., "5V", "G"). The script:
  1. reads the 44 hole positions and assigns each label to its hole,
  2. moves the dev board through the exact mounting used on our PCB (component side facing
     away from our board, i.e. seen from above we look at its back; antenna end left, USB end
     right), anchored on the hole labelled 3V3 that is furthest from the USB end,
  3. for every hole, finds our socket pad at that position and checks the pad's net is the right
     one for the label (3V3 -> +3V3, 5V -> +5V_USB, G -> GND, n -> the net we gave GPIOn or no
     connection).
Any hole without a pad within 0.05 mm, or any wrong net, is a failure.

Run:  python3 gbc-handheld/check_plugfit.py <devkit.dxf> [board.kicad_pcb] [netlist.net]
"""
import collections
import math
import os
import sys

import pcbnew

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from verify_netlist import load  # noqa: E402


def read_dxf(path):
    lines = open(path, errors="ignore").read().splitlines()
    pairs = [(lines[i].strip(), lines[i + 1].strip()) for i in range(0, len(lines) - 1, 2)]
    ents, cur = [], None
    for c, v in pairs:
        if c == "0":
            if cur:
                ents.append(cur)
            cur = {"type": v, "d": collections.defaultdict(list)}
        elif cur is not None:
            cur["d"][c].append(v)
    if cur:
        ents.append(cur)
    holes = []
    for e in ents:
        d = e["d"]
        if e["type"] == "LWPOLYLINE" and len(d["10"]) == 2 and d["42"]:
            x0, x1, y0, y1 = float(d["10"][0]), float(d["10"][1]), float(d["20"][0]), float(d["20"][1])
            if abs(y0 - y1) < 1e-6 and 0.62 < abs(x1 - x0) < 0.67:
                holes.append(((x0 + x1) / 2, y0))
    # each hole is drawn as two offset circles; merge pairs into one hole centre per row/position
    merged = collections.defaultdict(list)
    for x, y in holes:
        merged[(0 if x < 12.7 else 1, round(y, 2))].append(x)
    centres = [(sum(xs) / len(xs), y) for (_, y), xs in merged.items()]
    # keep only the two header columns (x = 1.27 and 24.13); drops USB-connector shell holes
    centres = [c for c in centres if min(abs(c[0] - 1.27), abs(c[0] - 24.13)) < 0.3]
    texts = [(e["d"]["1"][0], float(e["d"]["10"][0]), float(e["d"]["20"][0]))
             for e in ents if e["type"] == "TEXT" and e["d"]["1"] and e["d"]["8"]
             and e["d"]["8"][0] == "LAYNR3"]
    return centres, texts


def label_holes(centres, texts):
    """Attach each header label to the hole just above it in the same column."""
    names = {"3V3", "RST", "5V", "G", "TX", "RX"} | {str(n) for n in range(49)}
    out = {}
    for t, tx, ty in texts:
        if t not in names:
            continue
        col = [c for c in centres if abs(c[0] - (1.27 if tx < 12.7 else 24.13)) < 0.4]
        if not col or not (-1.0 < tx < 26.5):
            continue
        best = min(col, key=lambda c: abs(c[1] - (ty + 1.5)) + 10 * abs(c[0] - tx) * 0)
        if abs(best[1] - (ty + 1.5)) < 1.0:
            out.setdefault(best, []).append(t)
    return out


def expected_net(label, gpio_nets):
    if label == "3V3":
        return "+3V3"
    if label == "5V":
        return "+5V_USB"
    if label == "G":
        return "GND"
    if label in ("RST", "TX", "RX"):
        return None
    return gpio_nets.get(int(label))


def main(dxf, board_path, net_path):
    centres, texts = read_dxf(dxf)
    labelled = label_holes(centres, texts)
    fails = []

    def check(cond, msg):
        print(("PASS  " if cond else "FAIL  ") + msg)
        if not cond:
            fails.append(msg)

    check(len(centres) == 44, f"44 header holes in Espressif's DXF (found {len(centres)})")
    check(len(labelled) == 44 and all(len(v) == 1 for v in labelled.values()),
          f"every hole has exactly one label (labelled {len(labelled)})")

    # GPIO -> net as assigned in our schematic (read back from the netlist via the header maps)
    comps, nets = load(net_path)
    b = pcbnew.LoadBoard(board_path)
    pads = []
    for ref in ("J1", "J3"):
        fp = b.FindFootprintByReference(ref)
        for p in fp.Pads():
            pos = p.GetPosition()
            pads.append((ref, p.GetNumber(), pcbnew.ToMM(pos.x), pcbnew.ToMM(pos.y), p.GetNetname()))

    # Transform DXF (component side, Y up, antenna at +Y) -> our board (seen from the top, y down):
    # mirror (we see the back), rotate so the antenna points to -x, flip to y-down.
    mirror = os.environ.get("PLUGFIT_SELFTEST") != "nomirror"

    def to_board(x, y):
        xm, ym = (-x, y) if mirror else (x, y)   # back view (self-test: deliberately wrong)
        xr, yr = -ym, xm               # rotate +90 deg (antenna +Y -> -X)
        return xr, -yr                 # y down

    anchor = max((c for c, l in labelled.items() if l == ["3V3"]), key=lambda c: c[1])
    j1p1 = [p for p in pads if p[0] == "J1" and p[1] == "1"][0]
    ax, ay = to_board(*anchor)
    dx, dy = j1p1[2] - ax, j1p1[3] - ay

    ok = 0
    for (hx, hy), (label,) in sorted(labelled.items(), key=lambda kv: (kv[0][0], -kv[0][1])):
        bx, by = to_board(hx, hy)
        bx, by = bx + dx, by + dy
        near = min(pads, key=lambda p: math.hypot(p[2] - bx, p[3] - by))
        dist = math.hypot(near[2] - bx, near[3] - by)
        if dist > 0.05:
            check(False, f"hole '{label}' lands {dist:.2f} mm from the nearest pad")
            continue
        net = near[4]
        if label in ("3V3", "5V", "G", "RST", "TX", "RX"):
            want = expected_net(label, {})
            good = (net == want) if want else (net == "" or net.lower().startswith("unconnected"))
        else:
            # A numbered pin must carry a net whose ESP32 pin really is that GPIO: look it up in
            # the generated pin map (build/pinmap.md), which lists GPIOn -> signal.
            want = PINMAP.get(int(label))
            good = (net == want) if want else (net == "" or net.lower().startswith("unconnected"))
        if good:
            ok += 1
        else:
            check(False, f"label {label}: our pad {near[0]}.{near[1]} has net '{net}', expected '{want}'")
    check(ok == 44, f"{ok}/44 dev-board pins land on a pad with the correct net")
    print(f"\n{len(fails)} failure(s)")
    return 1 if fails else 0


def read_pinmap(path):
    import re
    m = {}
    for line in open(path):
        hit = re.match(r"\|\s*GPIO(\d+)\s*\|\s*([^|]+?)\s*\|", line)
        if hit and not hit.group(2).startswith("reserved"):
            m[int(hit.group(1))] = hit.group(2)
    return m


if __name__ == "__main__":
    dxf = sys.argv[1]
    board = sys.argv[2] if len(sys.argv) > 2 else os.path.join(HERE, "build", "gbc_handheld.kicad_pcb")
    net = sys.argv[3] if len(sys.argv) > 3 else os.path.join(HERE, "build", "gbc_handheld.net")
    PINMAP = read_pinmap(os.path.join(os.path.dirname(net), "pinmap.md"))
    sys.exit(main(dxf, board, net))
