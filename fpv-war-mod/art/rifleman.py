#!/usr/bin/env python3
"""Rifleman: a generic infantry soldier. Articulated (elbows, knees), helmet with goggles, plate carrier, rifle."""
import os, sys
sys.path.insert(0, os.path.dirname(__file__))
from lib.artmodel import ArtModel
from lib import paint as P

OUT = os.path.join(os.path.dirname(__file__), "out")

# generic olive uniform colours (no insignia anywhere)
OLIVE, DARK_OLIVE, TAN = (92, 102, 64), (62, 74, 46), (128, 116, 84)
VEST, POUCH, BELT = (58, 66, 48), (76, 82, 58), (52, 46, 38)
SKIN, GLOVE, BOOT = (205, 160, 128), (36, 36, 40), (42, 36, 32)
GUN, POLY = (62, 64, 70), (46, 48, 52)

m = ArtModel("rifleman", atlas=(128, 128))

# ---- bones (pivot = joint position; y up, front is -z, +x is the soldier's right) ------------------
m.bone("root", (0, 0, 0))
m.bone("body", (0, 12, 0), "root")
m.bone("torso", (0, 12, 0), "body")
m.bone("head", (0, 24, 0), "torso")
m.bone("arm_r", (6, 22, 0), "torso");   m.bone("arm_r_lower", (6, 16, 0), "arm_r")
m.bone("arm_l", (-6, 22, 0), "torso");  m.bone("arm_l_lower", (-6, 16, 0), "arm_l")
m.bone("rifle", (6, 9, 0), "arm_r_lower"); m.bone("mag", (6, 9, -2), "rifle")   # pivot at the grip = the right hand
m.bone("leg_r", (2, 12, 0), "body");    m.bone("leg_r_lower", (2, 6, 0), "leg_r")
m.bone("leg_l", (-2, 12, 0), "body");   m.bone("leg_l_lower", (-2, 6, 0), "leg_l")

camo = lambda seed, scale=3.0: P.weathering(P.camo(OLIVE, [DARK_OLIVE, TAN], scale=scale, seed=seed, threshold=(0.57, 0.68)), amount=0.3, seed=seed)
webbing = lambda col: P.with_decals(P.solid(col, grain=0.08, scale=1.3), {"north": P.rect_decal(
    [(x, y, x, y, P.shade(col, 0.7)) for x in range(0, 40, 3) for y in range(0, 12, 3)])})

# ---- torso, vest, gear ---------------------------------------------------------------------------
m.part("torso", "torso", (-4, 12, -2), (4, 24, 2), camo(1))
m.part("belt", "torso", (-4.5, 12, -2.5), (4.5, 14, 2.5), P.with_decals(P.solid(BELT, 0.08), {"north": P.rect_decal([(3, 0, 5, 1, (150, 150, 158)), (3, 0, 5, 0, (190, 190, 198))])}))
m.part("vest_front", "torso", (-4.5, 15, -3), (4.5, 23, -2), webbing(VEST))
m.part("vest_back", "torso", (-4.5, 15, 2), (4.5, 23, 3), P.solid(VEST, 0.08))
for i, (x0, x1) in enumerate(((-3.5, -1.5), (-1, 1), (1.5, 3.5))):
    m.part(f"pouch_{i}", "torso", (x0, 15, -4), (x1, 18, -3), P.with_decals(P.solid(POUCH, 0.07), {"north": P.rect_decal([(0, 0, 1, 0, P.shade(POUCH, 1.3))])}))
m.part("backpack", "torso", (-3.5, 13, 3), (3.5, 22, 6), camo(5, 2.6))
m.part("bedroll", "torso", (-3.5, 22, 3), (3.5, 24, 6), P.solid(TAN, 0.1))

# ---- head ----------------------------------------------------------------------------------------
face = P.with_decals(P.solid(SKIN, 0.05), {"north": P.rect_decal([
    (1, 3, 2, 3, (245, 245, 245)), (5, 3, 6, 3, (245, 245, 245)), (2, 3, 2, 3, (40, 56, 90)), (5, 3, 5, 3, (40, 56, 90)),   # eyes
    (1, 2, 2, 2, (80, 56, 40)), (5, 2, 6, 2, (80, 56, 40)),                                                                  # brows
    (3, 4, 4, 5, P.shade(SKIN, 0.88)), (3, 6, 4, 6, (150, 90, 80))]),                                                       # nose, mouth
    "up": P.rect_decal([(0, 0, 7, 7, (60, 44, 30))])})
m.part("head", "head", (-4, 24, -4), (4, 32, 4), face)
m.part("helmet", "head", (-4.5, 28, -4.5), (4.5, 33, 4.5), P.with_decals(P.camo(DARK_OLIVE, [OLIVE, (48, 56, 36)], scale=2.0, seed=9), {
    "north": P.rect_decal([(0, 4, 8, 4, (30, 34, 24))])}))
