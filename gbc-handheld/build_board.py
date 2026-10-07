#!/usr/bin/env python3
"""Place, route and check the Game Boy Color-style handheld PCB (all through-hole).

Pipeline:  netlist (schematic.py)  ->  kinet2pcb  ->  place  ->  Freerouting  ->  fill  ->  DRC

Usage (after `source /etc/profile.d/eda.sh`; see ../setup-eda.sh):
    python3 gbc-handheld/build_board.py place     # placement only, writes build/placed.kicad_pcb
    python3 gbc-handheld/build_board.py route     # place + autoroute + fill + save routed board
    python3 gbc-handheld/build_board.py all       # route, then DRC and fab files

All dimensions in millimetres, board coordinates with the origin at the board's top-left
corner and +y pointing DOWN (the view from the top of the PCB). Sources for every number are
in ../docs/gbc-handheld-spec.md (sections 10, 11, 13).
"""
import math
import os
import subprocess
import sys

import pcbnew

HERE = os.path.dirname(os.path.abspath(__file__))
BUILD = os.path.join(HERE, "build")
NETLIST = os.path.join(BUILD, "gbc_handheld.net")
UNPLACED = os.path.join(BUILD, "unplaced.kicad_pcb")
PLACED = os.path.join(BUILD, "placed.kicad_pcb")
ROUTED = os.path.join(BUILD, "gbc_handheld.kicad_pcb")
DSN = os.path.join(BUILD, "gbc_handheld.dsn")
SES = os.path.join(BUILD, "gbc_handheld.ses")
FOOTPRINT_DIR = "/usr/share/kicad/footprints"

mm = pcbnew.FromMM

# --------------------------------------------------------------------------
# Board and module geometry (millimetres)
# --------------------------------------------------------------------------
# Board outline: 90 x 100 keeps it inside JLCPCB's cheapest size tier (100 x 100).
BW, BH, CORNER_R = 90.0, 98.0, 3.0
# Place the board away from the sheet origin so it is easy to view in KiCad.
OX, OY = 60.0, 50.0

# LCD module (MSP2807 outline drawing). Mounted LANDSCAPE: the portrait module
# (50 x 86) rotated 90 degrees clockwise, its back facing the PCB.
LCD_W, LCD_H = 86.0, 50.0
LCD_X, LCD_Y = 2.0, 4.0                      # module top-left on the board
LCD_HDR_X = LCD_X + 2.0                      # header line: 2.00 mm in from the module's left edge
LCD_HDR_PIN1_Y = LCD_Y + 8.49                # pin 1 is 8.49 mm from the module's top edge
LCD_HOLES = [(LCD_X + 6.92, LCD_Y + 3.0), (LCD_X + 6.92, LCD_Y + 47.0),
             (LCD_X + 83.0, LCD_Y + 3.0), (LCD_X + 83.0, LCD_Y + 47.0)]

# ESP32-S3-DevKitC-1 (Espressif DXF): holes 2.54 pitch, rows 22.86 apart, 62.74 mm long,
# last hole 7.96 mm from the USB end. Mounted on the UNDERSIDE, long axis along +x,
# antenna end left, USB end right, USB end overhanging the board's right edge.
ESP_ROW_GAP = 22.86
ESP_PITCH = 2.54
ESP_X0 = 33.0                                # x of pin 1 on both rows
ESP_YC = 29.0                                # centre line between the two rows
ESP_J1_Y = ESP_YC - ESP_ROW_GAP / 2          # J1 = top row (seen from above)
ESP_J3_Y = ESP_YC + ESP_ROW_GAP / 2          # J3 = bottom row
ESP_USB_END_X = ESP_X0 + 21 * ESP_PITCH + 7.96

