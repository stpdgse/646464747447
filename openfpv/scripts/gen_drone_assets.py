#!/usr/bin/env python3
"""Generates the quadcopter Blockbench model (.bbmodel), its 64x64 texture, and the sounds.

  python3 scripts/gen_drone_assets.py

The .bbmodel opens in Blockbench ("Free Model" format, box UV). The renderer in
com.openfpv.client.BbModel loads the very same file at runtime, so editing it in Blockbench and saving it
over src/main/resources/assets/openfpv/models/entity/quadcopter.bbmodel changes the in-game drone.
Group names drive the animation: prop_fl/fr/bl/br spin, camera_pod follows the gimbal, and elements named
light_red / light_green / light_white / light_status are drawn fullbright and blink.
"""
import base64, json, math, os, random, struct, subprocess, uuid, wave, zlib

ROOT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "src", "main", "resources", "assets", "openfpv")
random.seed(7)

# ---------------------------------------------------------------- model description
# material -> base colour; 'light' stays white so the renderer can tint it
MATERIALS = {
    "carbon": (38, 40, 48), "carbon2": (52, 55, 66), "motor": (150, 156, 168), "prop": (22, 22, 26),
    "pod": (60, 64, 74), "lens": (30, 90, 200), "skid": (28, 28, 32), "light": (255, 255, 255),
    "battery": (210, 160, 40), "accent": (230, 60, 50),
}
elements = []   # dicts: name, from, to, origin, rotation, material, group
groups = {}     # name -> (origin, [element indices])

def cube(name, frm, to, mat, group, rot=(0, 0, 0), origin=None):
    if origin is None:
        origin = [(frm[i] + to[i]) / 2 for i in range(3)]
    elements.append(dict(name=name, frm=list(frm), to=list(to), mat=mat, rot=list(rot), origin=list(origin), group=group))
    groups.setdefault(group, ([0, 0, 0], []))[1].append(len(elements) - 1)

def group(name, origin):
    groups.setdefault(name, ([0, 0, 0], []))
    groups[name] = (list(origin), groups[name][1])

d = 5.7  # motor offset from centre (px)
group("body", [0, 0, 0]); group("arms", [0, 0, 0]); group("skids", [0, 0, 0])
cube("frame", [-3, 2, -4], [3, 5, 4], "carbon", "body")
cube("plate_top", [-2.5, 5, -3.5], [2.5, 5.6, 3.5], "carbon2", "body")
cube("battery", [-2, 5.6, -3], [2, 7.4, 3], "battery", "body")
cube("battery_stripe", [-2.05, 6.2, -1], [2.05, 6.8, 1], "accent", "body")
cube("arm_a", [-8.4, 3, -0.8], [8.4, 4.5, 0.8], "carbon2", "arms", rot=(0, 45, 0), origin=[0, 3.75, 0])
cube("arm_b", [-8.4, 3, -0.8], [8.4, 4.5, 0.8], "carbon2", "arms", rot=(0, -45, 0), origin=[0, 3.75, 0])
cube("skid_l", [-4.2, 0, -5], [-3.2, 0.9, 5], "skid", "skids")
cube("skid_r", [3.2, 0, -5], [4.2, 0.9, 5], "skid", "skids")
cube("leg_fl", [-4.2, 0.9, -3.8], [-3.2, 2, -2.8], "skid", "skids")
cube("leg_fr", [3.2, 0.9, -3.8], [4.2, 2, -2.8], "skid", "skids")
cube("leg_bl", [-4.2, 0.9, 2.8], [-3.2, 2, 3.8], "skid", "skids")
cube("leg_br", [3.2, 0.9, 2.8], [4.2, 2, 3.8], "skid", "skids")
corners = {"fl": (-d, -d), "fr": (d, -d), "bl": (-d, d), "br": (d, d)}
for key, (x, z) in corners.items():
    cube("motor_" + key, [x - 1.3, 3.2, z - 1.3], [x + 1.3, 5.6, z + 1.3], "motor", "arms")
    group("prop_" + key, [x, 5.6, z])
    cube("blade_a_" + key, [x - 4.3, 5.6, z - 0.55], [x + 4.3, 5.9, z + 0.55], "prop", "prop_" + key)
    cube("blade_b_" + key, [x - 4.3, 5.6, z - 0.55], [x + 4.3, 5.9, z + 0.55], "prop", "prop_" + key, rot=(0, 90, 0), origin=[x, 5.6, z])
    cube("hub_" + key, [x - 0.6, 5.6, z - 0.6], [x + 0.6, 6.2, z + 0.6], "motor", "prop_" + key)