m.part("helmet_lip", "head", (-4.5, 27, -5), (4.5, 28, -4), P.solid(DARK_OLIVE, 0.06))
m.part("goggles", "head", (-3.5, 29, -5.5), (3.5, 31, -4.5), P.with_decals(P.solid((30, 30, 34), 0.05), {"north": P.rect_decal([
    (0, 0, 2, 1, (60, 120, 150)), (4, 0, 6, 1, (60, 120, 150)), (0, 0, 2, 0, (140, 190, 210)), (4, 0, 6, 0, (140, 190, 210))])}))
m.part("chin_strap_r", "head", (3.5, 24, -3), (4.5, 28, -2), P.solid((34, 38, 28), 0.05))
m.part("chin_strap_l", "head", (-4.5, 24, -3), (-3.5, 28, -2), P.solid((34, 38, 28), 0.05))

# ---- arms (upper + lower with glove) -------------------------------------------------------------
for side, sx in (("r", 1), ("l", -1)):
    x0, x1 = (4, 8) if sx > 0 else (-8, -4)
    m.part(f"arm_{side}_upper", f"arm_{side}", (x0, 16, -2), (x1, 22, 2), camo(20 + sx))
    m.part(f"shoulder_pad_{side}", f"arm_{side}", (min(x0, x1) - 0.5, 21, -2.5), (max(x0, x1) + 0.5, 23, 2.5), P.solid(VEST, 0.08))
    m.part(f"arm_{side}_lower", f"arm_{side}_lower", (x0, 10, -2), (x1, 16, 2), camo(30 + sx, 2.8))
    m.part(f"elbow_pad_{side}", f"arm_{side}_lower", (x0 + 0.5, 15, 2), (x1 - 0.5, 17, 3), P.solid(POUCH, 0.08))
    m.part(f"hand_{side}", f"arm_{side}_lower", (x0 + 0.5, 8, -1.5), (x1 - 0.5, 10, 1.5), P.solid(GLOVE, 0.07))

# ---- legs, boots ---------------------------------------------------------------------------------
for side, sx in (("r", 1), ("l", -1)):
    x0, x1 = (0, 4) if sx > 0 else (-4, 0)
    m.part(f"leg_{side}_upper", f"leg_{side}", (x0, 6, -2), (x1, 12, 2), camo(40 + sx, 3.2))
    m.part(f"thigh_pouch_{side}", f"leg_{side}", ((x1 if sx > 0 else x0 - 1), 7, -1.5), ((x1 + 1 if sx > 0 else x0), 11, 1.5), P.solid(POUCH, 0.08))
    m.part(f"leg_{side}_lower", f"leg_{side}_lower", (x0, 3, -2), (x1, 6, 2), camo(50 + sx, 3.0))
    m.part(f"knee_pad_{side}", f"leg_{side}_lower", (x0 + 0.5, 4, -3), (x1 - 0.5, 7, -2), P.solid(VEST, 0.08))
    m.part(f"boot_{side}", f"leg_{side}_lower", (x0, 0, -3), (x1, 3, 2), P.with_decals(P.solid(BOOT, 0.1), {"down": P.rect_decal([(0, 0, 3, 4, (24, 22, 20))])}))

# ---- rifle: authored around the right hand (grip point G = hand centre); local frame x right, y up, z back --------
G = (6, 9, 0)
def rpart(name, bone, lo, hi, shader):
    m.part(name, bone, (G[0] + lo[0], G[1] + lo[1], G[2] + lo[2]), (G[0] + hi[0], G[1] + hi[1], G[2] + hi[2]), shader)
rpart("rifle_grip", "rifle", (-1, -3, -1), (1, 1, 1), P.solid(POLY, 0.08))
rpart("rifle_body", "rifle", (-1, 1, -4), (1, 4, 2), P.metal(GUN, 0.1))
rpart("rifle_handguard", "rifle", (-1, 1.5, -8), (1, 3.5, -4), P.solid(POLY, 0.08))
rpart("rifle_barrel", "rifle", (-0.5, 2, -11), (0.5, 3, -8), P.metal((40, 42, 46), 0.08))
rpart("rifle_stock", "rifle", (-1, 1, 2), (1, 4, 4), P.solid(POLY, 0.08))
rpart("rifle_butt", "rifle", (-1, 0.5, 4), (1, 4.5, 5), P.solid((30, 30, 34), 0.06))
rpart("rifle_sight", "rifle", (-0.5, 4, -3), (0.5, 5, -1), P.metal(GUN, 0.08))
rpart("rifle_front_sight", "rifle", (-0.5, 3, -10), (0.5, 5, -9), P.metal(GUN, 0.08))
rpart("magazine", "mag", (-1, -4, -3), (1, 1, -1), P.solid((54, 54, 58), 0.08))

# rifle landmarks in its local frame (relative to the grip): where the left hand holds it, the shoulder pad, the muzzle
FOREGRIP = (0, 0.6, -2.5)      # where the left hand holds the rifle (rear of the handguard)
BUTT = (0, 2.5, 4.0)           # centre of the butt pad
MUZZLE = (0, 2.5, -11)
MAG_WELL = (0, 0, -2)

if __name__ == "__main__":
    img = m.save(OUT)
    print(f"rifleman: {len(m.parts)} cubes, atlas used height {m.used_height}/{m.atlas[1]}, wrote {OUT}")
