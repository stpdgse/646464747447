"""Author a box-UV Blockbench model with an auto-packed, procedurally painted texture.

Usage: build bones and parts, then save the .bbmodel (the editable source of truth) and the PNG.
Face decals should only be put on 'north' (front), 'south' (back) and 'up': side-face orientation depends on the engine.
"""
import os, sys, json
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "..", "..", "tools", "blockbench"))
from bbmodel import Model
from PIL import Image


class Bone:
    def __init__(self, name, pivot, parent, rotation):
        self.name, self.pivot, self.parent, self.rotation = name, pivot, parent, rotation


class Part:
    def __init__(self, name, bone, frm, to, shader, inflate=0, rotation=(0, 0, 0), origin=None, tint=None):
        self.name, self.bone, self.frm, self.to, self.shader = name, bone, frm, to, shader
        self.inflate, self.rotation, self.origin = inflate, rotation, origin
        self.w, self.h, self.d = (round(abs(to[0] - frm[0])), round(abs(to[1] - frm[1])), round(abs(to[2] - frm[2])))
        for got, want in zip((self.w, self.h, self.d), (abs(to[0] - frm[0]), abs(to[1] - frm[1]), abs(to[2] - frm[2]))):
            assert abs(got - want) < 1e-6, f"{name}: cube sizes must be whole numbers (box UV), got {want}"
        self.uv = None


class ArtModel:
    def __init__(self, name, atlas=(128, 128)):
        self.name, self.atlas = name, atlas
        self.bones, self.parts = {}, []

    def bone(self, name, pivot=(0, 0, 0), parent=None, rotation=(0, 0, 0)):
        b = Bone(name, pivot, self.bones[parent] if isinstance(parent, str) else parent, rotation)
        self.bones[name] = b
        return b

    def part(self, name, bone, frm, to, shader, **kw):
        bone = self.bones[bone] if isinstance(bone, str) else bone
        p = Part(name, bone, frm, to, shader, **kw)
        self.parts.append(p)
        return p

    # ---- layout ---------------------------------------------------------------------------------
    def layout(self):
        W, H = self.atlas
        x = y = shelf = 0
        for p in sorted(self.parts, key=lambda p: -(p.d + p.h)):
            fw, fh = 2 * (p.d + p.w), p.d + p.h
            if x + fw > W: x, y, shelf = 0, y + shelf, 0
            assert y + fh <= H, f"texture atlas {self.atlas} is full at part {p.name}; enlarge it"
            p.uv = (x, y)
            x += fw
            shelf = max(shelf, fh)
        self.used_height = y + shelf

    def regions(self, p):
        u, v = p.uv
        w, h, d = p.w, p.h, p.d
        return {"up": (u + d, v, w, d), "down": (u + d + w, v, w, d), "west": (u, v + d, d, h), "north": (u + d, v + d, w, h),
                "east": (u + d + w, v + d, d, h), "south": (u + 2 * d + w, v + d, w, h)}

    # ---- painting -------------------------------------------------------------------------------
    def paint(self):
        self.layout()
        img = Image.new("RGBA", self.atlas, (0, 0, 0, 0))
        px = img.load()
        for p in self.parts:
            for face, (x0, y0, fw, fh) in self.regions(p).items():
                ctx = {"ox": x0, "oy": y0}
                for j in range(fh):
                    for i in range(fw):
                        px[x0 + i, y0 + j] = p.shader(face, i, j, fw, fh, ctx)
        return img

    # ---- model ----------------------------------------------------------------------------------
    def to_bbmodel(self, img):
        m = Model(self.name, self.atlas[0])
        m.res = self.atlas[0]
        m.add_texture(self.name, img)
        groups = {}
        def make(b):
            if b.name in groups: return groups[b.name]
            parent = make(b.parent) if b.parent else None
            groups[b.name] = m.group(b.name, origin=b.pivot, rotation=b.rotation, parent=parent)
            return groups[b.name]
        for b in self.bones.values(): make(b)
        for p in self.parts:
            m.box_cube(p.name, p.frm, p.to, p.uv, parent=groups[p.bone.name], rotation=p.rotation, origin=p.origin, inflate=p.inflate)
        data = m.to_json()
        data["resolution"] = {"width": self.atlas[0], "height": self.atlas[1]}
        return data

    def save(self, outdir):
        os.makedirs(outdir, exist_ok=True)
        img = self.paint()
        img.save(os.path.join(outdir, f"{self.name}.png"))
        with open(os.path.join(outdir, f"{self.name}.bbmodel"), "w") as f:
            json.dump(self.to_bbmodel(img), f, indent=1)
        return img
