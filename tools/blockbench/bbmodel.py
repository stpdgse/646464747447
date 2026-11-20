"""Minimal programmatic writer for Blockbench .bbmodel files (format 4.10, free-model, per-face UV)."""
import base64, io, json, uuid
from PIL import Image

FACES = ("north", "east", "south", "west", "up", "down")


def _uid():
    return str(uuid.uuid4())


class Model:
    def __init__(self, name, tex_size=16):
        self.name = name
        self.res = tex_size
        self.elements = []
        self.outliner = []
        self.textures = []
        self._groups = {}

    # -- texture -----------------------------------------------------------
    def add_texture(self, name, image: Image.Image):
        buf = io.BytesIO()
        image.save(buf, "PNG")
        data = "data:image/png;base64," + base64.b64encode(buf.getvalue()).decode()
        self.textures.append({
            "path": "", "name": name + ".png", "folder": "", "namespace": "",
            "id": str(len(self.textures)), "width": image.width, "height": image.height,
            "uv_width": image.width, "uv_height": image.height, "particle": False,
            "use_as_default": False, "layers_enabled": False, "sync_to_project": "",
            "render_mode": "default", "render_sides": "auto", "frame_time": 1,
            "frame_order_type": "loop", "frame_order": "", "frame_interpolate": False,
            "visible": True, "internal": True, "saved": False, "uuid": _uid(), "source": data,
        })
        return len(self.textures) - 1

    # -- geometry ----------------------------------------------------------
    def group(self, name, origin=(0, 0, 0), rotation=(0, 0, 0), parent=None):
        g = {"name": name, "origin": list(origin), "rotation": list(rotation), "color": 0,
             "uuid": _uid(), "export": True, "isOpen": True, "locked": False,
             "visibility": True, "autouv": 0, "children": []}
        (parent["children"] if parent else self.outliner).append(g)
        return g

    def cube(self, name, frm, to, uv_box=(0, 0, 4, 4), origin=None, rotation=(0, 0, 0),
             parent=None, tex=0, color=0, inflate=0):
        u0, v0, u1, v1 = uv_box
        e = {
            "name": name, "type": "cube", "box_uv": False, "rescale": False, "locked": False,
            "render_order": "default", "allow_mirror_modeling": True,
            "from": list(frm), "to": list(to), "autouv": 0, "color": color, "inflate": inflate,
            "origin": list(origin if origin is not None else [(a + b) / 2 for a, b in zip(frm, to)]),
            "rotation": list(rotation),
            "faces": {f: {"uv": [u0, v0, u1, v1], "texture": tex} for f in FACES},
            "uuid": _uid(),
        }
        self.elements.append(e)
        (parent["children"] if parent else self.outliner).append(e["uuid"])
        return e

    def box_cube(self, name, frm, to, uv_offset, parent=None, rotation=(0, 0, 0), origin=None, inflate=0, mirror_uv=False, tex=0, color=0):
        """A cube with Minecraft-style box UV (one unfolded strip at uv_offset); sizes should be whole numbers."""
        e = {
            "name": name, "type": "cube", "box_uv": True, "rescale": False, "locked": False,
            "render_order": "default", "allow_mirror_modeling": True,
            "from": list(frm), "to": list(to), "autouv": 0, "color": color, "inflate": inflate,
            "origin": list(origin if origin is not None else [(a + b) / 2 for a, b in zip(frm, to)]),
            "rotation": list(rotation), "uv_offset": list(uv_offset), "mirror_uv": mirror_uv,
            "faces": {f: {"uv": [0, 0, 0, 0], "texture": tex} for f in FACES},
            "uuid": _uid(),
        }
        self.elements.append(e)
        (parent["children"] if parent else self.outliner).append(e["uuid"])
        return e

    def to_json(self):
        return {
            "meta": {"format_version": "4.10", "model_format": "free", "box_uv": any(e.get("box_uv") for e in self.elements)},
            "name": self.name, "model_identifier": "", "visible_box": [1, 1, 0],
            "variable_placeholders": "", "variable_placeholder_buttons": [],
            "unhandled_marker_colors": [],
            "resolution": {"width": self.res, "height": self.res},
            "elements": self.elements, "outliner": self.outliner, "textures": self.textures,
        }

    def save(self, path):
        with open(path, "w") as f:
            json.dump(self.to_json(), f, indent=1)