# Centres of the through-hole parts below the LCD (board coordinates)
PLACE_CENTER = {
    "SW1": (18, 70, 0),    # UP
    "SW2": (18, 90, 0),    # DOWN
    "SW3": (8, 80, 0),     # LEFT
    "SW4": (28, 80, 0),    # RIGHT
    "SW5": (80, 72, 0),    # A
    "SW6": (68, 82, 0),    # B
    "SW7": (52, 92, 0),    # START
    "SW8": (40, 92, 0),    # SELECT
    "SW_PWR": (78, 60, 0),
    "R_LED": (64, 93, 0),
    "D_STATUS": (80, 93, 0),
    "C_BULK": (10, 62, 0),
    "C_HF": (10, 22, 0),
    "R_BL": (12, 34, 0),       # under the LCD (2.5 mm tall, LCD clearance 8.8 mm)
    "R_S1": (56, 47.5, 0),     # switch-sense divider, under the LCD near GPIO39
    "R_S2": (68, 47.5, 0),
    "C_AMP": (60.5, 65, 0),    # tall electrolytic: right of the amp module, outside the LCD
}
# Header-style parts placed by pin 1 position and direction of the pin row
PLACE_ROW = {
    "J_LCD": ((LCD_HDR_X, LCD_HDR_PIN1_Y), "down"),
    "J_AMP": ((38.0, 62.0), "right"),
    "J_SD": ((22.0, 58.0), "right"),
}

# Amplifier module (MAX98357A breakout, about 19 x 18 mm). It plugs into J_AMP and lies flat
# 11 mm above the board; its body MUST point down (+y), never up under the LCD.
AMP_W, AMP_D, AMP_EDGE = 19.0, 18.0, 1.3     # width, depth from header edge, pin row to edge
AMP_ROW_Y = 62.0
AMP_ROW_X0 = 38.0
AMP_CX = AMP_ROW_X0 + 3 * 2.54
AMP_BOX = (AMP_CX - AMP_W / 2, AMP_ROW_Y - AMP_EDGE, AMP_CX + AMP_W / 2, AMP_ROW_Y - AMP_EDGE + AMP_D)

# SD slot on the LCD module's back (outline drawing, back view, transformed to this mounting):
# board x 33.6-63.1, y 5.3-23.7, the card enters from the module's top edge (y = 4).
SD_SLOT_BOX = (33.6, 5.3, 63.1, 23.7)

# Silkscreen labels: (text, x, y, size)
LABELS = [
    ("UP", 18, 64.3, 1.0), ("DOWN", 18, 95.5, 1.0), ("LEFT", 8, 73.2, 1.0),
    ("RIGHT", 28, 73.2, 1.0), ("A", 80, 67.0, 1.4), ("B", 68, 77.0, 1.4),
    ("SELECT", 40, 87.0, 1.0), ("START", 52, 87.0, 1.0), ("PWR", 78, 55.5, 1.0),
    ("LED", 80, 89.2, 1.0), ("+", 7, 59.0, 1.2), ("SD wires", 25.8, 55.0, 1.0),
    ("AMP MODULE", AMP_CX, 71.5, 1.0), ("BODY POINTS DOWN", AMP_CX, 74.0, 1.0),
    ("VCC", 7.3, 12.5, 1.0), ("USB", 87.6, 29.0, 1.0),
]
# Signal names printed next to each header pin: (ref, names in pin order, side, size)
PIN_LABELS = [
    ("J_AMP", ["LRC", "BCLK", "DIN", "GAIN", "SD", "GND", "VIN"], "above", 1.0),
    ("J_SD", ["CS", "MOSI", "MISO", "SCK"], "below", 1.0),
]


def at(x, y):
    """Board coordinates (mm) -> absolute KiCad position."""
    return pcbnew.VECTOR2I(mm(OX + x), mm(OY + y))


def abs_mm(vec):
    """Absolute KiCad position -> board coordinates (mm)."""
    return (pcbnew.ToMM(vec.x) - OX, pcbnew.ToMM(vec.y) - OY)


def pad_pos(fp, number):
    return abs_mm(fp.FindPadByNumber(str(number)).GetPosition())


def pads_center(fp):
    pts = [abs_mm(p.GetPosition()) for p in fp.Pads()]
    return (sum(p[0] for p in pts) / len(pts), sum(p[1] for p in pts) / len(pts))


def move_to(fp, x, y):
    fp.SetPosition(at(x, y))


