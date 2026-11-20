#!/usr/bin/env python3
"""Generate a 5-inch FPV quadcopter as a .bbmodel (test of the headless Blockbench pipeline)."""
import sys
from PIL import Image
from bbmodel import Model

# 16x16 palette texture: each 4x4 quadrant is one flat colour
PAL = {"carbon": (28, 28, 32), "red": (200, 40, 40), "metal": (150, 155, 165), "prop": (60, 200, 255)}
img = Image.new("RGBA", (16, 16))
for (cx, cy), c in zip(((0, 0), (1, 0), (0, 1), (1, 1)), PAL.values()):
    for x in range(4):
        for y in range(4):
            img.putpixel((cx * 4 + x, cy * 4 + y), c + (255,))

m = Model("fpv_quad", 16)
m.add_texture("palette", img)
UV = {"carbon": (0, 0, 4, 4), "red": (4, 0, 8, 4), "metal": (0, 4, 4, 8), "prop": (4, 4, 8, 8)}

body = m.group("body", origin=(0, 0, 0))
m.cube("stack", (-2, 0, -3), (2, 2, 3), UV["carbon"], parent=body)
m.cube("battery", (-1.5, 2, -3.5), (1.5, 3.5, 3.5), UV["red"], parent=body)
m.cube("camera", (-1, 0.5, -5), (1, 2.5, -3), UV["metal"], parent=body)
m.cube("antenna", (-0.2, 3.5, 3), (0.2, 6, 3.4), UV["metal"], parent=body)

for i, (sx, sz) in enumerate(((1, 1), (-1, 1), (1, -1), (-1, -1))):
    arm = m.group(f"arm_{i}", origin=(0, 1, 0), parent=body)
    ax, az = 7 * sx, 7 * sz
    ang = {(1, 1): 45, (-1, 1): -45, (1, -1): 135, (-1, -1): -135}[(sx, sz)]
    m.cube(f"arm_bar_{i}", (-0.5, 0.5, 0), (0.5, 1.5, 9.9), UV["carbon"],
           origin=(0, 1, 0), rotation=(0, ang, 0), parent=arm)
    m.cube(f"motor_{i}", (ax - 1, 1.5, az - 1), (ax + 1, 3, az + 1), UV["metal"], parent=arm)
    prop = m.group(f"prop_{i}", origin=(ax, 3, az), parent=arm)   # spin pivot for animation
    m.cube(f"prop_a_{i}", (ax - 4, 3, az - 0.4), (ax + 4, 3.2, az + 0.4), UV["prop"], parent=prop)
    m.cube(f"prop_b_{i}", (ax - 0.4, 3, az - 4), (ax + 0.4, 3.2, az + 4), UV["prop"], parent=prop)

out = sys.argv[1] if len(sys.argv) > 1 else "out/fpv_quad.bbmodel"
m.save(out)
print("wrote", out, f"({len(m.elements)} cubes)")
