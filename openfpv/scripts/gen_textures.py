#!/usr/bin/env python3
"""Generates OpenFPV's item icons and module-bay GUI texture (stdlib only, no PIL needed).
Run from the openfpv/ directory:  python3 scripts/gen_textures.py
"""
import os
import struct
import zlib

OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "src", "main", "resources", "assets", "openfpv", "textures")


def write_png(path, w, h, px):
    raw = bytearray()
    for y in range(h):
        raw.append(0)
        for x in range(w):
            raw.extend(px[y][x])

    def chunk(tag, data):
        c = struct.pack(">I", len(data)) + tag + data
        return c + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

    png = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
    png += chunk(b"IDAT", zlib.compress(bytes(raw), 9)) + chunk(b"IEND", b"")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as f:
        f.write(png)


class Img:
    def __init__(self, w, h, fill=(0, 0, 0, 0)):
        self.w, self.h = w, h
        self.px = [[fill for _ in range(w)] for _ in range(h)]

    def set(self, x, y, c):
        if 0 <= x < self.w and 0 <= y < self.h:
            self.px[y][x] = c

    def rect(self, x1, y1, x2, y2, c):
        for y in range(y1, y2 + 1):
            for x in range(x1, x2 + 1):
                self.set(x, y, c)

    def disc(self, cx, cy, r, c):
        for y in range(int(cy - r) - 1, int(cy + r) + 2):
            for x in range(int(cx - r) - 1, int(cx + r) + 2):
                if (x - cx) ** 2 + (y - cy) ** 2 <= r * r:
                    self.set(x, y, c)

    def ring(self, cx, cy, r, c, t=1.0):
        for y in range(int(cy - r) - 2, int(cy + r) + 3):
            for x in range(int(cx - r) - 2, int(cx + r) + 3):
                d = ((x - cx) ** 2 + (y - cy) ** 2) ** 0.5
                if r - t < d <= r:
                    self.set(x, y, c)

    def line(self, x1, y1, x2, y2, c):
        n = max(abs(x2 - x1), abs(y2 - y1), 1)
        for i in range(n + 1):
            self.set(round(x1 + (x2 - x1) * i / n), round(y1 + (y2 - y1) * i / n), c)

    def save(self, name):
        write_png(os.path.join(OUT, name), self.w, self.h, self.px)


def rgb(h, a=255):
    h = h.lstrip("#")
    return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16), a)


def chip(accent, dark="#23232b", mid="#3a3a46"):
    """Square module chip with gold pins on the left/right edges."""
    im = Img(16, 16)
    im.rect(3, 2, 12, 13, rgb(dark))
    im.rect(4, 3, 11, 12, rgb(mid))
    for y in (4, 6, 8, 10):
        im.set(2, y, rgb("#d4af37"))
        im.set(13, y, rgb("#d4af37"))
    im.rect(3, 2, 12, 2, rgb(accent))  # accent strip on top
    return im


def camera():
    im = Img(16, 16)
    im.rect(1, 2, 14, 13, rgb("#1f3d2a"))
    im.rect(2, 3, 13, 12, rgb("#2f6b3a"))
    for x in (3, 5, 7, 9, 11):  # traces
        im.set(x, 12, rgb("#9ccc65"))
    for x in range(3, 13, 2):  # gold pins along the bottom
        im.set(x, 14, rgb("#d4af37"))
        im.set(x, 15, rgb("#a8872a"))
    im.disc(7.5, 7, 4.2, rgb("#14141a"))
    im.disc(7.5, 7, 3.2, rgb("#1e4fa8"))
    im.disc(7.5, 7, 2.2, rgb("#3d86e0"))
    im.disc(7.5, 7, 1.0, rgb("#0a1a3a"))
    im.set(6, 5, rgb("#d8ecff"))
    im.set(5, 6, rgb("#a8d0ff"))
    im.set(12, 4, rgb("#ff3b30"))  # status LED
    return im


def night_vision():
    im = chip("#33ff66")
    # eye
    for x, ys in ((5, (7, 8)), (6, (6, 7, 8, 9)), (7, (5, 6, 9, 10)), (8, (5, 6, 9, 10)), (9, (6, 7, 8, 9)), (10, (7, 8))):
        for y in ys:
            im.set(x, y, rgb("#c8ffd4"))
    for y in (7, 8):
        for x in (7, 8):
            im.set(x, y, rgb("#06210d"))
    return im