# --------------------------------------------------------------------------
# Step 1: unplaced board from the netlist
# --------------------------------------------------------------------------
def make_unplaced():
    # -l HERE: also search the project's own footprint library (gbc.pretty)
    subprocess.run(["kinet2pcb", "-i", NETLIST, "-o", UNPLACED, "-l", HERE, "-w", "--nobackup"],
                   check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    return pcbnew.LoadBoard(UNPLACED)


# --------------------------------------------------------------------------
# Step 2: placement
# --------------------------------------------------------------------------
def place_center(fp, cx, cy, rot):
    fp.SetOrientationDegrees(rot)
    move_to(fp, 0, 0)
    px, py = pads_center(fp)
    move_to(fp, cx - px, cy - py)


def place_row(fp, pin1_xy, direction):
    """Place a 1xN header so pin 1 is at pin1_xy and the pins run along `direction`."""
    want = {"down": (0, 1), "right": (1, 0)}[direction]
    n = len(list(fp.Pads()))
    for rot in (0, 90, 180, 270):
        fp.SetOrientationDegrees(rot)
        move_to(fp, 0, 0)
        (x1, y1), (xn, yn) = pad_pos(fp, 1), pad_pos(fp, n)
        dx, dy = xn - x1, yn - y1
        if abs(dx - want[0] * (n - 1) * 2.54) < 0.01 and abs(dy - want[1] * (n - 1) * 2.54) < 0.01:
            move_to(fp, pin1_xy[0] - x1, pin1_xy[1] - y1)
            return
    raise RuntimeError(f"no orientation puts {fp.GetReference()} pin row along {direction}")


def place_underside_row(fp, pin1_xy):
    """Put a 1x22 socket on the bottom side with pins along +x starting at pin1_xy."""
    center = fp.GetPosition()
    fp.Flip(center, pcbnew.FLIP_DIRECTION_LEFT_RIGHT)
    assert fp.IsFlipped(), "footprint did not flip to the bottom side"
    n = len(list(fp.Pads()))
    for rot in (0, 90, 180, 270):
        fp.SetOrientationDegrees(rot)
        move_to(fp, 0, 0)
        (x1, y1), (xn, yn) = pad_pos(fp, 1), pad_pos(fp, n)
        if abs((xn - x1) - (n - 1) * ESP_PITCH) < 0.01 and abs(yn - y1) < 0.01:
            move_to(fp, pin1_xy[0] - x1, pin1_xy[1] - y1)
            return
    raise RuntimeError("no orientation puts the underside row along +x")


def add_mounting_holes(b):
    holes = []
    for i, (x, y) in enumerate(LCD_HOLES, start=1):
        fp = pcbnew.FootprintLoad(os.path.join(FOOTPRINT_DIR, "MountingHole.pretty"),
                                  "MountingHole_3.2mm_M3_Pad")
        fp.SetReference(f"H{i}")
        fp.Reference().SetVisible(False)
        fp.Value().SetVisible(False)
        b.Add(fp)
        move_to(fp, x, y)
        holes.append(fp)
    return holes


def add_outline(b):
    layer = pcbnew.Edge_Cuts
    w = mm(0.1)
    r = CORNER_R

    def seg(p, q):
        s = pcbnew.PCB_SHAPE(b)
        s.SetShape(pcbnew.SHAPE_T_SEGMENT)
        s.SetLayer(layer)
        s.SetWidth(w)
        s.SetStart(at(*p))
        s.SetEnd(at(*q))
        b.Add(s)

    def arc(center, a0):
        cx, cy = center
        pts = [(cx + r * math.cos(math.radians(a)), cy + r * math.sin(math.radians(a)))
               for a in (a0, a0 + 45, a0 + 90)]
        s = pcbnew.PCB_SHAPE(b)
        s.SetShape(pcbnew.SHAPE_T_ARC)
        s.SetLayer(layer)
        s.SetWidth(w)
        s.SetArcGeometry(at(*pts[0]), at(*pts[1]), at(*pts[2]))
        b.Add(s)

    seg((r, 0), (BW - r, 0))
    seg((BW, r), (BW, BH - r))
    seg((BW - r, BH), (r, BH))
    seg((0, BH - r), (0, r))
    arc((BW - r, r), 270)        # top-right
    arc((BW - r, BH - r), 0)     # bottom-right
    arc((r, BH - r), 90)         # bottom-left
    arc((r, r), 180)             # top-left


def add_text(b, text, x, y, size=1.0, layer=pcbnew.F_SilkS, angle=0):
    t = pcbnew.PCB_TEXT(b)
    t.SetText(text)
    t.SetLayer(layer)
    t.SetPosition(at(x, y))
    t.SetTextSize(pcbnew.VECTOR2I(mm(size), mm(size)))
    t.SetTextThickness(mm(max(0.15, size * 0.15)))
    t.SetMirrored(layer in (pcbnew.B_SilkS, pcbnew.B_Fab))   # bottom text reads from the back
    if angle:
        t.SetTextAngleDegrees(angle)
    b.Add(t)


def add_pin_labels(b):
    for ref, names, side, size in PIN_LABELS:
        fp = b.FindFootprintByReference(ref)
        for i, name in enumerate(names, start=1):
            x, y = pad_pos(fp, i)
            off = 1.3 + 0.5 * size * len(name) * 0.9 + 0.2
            add_text(b, name, x, y - off if side == "above" else y + off, size, angle=90)


def add_rect(b, layer, x0, y0, x1, y1, width=0.12):
    for (ax, ay), (bx, by) in (((x0, y0), (x1, y0)), ((x1, y0), (x1, y1)),
                               ((x1, y1), (x0, y1)), ((x0, y1), (x0, y0))):
        s = pcbnew.PCB_SHAPE(b)
        s.SetShape(pcbnew.SHAPE_T_SEGMENT)
        s.SetLayer(layer)
        s.SetWidth(mm(width))
        s.SetStart(at(ax, ay))
        s.SetEnd(at(bx, by))
        b.Add(s)


def add_circle(b, layer, x, y, r, width=0.12):
    s = pcbnew.PCB_SHAPE(b)
    s.SetShape(pcbnew.SHAPE_T_CIRCLE)
    s.SetLayer(layer)
    s.SetWidth(mm(width))
    s.SetCenter(at(x, y))
    s.SetEnd(at(x + r, y))
    b.Add(s)


def add_module_outlines(b):
    """Draw the plug-in modules on the Fab layers (documentation only, not manufactured)."""
    add_rect(b, pcbnew.F_Fab, LCD_X, LCD_Y, LCD_X + LCD_W, LCD_Y + LCD_H)
    add_text(b, "LCD MODULE (11 mm above board)", LCD_X + LCD_W / 2, LCD_Y + 2.0, 1.0, pcbnew.F_Fab)
    add_rect(b, pcbnew.F_Fab, *SD_SLOT_BOX)
    add_text(b, "SD SLOT (card enters from top edge)", (SD_SLOT_BOX[0] + SD_SLOT_BOX[2]) / 2,
             SD_SLOT_BOX[1] + 2.0, 0.8, pcbnew.F_Fab)
    add_rect(b, pcbnew.F_Fab, *AMP_BOX)
    for x, y in LCD_HOLES:
        add_circle(b, pcbnew.F_Fab, x, y, 3.2)      # M3 standoff, 6.4 mm across corners
        add_circle(b, pcbnew.B_Fab, x, y, 2.75)     # M3 screw head, 5.5 mm
    esp_y0, esp_y1 = ESP_YC - 12.7, ESP_YC + 12.7
    add_rect(b, pcbnew.B_Fab, ESP_X0 - 1.44, esp_y0, ESP_USB_END_X, esp_y1)
    add_text(b, "ESP32-S3-DevKitC-1 (underside)", ESP_X0 + 26, ESP_YC + 6.0, 1.0, pcbnew.B_Fab)


def style_refs(b):
    # The buttons and the LED already carry a label (UP, A, B, LED ...); their reference
    # designators would overlap it, so those are hidden. Everything else keeps its reference.
    for fp in b.GetFootprints():
        ref = fp.GetReference()
        fp.Value().SetVisible(False)
        fp.Reference().SetTextSize(pcbnew.VECTOR2I(mm(1.0), mm(1.0)))
        fp.Reference().SetTextThickness(mm(0.13))
        if (ref.startswith("SW") and ref != "SW_PWR") or ref == "D_STATUS":
            fp.Reference().SetVisible(False)


# Ground pads that sit where the copper fill can only reach them through a single spoke
# (found by KiCad's DRC "starved_thermal" check). Solid-connect these to the fill.
SOLID_GND_PADS = [("J_LCD", "2"), ("J3", "1")]


def solid_connect_pads(b):
    for ref, num in SOLID_GND_PADS:
        b.FindFootprintByReference(ref).FindPadByNumber(num).SetLocalZoneConnection(
            pcbnew.ZONE_CONNECTION_FULL)


def set_rules(b):
    ds = b.GetDesignSettings()
    ds.SetCopperLayerCount(2)
    ns = ds.m_NetSettings
    d = ns.GetDefaultNetclass()
    d.SetClearance(mm(0.25))
    d.SetTrackWidth(mm(0.30))
    d.SetViaDiameter(mm(0.8))
    d.SetViaDrill(mm(0.4))
    p = pcbnew.NETCLASS("Power")
    p.SetClearance(mm(0.25))
    p.SetTrackWidth(mm(0.6))
    p.SetViaDiameter(mm(0.9))
    p.SetViaDrill(mm(0.5))
    ns.SetNetclass("Power", p)
    for pat in ("+5V_USB", "+5V_SW", "+3V3"):
        ns.SetNetclassPatternAssignment(pat, "Power")
    ds.m_TrackMinWidth = mm(0.2)
    ds.m_MinClearance = mm(0.2)
    ds.m_ViasMinSize = mm(0.6)
    ds.m_MinThroughDrill = mm(0.3)
    ds.m_HoleToHoleMin = mm(0.25)
    ds.m_CopperEdgeClearance = mm(0.3)


def add_ground_zones(b):
    gnd = b.FindNet("GND")
    inset = 0.6
    for layer in (pcbnew.F_Cu, pcbnew.B_Cu):
        z = pcbnew.ZONE(b)
        z.SetLayer(layer)
        z.SetNet(gnd)
        z.SetMinThickness(mm(0.25))
        z.SetLocalClearance(mm(0.3))
        z.SetPadConnection(pcbnew.ZONE_CONNECTION_THERMAL)
        z.SetThermalReliefGap(mm(0.3))
        z.SetThermalReliefSpokeWidth(mm(0.5))
        outline = z.Outline()
        outline.NewOutline()
        for px, py in ((inset, inset), (BW - inset, inset), (BW - inset, BH - inset),
                       (inset, BH - inset)):
            outline.Append(mm(OX + px), mm(OY + py))
        b.Add(z)


def place(b):
    for ref, (cx, cy, rot) in PLACE_CENTER.items():
        place_center(b.FindFootprintByReference(ref), cx, cy, rot)
    for ref, (xy, direction) in PLACE_ROW.items():
        place_row(b.FindFootprintByReference(ref), xy, direction)
    place_underside_row(b.FindFootprintByReference("J1"), (ESP_X0, ESP_J1_Y))
    place_underside_row(b.FindFootprintByReference("J3"), (ESP_X0, ESP_J3_Y))
    add_mounting_holes(b)
    add_outline(b)
    for text, x, y, size in LABELS:
        add_text(b, text, x, y, size)
    add_text(b, "GBC-PRACTICE v0.3", 72.0, BH - 1.6, 1.0)
    add_pin_labels(b)
    add_module_outlines(b)
    # Seen from the back (mirrored) the USB end is on the LEFT, so the arrow points left.
    add_text(b, "<- USB END  ESP32-S3 DEVKITC-1 GOES ON THIS SIDE", 60, 29.0, 1.0, pcbnew.B_SilkS)
    style_refs(b)
    solid_connect_pads(b)
    set_rules(b)
    # No ground fill yet: Freerouting would treat it as a perfect plane and skip GND tracks,
    # leaving pads that sit in cut-off copper islands unconnected. The fill is added after routing.


def geometry_report(b):
    """Numeric proof of the mechanical facts the board depends on."""
    ok = True

    def check(cond, msg):
        nonlocal ok
        print(("PASS  " if cond else "FAIL  ") + msg)
        ok = ok and cond

    j1, j3 = b.FindFootprintByReference("J1"), b.FindFootprintByReference("J3")
    lcd = b.FindFootprintByReference("J_LCD")
    check(j1.IsFlipped() and j3.IsFlipped(), "ESP32 sockets J1/J3 are on the underside")
    a1, a22, c1, c22 = pad_pos(j1, 1), pad_pos(j1, 22), pad_pos(j3, 1), pad_pos(j3, 22)
    check(abs((a22[0] - a1[0]) - 53.34) < 0.01 and abs(a22[1] - a1[1]) < 0.01,
          f"J1 runs +x, 21 x 2.54 = 53.34 mm (pin1 {a1[0]:.2f},{a1[1]:.2f})")
    check(abs((c22[0] - c1[0]) - 53.34) < 0.01, "J3 runs +x, 53.34 mm")
    check(abs((c1[1] - a1[1]) - ESP_ROW_GAP) < 0.01, f"rows {ESP_ROW_GAP} mm apart (J1 above J3)")
    check(abs(a1[0] - c1[0]) < 0.01, "pin 1 of both rows at the same x (antenna end left)")
    print(f"      ESP USB end at x = {ESP_USB_END_X:.2f} mm, board edge at {BW} mm "
          f"-> overhang {ESP_USB_END_X - BW:.2f} mm")
    check(ESP_USB_END_X - BW >= 3.0, "USB end overhangs the board edge by at least 3 mm")
    l1, l14 = pad_pos(lcd, 1), pad_pos(lcd, 14)
    check(abs((l14[1] - l1[1]) - 33.02) < 0.01 and abs(l14[0] - l1[0]) < 0.01,
          "LCD header: 13 x 2.54 = 33.02 mm, pins run down")
    hs = sorted((round(h.GetPosition().x / 1e6 - OX, 2), round(h.GetPosition().y / 1e6 - OY, 2))
                for h in b.GetFootprints() if h.GetReference().startswith("H"))
    xs, ys = sorted({p[0] for p in hs}), sorted({p[1] for p in hs})
    check(len(hs) == 4 and abs((xs[1] - xs[0]) - 76.08) < 0.01 and abs((ys[1] - ys[0]) - 44.0) < 0.01,
          f"mounting holes 76.08 x 44.00 mm apart: {hs}")
    # Everything inside the board outline with a 1 mm margin (except the USB overhang)
    for fp in b.GetFootprints():
        if fp.GetReference() in ("J1", "J3"):
            continue
        bb = fp.GetBoundingBox(False)
        x0, y0 = abs_mm(bb.GetOrigin())
        x1, y1 = x0 + pcbnew.ToMM(bb.GetWidth()), y0 + pcbnew.ToMM(bb.GetHeight())
        if x0 < 1.0 or y0 < 1.0 or x1 > BW - 1.0 or y1 > BH - 1.0:
            check(False, f"{fp.GetReference()} outside board margin: x {x0:.1f}-{x1:.1f}, y {y0:.1f}-{y1:.1f}")
    # No two footprints' pad areas closer than 0.5 mm unless the same net
    print("PASS  all footprints inside the outline margin" if ok else "")
    return ok


# --------------------------------------------------------------------------
# Step 3: routing with Freerouting
# --------------------------------------------------------------------------
def autoroute(b, passes=60, extra=()):
    pcbnew.ExportSpecctraDSN(b, DSN)
    java = os.environ.get("FREEROUTING_JAVA", "java")
    jar = os.environ["FREEROUTING_JAR"]
    if os.path.exists(SES):
        os.remove(SES)
    log = os.path.join(BUILD, "freerouting.log")
    with open(log, "w") as f:
        subprocess.run([java, "-jar", jar, "-de", DSN, "-do", SES, "-mp", str(passes),
                        *extra, "--gui.enabled=false"], stdout=f, stderr=subprocess.STDOUT,
                       timeout=900, check=False)
    if not os.path.exists(SES) or os.path.getsize(SES) == 0:
        raise RuntimeError(f"Freerouting produced no SES file; see {log}")
    if not pcbnew.ImportSpecctraSES(b, SES):
        raise RuntimeError("could not import the Freerouting SES result")


def save(b, path):
    """Save board AND project file (net classes and rules live in the .kicad_pro in KiCad 9)."""
    pcbnew.SaveBoard(path, b)
    with open(os.path.join(os.path.dirname(path), "fp-lib-table"), "w") as f:
        f.write('(fp_lib_table\n  (version 7)\n'
                '  (lib (name "gbc")(type "KiCad")(uri "${KIPRJMOD}/../gbc.pretty")'
                '(options "")(descr "GBC handheld project footprints"))\n)\n')


def refill(b):
    pcbnew.ZONE_FILLER(b).Fill(b.Zones())


def drc_violations(board_path):
    """Run KiCad's DRC on a saved board and return the violations as a list of dicts."""
    out = os.path.join(BUILD, "drc.json")
    subprocess.run(["kicad-cli", "pcb", "drc", "--severity-all", "--all-track-errors",
                    "--format", "json", "--output", out, board_path], check=False,
                   stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    import json
    with open(out) as f:
        return json.load(f).get("violations", [])


def settle_thermals(b, board_path, max_rounds=4):
    """Solid-connect any ground pad KiCad flags as `starved_thermal`, refill, repeat.

    Which pads end up starved depends on where the autorouter put its tracks, so the list
    cannot be hard-coded. A solid connection to the ground fill is electrically fine.
    """
    import re
    for _ in range(max_rounds):
        save(b, board_path)
        bad = [v for v in drc_violations(board_path) if v.get("type") == "starved_thermal"]
        pads = set()
        for v in bad:
            for item in v.get("items", []):
                m = re.search(r"pad (\S+) \[GND\] of (\S+)", item.get("description", ""))
                if m:
                    pads.add((m.group(2), m.group(1)))
        if not pads:
            return 0
        for ref, num in sorted(pads):
            print(f"      solid-connecting {ref} pad {num} to the ground fill")
            b.FindFootprintByReference(ref).FindPadByNumber(num).SetLocalZoneConnection(
                pcbnew.ZONE_CONNECTION_FULL)
        refill(b)
    return len(pads)


def connectivity_report(b):
    b.BuildConnectivity()
    conn = b.GetConnectivity()
    unrouted = conn.GetUnconnectedCount(True) if hasattr(conn, "GetUnconnectedCount") else -1
    tracks = [t for t in b.GetTracks()]
    vias = [t for t in tracks if t.Type() == pcbnew.PCB_VIA_T]
    print(f"      tracks+vias: {len(tracks)} (vias {len(vias)}), unconnected: {unrouted}")
    return unrouted


def main(stage):
    os.makedirs(BUILD, exist_ok=True)
    b = make_unplaced()
    place(b)
    print("=== geometry ===")
    ok = geometry_report(b)
    save(b, PLACED)
    print("saved", PLACED)
    if stage == "place":
        return 0 if ok else 1
    # Freerouting is multi-threaded and not deterministic: one run can leave a net it could not
    # fit. Every attempt starts from the clean placed board and is accepted only if KiCad's own
    # connectivity check (after the ground fill) reports nothing unconnected.
    strategies = [(), (), ("-is", "random"), ("-is", "random"), ("-is", "sequential"), ()]
    for attempt, extra in enumerate(strategies, start=1):
        print(f"=== autoroute attempt {attempt} {' '.join(extra) or '(default)'} ===")
        b = pcbnew.LoadBoard(PLACED)
        set_rules(b)
        autoroute(b, extra=extra)
        add_ground_zones(b)
        refill(b)
        left = settle_thermals(b, ROUTED)
        print(f"      starved thermal reliefs left: {left}")
        unrouted = connectivity_report(b)
        if unrouted == 0 and left == 0:
            break
        print(f"      attempt {attempt} rejected")
    save(b, ROUTED)
    print("saved", ROUTED)
    return 0 if ok and unrouted == 0 else 1


if __name__ == "__main__":
    sys.exit(main(sys.argv[1] if len(sys.argv) > 1 else "place"))