group("camera_pod", [0, 3.4, -4])
cube("pod", [-1.8, 1.9, -6.6], [1.8, 4.9, -3.8], "pod", "camera_pod")
cube("lens", [-1, 2.7, -7.2], [1, 4.1, -6.6], "lens", "camera_pod")
group("lights", [0, 0, 0])
cube("light_red", [-d - 0.8, 2.6, -d - 0.8], [-d + 0.8, 3.2, -d + 0.8], "light", "lights")
cube("light_green", [d - 0.8, 2.6, -d - 0.8], [d + 0.8, 3.2, -d + 0.8], "light", "lights")
cube("light_white", [-0.8, 7.4, -0.8], [0.8, 8.0, 0.8], "light", "lights")
cube("light_status", [-1.2, 3.0, 4.0], [1.2, 4.2, 4.6], "light", "lights")

# ---------------------------------------------------------------- UV packing (box UV, 64x64)
def size(e):
    return [e["to"][i] - e["frm"][i] for i in range(3)]
rects = []
for i, e in enumerate(elements):
    w, h, dd = [max(1, math.ceil(v)) for v in size(e)]
    rects.append((i, 2 * dd + 2 * w, dd + h))
x = y = rowh = 0
placed = {}
for i, rw, rh in sorted(rects, key=lambda r: -r[2]):
    if x + rw > 64:
        x, y, rowh = 0, y + rowh, 0
    placed[i] = (x, y)
    x += rw
    rowh = max(rowh, rh)
assert y + rowh <= 64, "texture overflow: %d" % (y + rowh)

W = H = 64
px = [[(0, 0, 0, 0)] * W for _ in range(H)]
for i, e in enumerate(elements):
    w, h, dd = [max(1, math.ceil(v)) for v in size(e)]
    u, v = placed[i]
    base = MATERIALS[e["mat"]]
    for yy in range(v, v + dd + h):
        for xx in range(u, u + 2 * dd + 2 * w):
            if xx >= W or yy >= H:
                continue
            n = random.randint(-7, 7) if e["mat"] != "light" else 0
            top = 1.18 if yy < v + dd else 1.0
            c = tuple(max(0, min(255, int(base[k] * top + n))) for k in range(3))
            if e["mat"] == "carbon" and (xx + yy) % 4 == 0:     # carbon weave
                c = tuple(min(255, k + 10) for k in c)
            px[yy][xx] = c + (255,)
    e["uv"] = (u, v)

def png(path, w, h, pix):
    raw = b"".join(b"\x00" + b"".join(bytes(p) for p in row) for row in pix)
    def ch(t, d):
        c = struct.pack(">I", len(d)) + t + d
        return c + struct.pack(">I", zlib.crc32(t + d) & 0xFFFFFFFF)
    data = b"\x89PNG\r\n\x1a\n" + ch(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0)) + ch(b"IDAT", zlib.compress(raw, 9)) + ch(b"IEND", b"")
    os.makedirs(os.path.dirname(path), exist_ok=True)
    open(path, "wb").write(data)
    return data

tex = png(os.path.join(ROOT, "textures", "entity", "quadcopter.png"), W, H, px)

# ---------------------------------------------------------------- .bbmodel
def uid():
    return str(uuid.uuid4())
for e in elements:
    e["uuid"] = uid()
bb_elements = []
for e in elements:
    bb_elements.append({
        "name": e["name"], "box_uv": True, "rescale": False, "locked": False, "light_emission": 0,
        "render_order": "default", "allow_mirror_modeling": True, "from": e["frm"], "to": e["to"], "autouv": 0,
        "color": 0, "origin": e["origin"], "rotation": e["rot"], "uv_offset": list(e["uv"]),
        "faces": {f: {"uv": [0, 0, 1, 1], "texture": 0} for f in ("north", "east", "south", "west", "up", "down")},
        "type": "cube", "uuid": e["uuid"]})
outliner = []
for gname, (origin, idx) in groups.items():
    outliner.append({"name": gname, "origin": origin, "color": 0, "uuid": uid(), "export": True, "isOpen": True,
                     "children": [elements[i]["uuid"] for i in idx]})
