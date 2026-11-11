#!/usr/bin/env python3
"""Generate the project's own footprints (written with KiCad's API so the format is valid).

SW_Slide_SS12D00G3: the common 1P2T mini slide switch SS-12D00G3.
  Data used (LCSC C22355741 listing and hobby-shop listings, see spec section 15):
  3 pins in a row, 2.54 mm pitch; body 8 x 4 x 3.8 mm ("packaging 8.8 x 3.9 mm");
  rating 0.5 A 50 V DC. Pins are flat strips; a 1.0 mm drill fits them.

Run:  python3 gbc-handheld/gen_footprints.py
"""
import os

import pcbnew

HERE = os.path.dirname(os.path.abspath(__file__))
LIB = os.path.join(HERE, "gbc.pretty")
mm = pcbnew.FromMM


def rect(fp, layer, x0, y0, x1, y1, width):
    for (ax, ay), (bx, by) in (((x0, y0), (x1, y0)), ((x1, y0), (x1, y1)),
                               ((x1, y1), (x0, y1)), ((x0, y1), (x0, y0))):
        s = pcbnew.PCB_SHAPE(fp)
        s.SetShape(pcbnew.SHAPE_T_SEGMENT)
        s.SetLayer(layer)
        s.SetWidth(mm(width))
        s.SetStart(pcbnew.VECTOR2I(mm(ax), mm(ay)))
        s.SetEnd(pcbnew.VECTOR2I(mm(bx), mm(by)))
        fp.Add(s)


def ss12d00g3():
    fp = pcbnew.FOOTPRINT(None)
    fp.SetFPID(pcbnew.LIB_ID("gbc", "SW_Slide_SS12D00G3"))
    fp.SetLibDescription("Mini slide switch SS-12D00G3, 1P2T, 3 pins 2.54 mm, body 8.8 x 4.0 mm, 0.5 A 50 V DC")
    fp.SetKeywords("slide switch SPDT SS12D00 SS-12D00G3")
    fp.SetAttributes(pcbnew.FP_THROUGH_HOLE)
    for i in range(3):
        p = pcbnew.PAD(fp)
        p.SetNumber(str(i + 1))
        p.SetAttribute(pcbnew.PAD_ATTRIB_PTH)
        p.SetShape(pcbnew.PAD_SHAPE_RECTANGLE if i == 0 else pcbnew.PAD_SHAPE_CIRCLE)
        p.SetSize(pcbnew.VECTOR2I(mm(1.8), mm(1.8)))
        p.SetDrillSize(pcbnew.VECTOR2I(mm(1.0), mm(1.0)))
        p.SetLayerSet(pcbnew.PAD.PTHMask())
        p.SetPosition(pcbnew.VECTOR2I(mm(2.54 * i), 0))
        fp.Add(p)
    cx = 2.54
    rect(fp, pcbnew.F_Fab, cx - 4.4, -2.0, cx + 4.4, 2.0, 0.1)          # body 8.8 x 4.0
    rect(fp, pcbnew.F_SilkS, cx - 4.52, -2.12, cx + 4.52, 2.12, 0.12)   # silk just outside body
    rect(fp, pcbnew.F_CrtYd, cx - 4.75, -2.35, cx + 4.75, 2.35, 0.05)   # courtyard
    fp.Reference().SetText("REF**")
    fp.Reference().SetPosition(pcbnew.VECTOR2I(mm(cx), mm(-3.2)))
    fp.Reference().SetLayer(pcbnew.F_SilkS)
    fp.Value().SetText("SS-12D00G3")
    fp.Value().SetPosition(pcbnew.VECTOR2I(mm(cx), mm(3.2)))
    fp.Value().SetLayer(pcbnew.F_Fab)
    return fp


if __name__ == "__main__":
    os.makedirs(LIB, exist_ok=True)
    fp = ss12d00g3()
    # Ask for KiCad's native plugin explicitly; guessing from an empty .pretty folder fails.
    io = pcbnew.PCB_IO_MGR.PluginFind(pcbnew.PCB_IO_MGR.KICAD_SEXP)
    io.FootprintSave(LIB, fp)
    print("wrote", os.path.join(LIB, "SW_Slide_SS12D00G3.kicad_mod"))
