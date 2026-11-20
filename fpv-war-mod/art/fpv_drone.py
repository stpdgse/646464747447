#!/usr/bin/env python3
"""5-inch FPV quadcopter, hero version: carbon plates on standoffs, ESC stack, FPV camera with lens, strapped LiPo, anodised motors, tipped props, VTX mast.
Bone names (body, arm_N, prop_N) are kept so the existing spin animation and renderer work unchanged."""
import os, shutil, sys
sys.path.insert(0, os.path.dirname(__file__))
from lib.artmodel import ArtModel
from lib import paint as P

OUT = os.path.join(os.path.dirname(__file__), "out")
CARBON, CARBON2, METAL, RED, PCB, LIPO, STRAP, LENS = (26, 27, 31), (38, 39, 44), (150, 154, 162), (190, 38, 38), (28, 92, 52), (30, 32, 40), (226, 126, 24), (24, 40, 74)
m = ArtModel("fpv_drone", atlas=(128, 128))
m.bone("body", (0, 0, 0))
ARMS = {0: (1, 1, 45), 1: (-1, 1, -45), 2: (1, -1, 135), 3: (-1, -1, -135)}      # matches the original layout: arm_N reaches (sx*7, sz*7)
for i, (sx, sz, ang) in ARMS.items():
    m.bone(f"arm_{i}", (0, 1, 0), "body")
    m.bone(f"prop_{i}", (sx * 7, 4, sz * 7), f"arm_{i}")

def weave(face, px, py, w, h, ctx):                       # carbon-fibre twill
    base = P.solid(CARBON, grain=0.05, seed=2)(face, px, py, w, h, ctx)
    k = 1.22 if (px + py) % 4 < 2 else 0.9
    return (P.clamp(base[0] * k), P.clamp(base[1] * k), P.clamp(base[2] * k), 255)

def pcb(face, px, py, w, h, ctx):
    base = P.solid(PCB, grain=0.1, seed=4)(face, px, py, w, h, ctx)
    if face == "up" and ((px % 3 == 1 and py % 3 == 1) or (px == 0 and py % 2)): return (P.clamp(base[0] * 0.4), P.clamp(base[1] * 0.4), P.clamp(base[2] * 0.4), 255)  # chips
    if face == "up" and py == h // 2: return (196, 170, 70, 255)                                                                                       # gold trace
    return base

def lipo(face, px, py, w, h, ctx):
    base = P.solid(LIPO, grain=0.06, seed=6)(face, px, py, w, h, ctx)
    if face in ("up", "north", "south") and py % 2 == 1 and 1 <= px < w - 1 and h > 2: return (190, 190, 196, 255)      # printed label lines
    return base

def lens(face, px, py, w, h, ctx):
    base = P.solid(CARBON2, grain=0.04, seed=8)(face, px, py, w, h, ctx)
    if face == "north":
        cx, cy = (w - 1) / 2, (h - 1) / 2
        r = ((px - cx) ** 2 + (py - cy) ** 2) ** 0.5
        if r < 1.6: return (P.clamp(LENS[0] * (1.5 if (px, py) == (int(cx), int(cy)) else 1)), P.clamp(LENS[1] * 1.6), P.clamp(LENS[2] * 1.8), 255)   # glass
        if r < 2.4: return (14, 14, 16, 255)
    return base

def blade(face, px, py, w, h, ctx):                        # smoked prop with bright tips
    base = P.solid((52, 54, 60), grain=0.08, seed=10)(face, px, py, w, h, ctx)
    long_axis = px if w >= h else py; length = max(w, h)
    if face in ("up", "down") and (long_axis < 1 or long_axis >= length - 1): return (40, 170, 220, 255)
    return base

def part(*a, **k): m.part(*a, **k)
part("plate_bottom", "body", (-3, 0, -5), (3, 1, 5), weave)
part("plate_top", "body", (-3, 4, -5), (3, 5, 5), weave)
for k, (x, z) in enumerate([(-3, -5), (2, -5), (-3, 4), (2, 4)]): part(f"standoff_{k}", "body", (x, 1, z), (x + 1, 4, z + 1), P.metal(METAL, seed=12 + k))
part("esc_stack", "body", (-2, 1, -3), (2, 2, 3), pcb)
part("fc_board", "body", (-2, 2, -3), (2, 3, 3), pcb)
part("lipo", "body", (-2, 5, -4), (2, 8, 4), lipo)
part("strap", "body", (-3, 5, -1), (3, 8, 0), P.solid(STRAP, grain=0.04, seed=14))
part("cam_body", "body", (-2, 1, -7), (2, 5, -5), P.solid(CARBON2, grain=0.05, seed=16))
part("cam_lens", "body", (-1, 2, -8), (1, 4, -7), lens)
part("cam_cheek_l", "body", (-3, 1, -7), (-2, 4, -5), P.solid(RED, grain=0.05, seed=17))
part("cam_cheek_r", "body", (2, 1, -7), (3, 4, -5), P.solid(RED, grain=0.05, seed=18))
part("vtx_mast", "body", (-1, 5, 5), (0, 12, 6), P.metal(METAL, seed=19))
part("vtx_tip", "body", (-1, 12, 5), (0, 13, 6), P.solid(RED, grain=0.02, seed=20))
part("rx_pod", "body", (0, 5, 5), (2, 7, 7), P.solid(CARBON2, grain=0.05, seed=21))
for i, (sx, sz, ang) in ARMS.items():
    ax, az = sx * 7, sz * 7
    part(f"arm_bar_{i}", f"arm_{i}", (-1, 0, 0), (1, 1, 10), weave, rotation=(0, ang, 0), origin=(0, 1, 0))
    part(f"motor_{i}", f"arm_{i}", (ax - 2, 1, az - 2), (ax + 2, 3, az + 2), P.metal(METAL, seed=30 + i))
    part(f"bell_{i}", f"arm_{i}", (ax - 1, 3, az - 1), (ax + 1, 4, az + 1), P.solid(RED, grain=0.04, seed=40 + i))
    part(f"blade_a_{i}", f"prop_{i}", (ax - 5, 4, az - 1), (ax + 5, 5, az + 1), blade)
    part(f"blade_b_{i}", f"prop_{i}", (ax - 1, 4, az - 5), (ax + 1, 5, az + 5), blade, inflate=0.01)

if __name__ == "__main__":
    img = m.save(OUT)
    shutil.copy(os.path.join(os.path.dirname(__file__), "..", "src/main/resources/assets/kamikaze_skies/geckolib/animations/entity/fpv_drone.animation.json"), os.path.join(OUT, "fpv_drone.animation.json"))
    print(f"fpv_drone: {len(m.parts)} cubes, atlas used {m.used_height}/{m.atlas[1]}")
