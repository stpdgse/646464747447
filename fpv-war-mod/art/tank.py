#!/usr/bin/env python3
"""Main battle tank (fictional, unmarked): hull, two tracks with 6 road wheels each, turret, recoiling gun, commander MG, radio mast."""
import math, os, sys
sys.path.insert(0, os.path.dirname(__file__))
from lib.artmodel import ArtModel
from lib import paint as P
from PIL import Image

OUT = os.path.join(os.path.dirname(__file__), "out")
OLIVE, BROWN, TAN = (84, 92, 58), (62, 54, 40), (122, 110, 78)
DARK, STEEL, GRILLE = (42, 42, 40), (58, 60, 62), (34, 34, 34)

m = ArtModel("tank", atlas=(256, 256))
m.bone("root"); m.bone("chassis", (0, 0, 0), "root")
m.bone("turret", (0, 15, 3), "chassis"); m.bone("gun", (0, 18, -11), "turret"); m.bone("barrel", (0, 18, -13), "gun")
m.bone("mg", (6, 27, 1), "turret"); m.bone("antenna", (-12.5, 15, 19.5), "chassis")
WHEEL_Z = [-21, -12, -3, 6, 15, 24]
for side, sx in (("l", -23), ("r", 23)):
    for i, z in enumerate(WHEEL_Z): m.bone(f"wheel_{side}{i}", (sx, 5, z), "chassis")

hull = lambda seed: P.weathering(P.camo(OLIVE, [BROWN, TAN], scale=4.5, seed=seed, threshold=(0.55, 0.66)), dirt=(78, 64, 48), amount=0.5, seed=seed + 4)

def tread(face, px, py, w, h, ctx):                      # track links: dark steel with a transverse rib every 4 pixels
    base = P.metal(DARK, grain=0.1, seed=7, brushed=False)(face, px, py, w, h, ctx)
    along = px if face in ("west", "east", "north", "south") and w > h else py
    if face in ("west", "east") and w > h: k = 0.78 if px % 4 == 0 else 1.0
    elif face == "up": k = 0.78 if py % 4 == 0 else 1.0
    else: k = 1.0
    return (P.clamp(base[0] * k), P.clamp(base[1] * k), P.clamp(base[2] * k), 255)

def wheel(face, px, py, w, h, ctx):                      # rubber-rimmed road wheel with a bolted hub
    base = P.metal(STEEL, grain=0.1, seed=5, brushed=False)(face, px, py, w, h, ctx)
    if face in ("west", "east"):
        cx, cy = (w - 1) / 2, (h - 1) / 2
        r = math.hypot(px - cx, py - cy)
        if r > 3.2: return (P.clamp(base[0] * 0.55), P.clamp(base[1] * 0.55), P.clamp(base[2] * 0.55), 255)       # tyre
        if r < 1.0: return (150, 150, 146, 255)                                                                  # hub cap
        ang = math.atan2(py - cy, px - cx)
        if 1.8 < r < 2.8 and abs(((ang / (math.pi / 2)) % 1.0) - 0.0) < 0.2: return (130, 130, 126, 255)       # bolts
    return base

def p(name, bone, frm, to, shader, **kw): m.part(name, bone, frm, to, shader, **kw)

# hull
p("hull", "chassis", (-16, 5, -26), (16, 13, 26), hull(1))
p("glacis", "chassis", (-16, 13, -26), (16, 15, -14), hull(2))
p("nose_plate", "chassis", (-14, 3, -28), (14, 8, -26), hull(3))
p("engine_deck", "chassis", (-14, 13, 14), (14, 15, 26), P.metal(GRILLE, grain=0.14, seed=9))
for side, x0 in (("l", -23), ("r", 15)):
    p(f"fender_{side}", "chassis", (x0, 10, -27), (x0 + 8, 11, 27), hull(5 if side == "l" else 6))
    p(f"track_{side}", "chassis", (-22, 0, -28) if side == "l" else (16, 0, -28), (-16, 10, 28) if side == "l" else (22, 10, 28), tread)
for i, z in enumerate(WHEEL_Z):
    p(f"wheel_l{i}", f"wheel_l{i}", (-24, 1, z - 4), (-22, 9, z + 4), wheel)
    p(f"wheel_r{i}", f"wheel_r{i}", (22, 1, z - 4), (24, 9, z + 4), wheel)
p("fuel_l", "chassis", (-12, 15, 17), (-8, 19, 25), P.camo((68, 76, 50), [BROWN], seed=21))
p("fuel_r", "chassis", (8, 15, 17), (12, 19, 25), P.camo((68, 76, 50), [BROWN], seed=22))
p("antenna", "antenna", (-13, 15, 19), (-12, 39, 20), P.solid((20, 20, 22)))
# turret
p("turret_body", "turret", (-11, 15, -9), (11, 22, 15), hull(11))
p("turret_cheek", "turret", (-9, 15, -13), (9, 21, -9), hull(12))
p("turret_bustle", "turret", (-9, 16, 15), (9, 21, 23), hull(13))
p("hatch_cmd", "turret", (3, 22, 2), (9, 24, 8), P.metal(STEEL, seed=14))
p("hatch_load", "turret", (-9, 22, 4), (-3, 23, 10), P.metal(STEEL, seed=15))
p("mg_mount", "turret", (4, 24, 3), (8, 26, 7), P.metal(DARK, seed=16))
p("mg_gun", "mg", (5, 26, -4), (7, 28, 6), P.metal(DARK, seed=17))
p("mantlet", "gun", (-4, 14, -15), (4, 22, -11), hull(18))
p("barrel", "barrel", (-2, 16, -43), (2, 20, -13), P.metal((46, 48, 50), grain=0.1, seed=19))
p("extractor", "barrel", (-3, 15, -30), (3, 21, -22), P.metal((52, 54, 56), seed=20))
p("brake", "barrel", (-3, 15, -47), (3, 21, -43), P.metal((38, 38, 40), seed=23))

def wreck(img):
    """Burnt-out texture: soot over everything, scorch patches, rust on the metal."""
    out = img.copy(); px = out.load()
    for y in range(out.height):
        for x in range(out.width):
            r, g, b, a = px[x, y]
            if a == 0: continue
            n = P.vnoise(x / 2.5, y / 2.5, 5); s = 0.50 + 0.30 * n
            burn = P.vnoise(x / 6.0, y / 6.0, 77)
            col = (r * s, g * s * 0.95, b * s * 0.9)
            if burn > 0.62: col = (col[0] * 0.5, col[1] * 0.5, col[2] * 0.5)
            if 0.40 < burn < 0.46: col = (110 * (0.6 + n * 0.4), 52 * (0.6 + n * 0.4), 28)       # rust
            px[x, y] = (P.clamp(col[0]), P.clamp(col[1]), P.clamp(col[2]), a)
    return out

if __name__ == "__main__":
    img = m.save(OUT)
    wreck(img).save(os.path.join(OUT, "tank_wreck.png"))
    print(f"tank: {len(m.parts)} cubes, atlas used height {m.used_height}/{m.atlas[1]}")
