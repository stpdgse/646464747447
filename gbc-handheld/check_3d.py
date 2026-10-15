#!/usr/bin/env python3
"""3D fit check: parts, plug-in modules, standoffs and access paths, with heights.

KiCad's DRC is 2D. This builds a box for every part and module (x, y from the real board,
z from the height table below) and checks the physical questions DRC cannot answer.
z = 0 is the TOP surface of the PCB, the bottom surface is z = -1.6, up is +z.

Heights are the parts' datasheet / typical values; every one is listed in HEIGHTS so it can be
checked against the actual parts bought. Writes build/fit_top.png and build/fit_sections.png.

Run:  python3 gbc-handheld/check_3d.py [board.kicad_pcb]
"""
import math
import os
import sys

import pcbnew

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, HERE)
import build_board as BB  # noqa: E402  (shares the module geometry constants)

PCB_T = 1.6
SOCKET_H = 8.5          # standard 2.54 mm female header body height (BOM requires this)
HDR_PLASTIC = 2.54      # male header spacer on the modules (MSP2807 drawing: 8.38 - 5.84)
MODULE_Z = SOCKET_H + HDR_PLASTIC          # 11.04: module PCB underside above our board
LCD_BACK_PARTS = 2.20                      # MSP2807 drawing: "2.20 max (SMD)" on its back
LCD_LOW = MODULE_Z - LCD_BACK_PARTS        # 8.84: lowest point of the LCD module
AMP_BACK_PARTS = 1.0                       # assumption: breakout underside parts <= 1.0 mm
AMP_LOW = MODULE_Z - AMP_BACK_PARTS
LEAD_PROTRUSION = 2.0                      # clipped THT leads on the far side
SCREW_HEAD_H, SCREW_HEAD_R = 2.5, 2.75     # M3 pan/cheese head
STANDOFF_R = 3.2                           # M3 hex standoff, 6.4 mm across corners

# Height of each footprint type above its own board face (mm)
HEIGHTS = {
    "Button_Switch_THT:SW_PUSH_6mm": 5.0,          # 6x6x5 mm tactile (BOM: hoehe 5 mm)
    "gbc:SW_Slide_SS12D00G3": 6.8,                 # body 3.8 + handle 3.0
    "Capacitor_THT:CP_Radial_D5.0mm_P2.50mm": 12.0,  # 5x11 mm can + 1 mm stand-off
    "Capacitor_THT:C_Disc_D5.0mm_W2.5mm_P5.00mm": 8.0,  # ceramic radial, must be <= 8 mm
    "Resistor_THT:R_Axial_DIN0207_L6.3mm_D2.5mm_P7.62mm_Horizontal": 3.0,
    "LED_THT:LED_D3.0mm": 6.3,
    "Connector_PinSocket_2.54mm:PinSocket_1x22_P2.54mm_Vertical": SOCKET_H,
    "Connector_PinSocket_2.54mm:PinSocket_1x14_P2.54mm_Vertical": SOCKET_H,
    "Connector_PinSocket_2.54mm:PinSocket_1x07_P2.54mm_Vertical": SOCKET_H,
    "Connector_PinHeader_2.54mm:PinHeader_1x04_P2.54mm_Vertical": 8.5,   # 2.5 plastic + 6 pin
}


def bbox_mm(fp):
    bb = fp.GetBoundingBox(False)
    x0 = pcbnew.ToMM(bb.GetX()) - BB.OX
    y0 = pcbnew.ToMM(bb.GetY()) - BB.OY
    return (x0, y0, x0 + pcbnew.ToMM(bb.GetWidth()), y0 + pcbnew.ToMM(bb.GetHeight()))


def overlap_xy(a, b, margin=0.0):
    return a[0] < b[2] + margin and b[0] < a[2] + margin and a[1] < b[3] + margin and b[1] < a[3] + margin


