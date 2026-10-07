#!/usr/bin/env python3
"""Independent check of the routed board against the netlist and fab limits.

Compares every pad's net in build/gbc_handheld.kicad_pcb with build/gbc_handheld.net (read by
verify_netlist.load), and checks track widths, hole sizes, board size and layer use.
Run:  python3 gbc-handheld/verify_board.py [board.kicad_pcb] [netlist.net]
"""
import os
import sys

import pcbnew

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
from verify_netlist import load  # noqa: E402

# JLCPCB standard 2-layer limits we stay above (see spec): track/space 0.127, drill 0.3
MIN_TRACK, MIN_DRILL, MAX_BOARD = 0.2, 0.3, (100.0, 100.0)


def main(board_path, net_path):
    b = pcbnew.LoadBoard(board_path)
    comps, nets = load(net_path)
    fails = []

    def check(cond, msg):
        print(("PASS  " if cond else "FAIL  ") + msg)
        if not cond:
            fails.append(msg)

    # 1. Footprints and pad nets match the netlist exactly
    board_fps = {f.GetReference(): f for f in b.GetFootprints() if not f.GetReference().startswith("H")}
    check(set(board_fps) == set(comps), f"board has the same {len(comps)} parts as the netlist")
    wrong = []
    for net_name, nodes in nets.items():
        for ref, pin in nodes:
            pads = [p for p in board_fps[ref].Pads() if p.GetNumber() == pin]
            if not pads:
                wrong.append((ref, pin, "no such pad"))
                continue
            for p in pads:
                got = p.GetNetname()
                # no-connect pads appear in the netlist as "unconnected-(...)" nets
                if net_name.lower().startswith("unconnected"):
                    if got not in ("", net_name) and not got.lower().startswith("unconnected"):
                        wrong.append((ref, pin, f"expected NC, board has {got}"))
                elif got != net_name:
                    wrong.append((ref, pin, f"netlist {net_name}, board {got}"))
    check(not wrong, f"every pad's net matches the netlist ({len(wrong)} mismatches) {wrong[:3]}")

    # 2. Connectivity (KiCad's own engine)
    b.BuildConnectivity()
    check(b.GetConnectivity().GetUnconnectedCount(True) == 0, "no unrouted connections")

    # 3. Track widths: power nets wide, everything above the minimum
    power = {"+5V_USB", "+5V_SW", "+3V3"}
    tracks = [t for t in b.GetTracks() if t.Type() == pcbnew.PCB_TRACE_T]
    narrow = [t for t in tracks if pcbnew.ToMM(t.GetWidth()) < MIN_TRACK - 1e-6]
    check(not narrow, f"all {len(tracks)} tracks >= {MIN_TRACK} mm")
    pw = [t for t in tracks if t.GetNetname() in power]
    thin_pw = [t for t in pw if pcbnew.ToMM(t.GetWidth()) < 0.5]
    check(len(pw) > 0 and not thin_pw,
          f"{len(pw)} power track segments, all >= 0.5 mm (min {min(pcbnew.ToMM(t.GetWidth()) for t in pw):.2f})")

    # 4. Holes and vias
    drills = [pcbnew.ToMM(p.GetDrillSize().x) for f in b.GetFootprints() for p in f.Pads()
              if p.GetDrillSize().x > 0]
    vias = [t for t in b.GetTracks() if t.Type() == pcbnew.PCB_VIA_T]
    vdrills = [pcbnew.ToMM(v.GetDrillValue()) for v in vias]
    check(min(drills + vdrills) >= MIN_DRILL, f"smallest hole {min(drills + vdrills):.2f} mm >= {MIN_DRILL} mm")

    # 5. Board size and layers
    bb = b.GetBoardEdgesBoundingBox()
    w, h = pcbnew.ToMM(bb.GetWidth()), pcbnew.ToMM(bb.GetHeight())
    check(w <= MAX_BOARD[0] and h <= MAX_BOARD[1], f"board {w:.1f} x {h:.1f} mm within 100 x 100")
    check(b.GetCopperLayerCount() == 2, "2 copper layers")
    check(all(not f.GetFPIDAsString().count("SMD") for f in b.GetFootprints()), "no SMD footprints")
    under = sorted(r for r, f in board_fps.items() if f.IsFlipped())
    check(under == ["J1", "J3"], f"only the ESP32 sockets are on the underside: {under}")

    print(f"\n{len(fails)} failure(s)")
    return 1 if fails else 0


if __name__ == "__main__":
    bp = sys.argv[1] if len(sys.argv) > 1 else os.path.join(HERE, "build", "gbc_handheld.kicad_pcb")
    np_ = sys.argv[2] if len(sys.argv) > 2 else os.path.join(HERE, "build", "gbc_handheld.net")
    sys.exit(main(bp, np_))
