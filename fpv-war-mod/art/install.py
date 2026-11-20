#!/usr/bin/env python3
"""Install built art (Blockbench-exported geo, texture, animations) into the mod resources.
usage: install.py <name> [<geo-identifier>]   expects art/out/export/export_bedrock.txt, art/out/<name>.png, art/out/<name>.animation.json"""
import json, os, shutil, sys
root = os.path.dirname(os.path.abspath(__file__)); res = os.path.join(root, "..", "src/main/resources/assets/kamikaze_skies")
name = sys.argv[1]; ident = sys.argv[2] if len(sys.argv) > 2 else f"geometry.kamikaze_skies.{name}"
out = os.path.join(root, "out")
geo = json.load(open(os.path.join(out, "export", f"export_bedrock_{name}.txt" if os.path.exists(os.path.join(out, "export", f"export_bedrock_{name}.txt")) else "export_bedrock.txt")))
geo["minecraft:geometry"][0]["description"]["identifier"] = ident
for sub, src, dst in [("geckolib/models/entity", None, f"{name}.geo.json"), ("geckolib/animations/entity", f"{name}.animation.json", f"{name}.animation.json"), ("textures/entity", f"{name}.png", f"{name}.png")]:
    d = os.path.join(res, sub); os.makedirs(d, exist_ok=True)
    if src is None: json.dump(geo, open(os.path.join(d, dst), "w"), indent=1)
    else: shutil.copy(os.path.join(out, src), os.path.join(d, dst))
bones = {b["name"] for b in geo["minecraft:geometry"][0]["bones"]}
anims = json.load(open(os.path.join(out, f"{name}.animation.json")))["animations"]
used = {b for a in anims.values() for b in a["bones"]}
assert used <= bones, f"animation bones missing from geo: {used - bones}"
print(f"installed {name}: {len(bones)} bones, {len(anims)} animations")