def main(board_path):
    b = pcbnew.LoadBoard(board_path)
    fails, notes = [], []

    def check(cond, msg):
        print(("PASS  " if cond else "FAIL  ") + msg)
        if not cond:
            fails.append(msg)

    parts = {}
    for fp in b.GetFootprints():
        ref = fp.GetReference()
        if ref.startswith("H"):
            continue
        fpid = fp.GetFPIDAsString()
        if fpid not in HEIGHTS:
            check(False, f"{ref}: no height known for {fpid}")
            continue
        parts[ref] = {"box": bbox_mm(fp), "h": HEIGHTS[fpid], "bottom": fp.IsFlipped(), "fp": fp}
    holes = [(x, y) for x, y in BB.LCD_HOLES]

    lcd_box = (BB.LCD_X, BB.LCD_Y, BB.LCD_X + BB.LCD_W, BB.LCD_Y + BB.LCD_H)
    amp_box = BB.AMP_BOX
    esp_box = (BB.ESP_X0 - 1.44, BB.ESP_YC - 12.7, BB.ESP_USB_END_X, BB.ESP_YC + 12.7)

    # 1. Nothing on top under the LCD reaches its underside (J_LCD is its own socket)
    worst = None
    for ref, p in parts.items():
        if p["bottom"] or ref == "J_LCD" or not overlap_xy(p["box"], lcd_box):
            continue
        gap = LCD_LOW - p["h"]
        worst = gap if worst is None else min(worst, gap)
        check(gap > 0.3, f"{ref} under the LCD: {p['h']:.1f} mm tall, {gap:.2f} mm below the LCD's lowest part")
    # 2. Standoffs clear of parts (top) and screw heads clear (bottom)
    for ref, p in parts.items():
        for hx, hy in holes:
            ring = (hx - STANDOFF_R, hy - STANDOFF_R, hx + STANDOFF_R, hy + STANDOFF_R)
            if overlap_xy(p["box"], ring):
                d = math.hypot(max(p["box"][0] - hx, 0, hx - p["box"][2]),
                               max(p["box"][1] - hy, 0, hy - p["box"][3]))
                check(d >= STANDOFF_R, f"{ref} body is {d:.2f} mm from the standoff at ({hx},{hy}), needs {STANDOFF_R}")
    check(True, "standoff/screw-head zones checked for all parts")
    # 3. Amplifier module, body pointing down: nothing under it taller than its underside
    for ref, p in parts.items():
        if p["bottom"] or ref == "J_AMP" or not overlap_xy(p["box"], amp_box):
            continue
        check(p["h"] < AMP_LOW - 0.3, f"{ref} under the amp module: {p['h']:.1f} mm vs {AMP_LOW:.1f} mm")
    check(not overlap_xy(amp_box, lcd_box), "amp module (body down) does not overlap the LCD")
    flipped = (amp_box[0], BB.AMP_ROW_Y + BB.AMP_EDGE - BB.AMP_D, amp_box[2], BB.AMP_ROW_Y + BB.AMP_EDGE)
    collide_up = overlap_xy(flipped, lcd_box)
    print(f"INFO  if the amp module were plugged in pointing UP it would "
          f"{'HIT the LCD (y ' + format(flipped[1], '.1f') + ' < 54)' if collide_up else 'not hit the LCD'}")
    # 4. ESP32 board underneath: nothing on our underside reaches its header plastic
    esp_top = -PCB_T - SOCKET_H                    # dev board header plastic starts here
    for ref, p in parts.items():
        if ref in ("J1", "J3"):
            continue
        low = -PCB_T - (p["h"] if p["bottom"] else LEAD_PROTRUSION)
        if overlap_xy(p["box"], esp_box):
            check(low > esp_top + 0.3, f"{ref} leads/body under the ESP32 board reach z={low:.1f} (limit {esp_top:.1f})")
    for hx, hy in holes:
        sc = (hx - SCREW_HEAD_R, hy - SCREW_HEAD_R, hx + SCREW_HEAD_R, hy + SCREW_HEAD_R)
        check(not overlap_xy(sc, esp_box), f"screw head at ({hx},{hy}) clear of the ESP32 board")
    check(esp_box[2] - BB.BW >= 3.0, f"ESP32 USB end overhangs the board edge by {esp_box[2] - BB.BW:.1f} mm")
    # 5. SD card path: from the board's top edge into the slot, under the LCD
    path = (BB.SD_SLOT_BOX[0], 0.0, BB.SD_SLOT_BOX[2], BB.SD_SLOT_BOX[3])
    for ref, p in parts.items():
        h = LEAD_PROTRUSION if p["bottom"] else p["h"]
        if overlap_xy(p["box"], path) and h > LCD_LOW - 0.5:
            check(False, f"{ref} ({h:.1f} mm) blocks the SD card path")
    check(all(not overlap_xy(((hx - STANDOFF_R, hy - STANDOFF_R, hx + STANDOFF_R, hy + STANDOFF_R)), path)
              for hx, hy in holes), "SD card path is clear of the standoffs")
    # 6. Buttons and the slide switch are reachable (not under a module)
    for ref in [r for r in parts if r.startswith("SW")]:
        under = [n for n, bx in (("LCD", lcd_box), ("amp", amp_box)) if overlap_xy(parts[ref]["box"], bx)]
        check(not under, f"{ref} is reachable (not under {', '.join(under) if under else 'any module'})")
    # 7. Hand-solderability: pad edge gaps between different nets, annular rings
    pads = []
    for fp in b.GetFootprints():
        for pd in fp.Pads():
            c = pd.GetPosition()
            pads.append((fp.GetReference(), pd.GetNumber(), pcbnew.ToMM(c.x), pcbnew.ToMM(c.y),
                         pcbnew.ToMM(max(pd.GetSize().x, pd.GetSize().y)) / 2, pd.GetNetname(),
                         pcbnew.ToMM(pd.GetSize().x), pcbnew.ToMM(pd.GetDrillSize().x)))
    min_gap, where, tight = 99, None, []
    for i in range(len(pads)):
        for j in range(i + 1, len(pads)):
            a, c = pads[i], pads[j]
            if a[5] and a[5] == c[5]:
                continue
            gap = math.hypot(a[2] - c[2], a[3] - c[3]) - a[4] - c[4]
            if gap < 0.6:
                tight.append((round(gap, 2), a[:2], c[:2]))
            if gap < min_gap:
                min_gap, where = gap, (a[:2], c[:2])
    for gap, a, c in sorted(tight):
        check(False, f"pads {a} and {c} only {gap} mm apart (different nets, < 0.6 for hand soldering)")
    check(min_gap >= 0.6, f"smallest gap between pads of different nets {min_gap:.2f} mm at {where} (>= 0.6 for hand soldering)")
    rings = [(r[0], r[1], (r[6] - r[7]) / 2) for r in pads if r[7] > 0]
    worst_ring = min(rings, key=lambda r: r[2])
    check(worst_ring[2] >= 0.25, f"smallest annular ring {worst_ring[2]:.2f} mm ({worst_ring[0]} pad {worst_ring[1]}) >= 0.25")
    vias = [t for t in b.GetTracks() if t.Type() == pcbnew.PCB_VIA_T]
    if vias:
        vr = min((pcbnew.ToMM(v.GetWidth(pcbnew.F_Cu)) - pcbnew.ToMM(v.GetDrillValue())) / 2 for v in vias)
        check(vr >= 0.15, f"via annular ring {vr:.2f} mm >= 0.15 (JLCPCB minimum 0.13)")

    draw(parts, holes, lcd_box, amp_box, esp_box)
    print(f"\n{len(fails)} failure(s)")
    return 1 if fails else 0