model = {
    "meta": {"format_version": "4.5", "model_format": "free", "box_uv": True},
    "name": "quadcopter", "model_identifier": "", "visible_box": [1, 1, 0],
    "variable_placeholders": "", "resolution": {"width": W, "height": H},
    "elements": bb_elements, "outliner": outliner,
    "textures": [{"path": "", "name": "quadcopter.png", "folder": "", "namespace": "", "id": "0", "particle": False,
                  "render_mode": "normal", "render_sides": "auto", "frame_time": 1, "frame_order_type": "loop",
                  "frame_interpolate": False, "visible": True, "mode": "bitmap", "saved": True, "uuid": uid(),
                  "relative_path": "../../textures/entity/quadcopter.png",
                  "source": "data:image/png;base64," + base64.b64encode(tex).decode()}],
}
os.makedirs(os.path.join(ROOT, "models", "entity"), exist_ok=True)
json.dump(model, open(os.path.join(ROOT, "models", "entity", "quadcopter.bbmodel"), "w"), indent=1)
print("model:", len(elements), "elements,", len(groups), "groups")

# ---------------------------------------------------------------- sounds (numpy-free synth -> wav -> ogg)
RATE = 44100
def write_wav(path, samples):
    with wave.open(path, "wb") as w:
        w.setnchannels(1); w.setsampwidth(2); w.setframerate(RATE)
        w.writeframes(b"".join(struct.pack("<h", int(max(-1, min(1, s)) * 32000)) for s in samples))

def to_ogg(samples, name):
    tmp = os.path.join(ROOT, "sounds", name + ".wav")
    os.makedirs(os.path.dirname(tmp), exist_ok=True)
    write_wav(tmp, samples)
    subprocess.check_call(["ffmpeg", "-y", "-loglevel", "error", "-i", tmp, "-c:a", "libvorbis", "-q:a", "4", "-ac", "1",
                           os.path.join(ROOT, "sounds", name + ".ogg")])
    os.remove(tmp)

def motor_loop(seconds=2.0, f0=190.0):
    n = int(RATE * seconds)
    cycles = round(f0 * seconds)
    f = cycles / seconds                         # whole cycles -> seamless loop
    out = []
    for i in range(n):
        t = i / RATE
        s = 0.0
        for k, a in ((1, 0.55), (2, 0.35), (3, 0.28), (4, 0.18), (6, 0.12), (8, 0.07)):
            s += a * math.sin(2 * math.pi * f * k * t + k)
        s *= 0.75 + 0.25 * math.sin(2 * math.pi * 4 * t)           # prop beating, 4 Hz divides 2 s evenly
        out.append(s * 0.28)
    # periodic hiss: sum of random-phase sines at integer multiples of 1/seconds
    hiss = [0.0] * n
    for k in range(1500, 6000, 37):
        ph = random.random() * 6.283
        amp = 0.012
        for i in range(0, n):
            hiss[i] += amp * math.sin(2 * math.pi * (k / seconds) * i / RATE + ph)
    return [max(-1, min(1, a + b)) for a, b in zip(out, hiss)]

def tone_seq(parts):
    out = []
    for freq, dur, vol in parts:
        n = int(RATE * dur)
        for i in range(n):
            env = min(1.0, i / 220, (n - i) / 220)
            out.append(0 if freq == 0 else vol * env * math.sin(2 * math.pi * freq * i / RATE))
    return out

to_ogg(motor_loop(), "drone_motor")
to_ogg(tone_seq([(2900, 0.12, 0.5), (0, 0.08, 0), (2900, 0.12, 0.5)]), "beep_low_battery")
to_ogg(tone_seq([(1400, 0.07, 0.45), (2100, 0.09, 0.45), (2800, 0.12, 0.45)]), "link_up")
to_ogg(tone_seq([(2400, 0.18, 0.5), (1800, 0.18, 0.5), (1200, 0.3, 0.5)]), "link_lost")
to_ogg(tone_seq([(1800, 0.05, 0.4)]), "rec_click")
json.dump({
    "drone_motor": {"category": "neutral", "sounds": [{"name": "openfpv:drone_motor", "stream": False}]},
    "beep_low_battery": {"category": "neutral", "sounds": [{"name": "openfpv:beep_low_battery", "stream": False}]},
    "link_up": {"category": "neutral", "sounds": [{"name": "openfpv:link_up", "stream": False}]},
    "link_lost": {"category": "neutral", "sounds": [{"name": "openfpv:link_lost", "stream": False}]},
    "rec_click": {"category": "neutral", "sounds": [{"name": "openfpv:rec_click", "stream": False}]},
}, open(os.path.join(ROOT, "sounds.json"), "w"), indent=2)
print("sounds written")
