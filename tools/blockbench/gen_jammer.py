#!/usr/bin/env python3
"""Generate the anti-drone jammer mast as a .bbmodel (about 1.6 blocks tall, 16 units = 1 block)."""
import sys
from PIL import Image
from bbmodel import Model

PAL = {"carbon": (28, 28, 32), "red": (200, 40, 40), "metal": (150, 155, 165), "prop": (60, 200, 255)}
img = Image.new("RGBA", (16, 16))
for (cx, cy), c in zip(((0, 0), (1, 0), (0, 1), (1, 1)), PAL.values()):
    for x in range(4):
        for y in range(4):
            img.putpixel((cx * 4 + x, cy * 4 + y), c + (255,))
UV = {"carbon": (0, 0, 4, 4), "red": (4, 0, 8, 4), "metal": (0, 4, 4, 8), "cyan": (4, 4, 8, 8)}

m = Model("jammer", 16)
m.add_texture("palette", img)
base = m.group("base", origin=(0, 0, 0))
m.cube("plate", (-5, 0, -5), (5, 2, 5), UV["carbon"], parent=base)
m.cube("mast", (-1, 2, -1), (1, 18, 1), UV["metal"], parent=base)
m.cube("bar_x", (-7, 14, -0.5), (7, 15, 0.5), UV["carbon"], parent=base)
m.cube("bar_z", (-0.5, 14, -7), (0.5, 15, 7), UV["carbon"], parent=base)
for i, (x, z) in enumerate(((6, 0), (-6, 0), (0, 6), (0, -6))):
    m.cube(f"tip_{i}", (x - 1, 14, z - 1), (x + 1, 16, z + 1), UV["cyan"], parent=base)
emitter = m.group("emitter", origin=(0, 18, 0), parent=base)
m.cube("emitter_box", (-2, 18, -2), (2, 22, 2), UV["red"], parent=emitter)
m.cube("spike", (-0.3, 22, -0.3), (0.3, 27, 0.3), UV["metal"], parent=emitter)
for i, (x, z) in enumerate(((3, 3), (-3, 3), (3, -3), (-3, -3))):
    m.cube(f"foot_{i}", (x - 1, 0, z - 1), (x + 1, 3, z + 1), UV["carbon"], parent=base)

out = sys.argv[1] if len(sys.argv) > 1 else "out/jammer.bbmodel"
m.save(out)
print("wrote", out, f"({len(m.elements)} cubes)")