def draw(parts, holes, lcd_box, amp_box, esp_box):
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    from matplotlib.patches import Circle, Rectangle

    # Top view: parts coloured by height, modules outlined
    fig, ax = plt.subplots(figsize=(8, 8.8))
    ax.add_patch(Rectangle((0, 0), BB.BW, BB.BH, fill=False, lw=2, ec="black"))
    cmap = plt.get_cmap("viridis")
    for ref, p in parts.items():
        x0, y0, x1, y1 = p["box"]
        col = "#999999" if p["bottom"] else cmap(min(p["h"], 12) / 12)
        ax.add_patch(Rectangle((x0, y0), x1 - x0, y1 - y0, fc=col, ec="k", lw=0.5,
                               alpha=0.35 if p["bottom"] else 0.85, hatch="//" if p["bottom"] else None))
        if not p["bottom"]:
            ax.text((x0 + x1) / 2, (y0 + y1) / 2, f"{ref}\n{p['h']:.1f}", ha="center", va="center", fontsize=5.5)
    for box, name, colr in ((lcd_box, "LCD module, underside 8.8 mm", "red"),
                            (amp_box, "amp module, underside 10 mm", "blue"),
                            (esp_box, "ESP32 board (underneath)", "gray"),
                            (BB.SD_SLOT_BOX, "SD slot", "orange")):
        ax.add_patch(Rectangle((box[0], box[1]), box[2] - box[0], box[3] - box[1], fill=False, ec=colr, lw=1.6, ls="--"))
        ax.text(box[0] + 0.5, box[1] + 1.2, name, color=colr, fontsize=7)
    for hx, hy in holes:
        ax.add_patch(Circle((hx, hy), STANDOFF_R, fill=False, ec="purple", lw=1.2))
    ax.set_xlim(-2, 98)
    ax.set_ylim(BB.BH + 2, -2)
    ax.set_aspect("equal")
    ax.set_title("Top view: part heights in mm (colour), modules dashed, standoffs purple")
    sm = plt.cm.ScalarMappable(cmap=cmap, norm=plt.Normalize(0, 12))
    fig.colorbar(sm, ax=ax, fraction=0.035, label="height above board (mm)")
    fig.savefig(os.path.join(BB.BUILD, "fit_top.png"), dpi=130, bbox_inches="tight")
    plt.close(fig)

    # Side sections: x-z cut through the ESP32 rows (y = 29) and the controls (y = 70)
    fig, axes = plt.subplots(2, 1, figsize=(10, 7))
    for ax, ycut, title in ((axes[0], BB.ESP_YC, f"Section at y = {BB.ESP_YC} mm (LCD above, ESP32 below)"),
                            (axes[1], 70.0, "Section at y = 70 mm (buttons, amp module)")):
        ax.add_patch(Rectangle((0, -PCB_T), BB.BW, PCB_T, fc="#2e7d32"))
        for ref, p in parts.items():
            x0, y0, x1, y1 = p["box"]
            if not (y0 <= ycut <= y1):
                continue
            if p["bottom"]:
                ax.add_patch(Rectangle((x0, -PCB_T - p["h"]), x1 - x0, p["h"], fc="#777", alpha=0.8))
            else:
                ax.add_patch(Rectangle((x0, 0), x1 - x0, p["h"], fc="#1565c0", alpha=0.75))
                ax.text((x0 + x1) / 2, p["h"] + 0.3, ref, ha="center", fontsize=6, rotation=90)
        if lcd_box[1] <= ycut <= lcd_box[3]:
            ax.add_patch(Rectangle((lcd_box[0], LCD_LOW), lcd_box[2] - lcd_box[0], MODULE_Z - LCD_LOW, fc="#ef9a9a"))
            ax.add_patch(Rectangle((lcd_box[0], MODULE_Z), lcd_box[2] - lcd_box[0], 5.6, fc="#e53935"))
            ax.text(45, MODULE_Z + 2.5, "LCD module", ha="center", color="white", fontsize=8)
        if amp_box[1] <= ycut <= amp_box[3]:
            ax.add_patch(Rectangle((amp_box[0], AMP_LOW), amp_box[2] - amp_box[0], MODULE_Z - AMP_LOW + 1.6, fc="#64b5f6"))
            ax.text((amp_box[0] + amp_box[2]) / 2, MODULE_Z + 2.2, "amp", ha="center", fontsize=8)
        if esp_box[1] <= ycut <= esp_box[3]:
            z = -PCB_T - SOCKET_H
            ax.add_patch(Rectangle((esp_box[0], z - HDR_PLASTIC), esp_box[2] - esp_box[0], HDR_PLASTIC, fc="#424242"))
            ax.add_patch(Rectangle((esp_box[0], z - HDR_PLASTIC - 1.6), esp_box[2] - esp_box[0], 1.6, fc="#1b1b1b"))
            ax.add_patch(Rectangle((esp_box[0], z - HDR_PLASTIC - 1.6 - 3.5), esp_box[2] - esp_box[0], 3.5, fc="#9e9e9e"))
            ax.text(60, z - HDR_PLASTIC - 4.5, "ESP32-S3-DevKitC-1 (components face down)", ha="center", fontsize=8)
        ax.axhline(LCD_LOW, color="red", lw=0.6, ls=":")
        ax.set_xlim(-2, 98)
        ax.set_ylim(-22, 20)
        ax.set_xlabel("x (mm)")
        ax.set_ylabel("z (mm)")
        ax.set_title(title)
    fig.tight_layout()
    fig.savefig(os.path.join(BB.BUILD, "fit_sections.png"), dpi=120)
    plt.close(fig)


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else BB.ROUTED))