def zoom():
    im = chip("#4fc3f7")
    im.ring(7, 7, 3.4, rgb("#e3f4ff"), 1.3)
    im.disc(7, 7, 2.0, rgb("#2b6a8f"))
    im.line(9, 9, 11, 11, rgb("#e3f4ff"))
    im.line(10, 9, 11, 10, rgb("#e3f4ff"))
    im.set(6, 6, rgb("#bfe9ff"))
    return im


def gimbal():
    im = chip("#ffb74d")
    im.ring(7.5, 7.5, 4.0, rgb("#ffe0b2"), 1.0)
    im.ring(7.5, 7.5, 2.2, rgb("#ffa726"), 1.0)
    im.set(7, 7, rgb("#fff3e0"))
    im.set(8, 8, rgb("#fff3e0"))
    for p in ((7, 3), (8, 3), (7, 12), (8, 12)):
        im.set(*p, rgb("#ffe0b2"))
    return im


def thermal():
    im = chip("#ff5722")
    # flame
    flame = {
        4: (7, 8), 5: (6, 7, 8, 9), 6: (6, 7, 8, 9), 7: (5, 6, 7, 8, 9, 10),
        8: (5, 6, 7, 8, 9, 10), 9: (5, 6, 7, 8, 9, 10), 10: (6, 7, 8, 9),
    }
    for y, xs in flame.items():
        for x in xs:
            t = (y - 4) / 6.0
            col = (255, int(235 - 150 * t), int(60 - 40 * t), 255)
            im.set(x, y, col)
    im.set(7, 9, rgb("#fff59d"))
    im.set(8, 9, rgb("#fff59d"))
    im.set(7, 10, rgb("#fff59d"))
    return im


def depth():
    im = chip("#7e57c2")
    # perspective grid
    c = rgb("#b39ddb")
    im.rect(5, 5, 10, 10, rgb("#1b1238"))
    im.rect(5, 5, 10, 5, c)
    im.rect(5, 10, 10, 10, c)
    im.rect(5, 5, 5, 10, c)
    im.rect(10, 5, 10, 10, c)
    im.line(5, 5, 7, 7, c)
    im.line(10, 5, 8, 7, c)
    im.line(5, 10, 7, 8, c)
    im.line(10, 10, 8, 8, c)
    im.rect(7, 7, 8, 8, rgb("#ede7f6"))
    return im


def gui():
    W, H = 256, 256
    im = Img(W, H)
    PW, PH = 176, 133
    im.rect(0, 0, PW - 1, PH - 1, rgb("#c6c6c6"))
    im.rect(0, 0, PW - 1, 0, rgb("#ffffff"))
    im.rect(0, 0, 0, PH - 1, rgb("#ffffff"))
    im.rect(0, PH - 1, PW - 1, PH - 1, rgb("#555555"))
    im.rect(PW - 1, 0, PW - 1, PH - 1, rgb("#555555"))
    im.set(0, PH - 1, rgb("#c6c6c6"))
    im.set(PW - 1, 0, rgb("#c6c6c6"))

    def slot(x, y):
        # (x,y) is the item origin; the frame is one pixel bigger on each side.
        im.rect(x - 1, y - 1, x + 16, y + 16, rgb("#8b8b8b"))
        im.rect(x - 1, y - 1, x + 16, y - 1, rgb("#373737"))
        im.rect(x - 1, y - 1, x - 1, y + 16, rgb("#373737"))
        im.rect(x - 1, y + 16, x + 16, y + 16, rgb("#ffffff"))
        im.rect(x + 16, y - 1, x + 16, y + 16, rgb("#ffffff"))
        im.set(x - 1, y + 16, rgb("#8b8b8b"))
        im.set(x + 16, y - 1, rgb("#8b8b8b"))

    for i in range(5):
        slot(43 + i * 18, 20)
    for row in range(3):
        for col in range(9):
            slot(8 + col * 18, 51 + row * 18)
    for col in range(9):
        slot(8 + col * 18, 109)
    # thin divider under the bay
    im.rect(8, 40 - 3, PW - 9, 40 - 3, rgb("#a0a0a0"))
    return im


if __name__ == "__main__":
    camera().save("items/camera_upgrade.png")
    night_vision().save("items/night_vision_module.png")
    zoom().save("items/zoom_module.png")
    gimbal().save("items/gimbal_module.png")
    thermal().save("items/thermal_module.png")
    depth().save("items/depth_module.png")
    gui().save("gui/module_bay.png")
    print("textures written to", os.path.normpath(OUT))
