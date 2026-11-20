#!/usr/bin/env python3
"""Generate part items, the workbench block and all crafting recipes: textures, models, language entries, recipes.

Everything is derived from the tables below, so balance/economy changes are one edit plus a re-run. Idempotent.
"""
import json, os
from PIL import Image

ROOT = os.path.join(os.path.dirname(__file__), "..", "src", "main", "resources")
A = os.path.join(ROOT, "assets", "kamikaze_skies")
D = os.path.join(ROOT, "data", "kamikaze_skies")
for d in ("textures/item", "textures/block", "models/item", "models/block", "items", "blockstates", "lang"):
    os.makedirs(os.path.join(A, d), exist_ok=True)
for d in ("recipe", "loot_table/blocks"):
    os.makedirs(os.path.join(D, d), exist_ok=True)
os.makedirs(os.path.join(ROOT, "data", "minecraft", "tags", "block", "mineable"), exist_ok=True)

# ---- parts ---------------------------------------------------------------------------------------------
TIER_COLOR = {1: (170, 175, 185, 255), 2: (214, 128, 64, 255), 3: (90, 225, 235, 255)}
CARBON, DARK, RED, GREEN, YELLOW, ORANGE, WHITE = (30, 30, 34, 255), (55, 58, 66, 255), (200, 40, 40, 255), (40, 120, 70, 255), (245, 215, 70, 255), (235, 130, 40, 255), (240, 240, 240, 255)

KINDS = [  # id, display name, tiered
    ("frame", "Frame", True), ("motors", "Motor Set", True), ("props", "Propeller Set", True), ("battery", "Battery Pack", True),
    ("flight_controller", "Flight Controller", True), ("camera", "FPV Camera", True), ("antenna", "Antenna", True),
    ("warhead_impact", "Impact Warhead", False), ("warhead_shaped", "Shaped-Charge Warhead", False), ("blackbox", "Blackbox", False),
]

def draw(kind, tier):
    im = Image.new("RGBA", (16, 16), (0, 0, 0, 0)); px = im.load(); c = TIER_COLOR[tier]
    def rect(x0, y0, x1, y1, col):
        for x in range(x0, x1 + 1):
            for y in range(y0, y1 + 1):
                px[x, y] = col
    if kind == "frame":
        for i in range(2, 14): px[i, i] = CARBON; px[i, 15 - i] = CARBON; px[i + 1 if i < 13 else i, i] = CARBON; px[i + 1 if i < 13 else i, 15 - i] = CARBON
        rect(6, 6, 9, 9, c)
        for x, y in ((2, 2), (13, 2), (2, 13), (13, 13)): rect(x - 1 if x > 2 else x, y - 1 if y > 2 else y, x + 1 if x < 13 else x, y + 1 if y < 13 else y, DARK)
    elif kind == "motors":
        for cx, cy in ((4, 4), (11, 4), (4, 11), (11, 11)):
            rect(cx - 2, cy - 2, cx + 2, cy + 2, c); rect(cx - 1, cy - 1, cx + 1, cy + 1, DARK); px[cx, cy] = CARBON
    elif kind == "props":
        rect(7, 1, 8, 14, c); rect(1, 7, 14, 8, c)
        for i in range(3, 13):
            px[i, i] = c; px[i, 15 - i] = c
        rect(7, 7, 8, 8, DARK)
    elif kind == "battery":
        rect(2, 4, 13, 12, CARBON); rect(2, 6, 13, 9, c); rect(5, 7, 10, 8, DARK); rect(13, 6, 14, 9, YELLOW)
    elif kind == "flight_controller":
        rect(2, 2, 13, 13, GREEN); rect(2, 2, 13, 2, DARK)
        rect(4, 4, 7, 7, CARBON); rect(9, 4, 12, 6, c); rect(4, 9, 6, 12, c); rect(8, 9, 12, 12, CARBON)
        for x, y in ((2, 2), (13, 2), (2, 13), (13, 13)): px[x, y] = YELLOW
    elif kind == "camera":
        rect(3, 3, 12, 12, CARBON); rect(5, 5, 10, 10, c); rect(6, 6, 9, 9, (40, 90, 200, 255)); px[7, 7] = WHITE
    elif kind == "antenna":
        rect(7, 3, 8, 13, (200, 205, 215, 255)); rect(5, 13, 10, 14, CARBON); rect(6, 1, 9, 4, c)
    elif kind == "warhead_impact":
        rect(5, 2, 10, 13, ORANGE); rect(5, 2, 10, 3, DARK); rect(5, 12, 10, 13, DARK); rect(5, 7, 10, 8, YELLOW)
    elif kind == "warhead_shaped":
        for y in range(2, 14):
            w = 1 + (y - 2) // 2
            rect(8 - w, y, 7 + w, y, YELLOW)
        rect(4, 13, 11, 14, DARK)
    elif kind == "blackbox":
        rect(2, 4, 13, 12, CARBON); rect(2, 4, 13, 5, ORANGE); rect(2, 11, 13, 12, ORANGE); rect(6, 7, 9, 8, RED)
    if kind in ("frame", "motors", "props", "battery", "flight_controller", "camera", "antenna"):
        for i in range(tier): px[1 + 2 * i, 15] = WHITE          # tier pips
    return im

names = {}      # registry name -> display name
for kind, label, tiered in KINDS:
    for tier in (range(1, 4) if tiered else [1]):
        reg = f"{kind}_t{tier}" if tiered else kind
        draw(kind, tier).save(os.path.join(A, "textures", "item", f"part_{reg}.png"))
        json.dump({"model": {"type": "minecraft:model", "model": f"kamikaze_skies:item/part_{reg}"}}, open(os.path.join(A, "items", f"part_{reg}.json"), "w"), indent=2)
        json.dump({"parent": "minecraft:item/generated", "textures": {"layer0": f"kamikaze_skies:item/part_{reg}"}}, open(os.path.join(A, "models", "item", f"part_{reg}.json"), "w"), indent=2)
        names[f"item.kamikaze_skies.part_{reg}"] = f"{label} (Tier {tier})" if tiered else label

# ---- workbench ----------------------------------------------------------------------------------------
def block_tex(name, base, detail):
    im = Image.new("RGBA", (16, 16), base); px = im.load()
    detail(px)
    im.save(os.path.join(A, "textures", "block", f"{name}.png"))
def top(px):
    for i in range(16):
        px[i, 0] = px[i, 15] = px[0, i] = px[15, i] = (45, 48, 56, 255)
    for i in range(2, 14, 4):
        for j in range(16): px[i, j] = (66, 70, 80, 255); px[j, i] = (66, 70, 80, 255)
    for (x, y, col) in ((3, 3, RED), (9, 3, TIER_COLOR[2]), (3, 9, TIER_COLOR[3]), (10, 10, GREEN), (7, 6, YELLOW)):
        for dx in range(3):
            for dy in range(3): px[x + dx, y + dy] = col
def side(px):
    for y in range(16):
        for x in range(16):
            px[x, y] = (150, 110, 70, 255) if (y // 4) % 2 == 0 else (132, 96, 60, 255)
    for x in range(16): px[x, 0] = (45, 48, 56, 255)
    for y in range(2, 7):
        for x in range(10, 14): px[x, y] = (100, 105, 115, 255)        # vise
    for x in range(2, 8): px[x, 10] = px[x, 11] = (180, 185, 195, 255)   # tool
def bottom(px):
    for y in range(16):
        for x in range(16): px[x, y] = (120, 88, 56, 255)
block_tex("drone_workbench_top", (58, 62, 72, 255), top)
block_tex("drone_workbench_side", (150, 110, 70, 255), side)
block_tex("drone_workbench_bottom", (120, 88, 56, 255), bottom)
json.dump({"parent": "minecraft:block/cube_bottom_top", "textures": {
    "top": "kamikaze_skies:block/drone_workbench_top", "side": "kamikaze_skies:block/drone_workbench_side", "bottom": "kamikaze_skies:block/drone_workbench_bottom"}},
    open(os.path.join(A, "models", "block", "drone_workbench.json"), "w"), indent=2)
json.dump({"variants": {"": {"model": "kamikaze_skies:block/drone_workbench"}}}, open(os.path.join(A, "blockstates", "drone_workbench.json"), "w"), indent=2)
json.dump({"model": {"type": "minecraft:model", "model": "kamikaze_skies:block/drone_workbench"}}, open(os.path.join(A, "items", "drone_workbench.json"), "w"), indent=2)
json.dump({"type": "minecraft:block", "pools": [{"rolls": 1, "entries": [{"type": "minecraft:item", "name": "kamikaze_skies:drone_workbench"}],
    "conditions": [{"condition": "minecraft:survives_explosion"}]}], "random_sequence": "kamikaze_skies:blocks/drone_workbench"},
    open(os.path.join(D, "loot_table", "blocks", "drone_workbench.json"), "w"), indent=2)
json.dump({"replace": False, "values": ["kamikaze_skies:drone_workbench"]}, open(os.path.join(ROOT, "data", "minecraft", "tags", "block", "mineable", "axe.json"), "w"), indent=2)

# ---- recipes (shapeless; ingredient lists) ---------------------------------------------------------------
def x(item, n): return [item] * n
RECIPES = {
    "frame_t1": x("stick", 4) + ["iron_ingot"],
    "frame_t2": x("iron_ingot", 3) + ["copper_ingot", "stick"],
    "frame_t3": x("iron_ingot", 3) + ["diamond", "copper_ingot"],
    "motors_t1": x("copper_ingot", 2) + ["iron_ingot", "redstone"],
    "motors_t2": x("copper_ingot", 3) + x("iron_ingot", 2) + x("redstone", 2),
    "motors_t3": x("copper_ingot", 3) + ["diamond", "iron_ingot"] + x("redstone", 3),
    "props_t1": x("paper", 2) + ["stick"],
    "props_t2": x("iron_nugget", 3) + ["stick"],
    "props_t3": ["iron_ingot"] + x("amethyst_shard", 2) + ["stick"],
    "battery_t1": x("redstone", 2) + ["iron_ingot", "copper_ingot"],
    "battery_t2": x("redstone", 3) + ["gold_ingot"] + x("copper_ingot", 2),
    "battery_t3": ["redstone_block"] + x("gold_ingot", 2) + ["diamond"],
    "flight_controller_t1": ["redstone", "quartz", "iron_nugget"],
    "flight_controller_t2": x("redstone", 2) + x("quartz", 2) + ["gold_ingot"],
    "flight_controller_t3": x("redstone", 3) + x("quartz", 2) + ["diamond", "gold_ingot"],
    "camera_t1": ["glass", "iron_ingot", "redstone"],
    "camera_t2": x("glass", 2) + ["copper_ingot", "redstone", "quartz"],
    "camera_t3": ["glass", "diamond"] + x("redstone", 2) + ["gold_ingot", "quartz"],
    "antenna_t1": ["copper_ingot", "stick", "iron_nugget"],
    "antenna_t2": ["lightning_rod", "iron_ingot", "redstone"],
    "antenna_t3": ["lightning_rod", "diamond", "gold_ingot", "redstone"],
    "warhead_impact": x("gunpowder", 3) + ["iron_ingot"],
    "warhead_shaped": x("gunpowder", 3) + x("copper_ingot", 2) + ["quartz"],
    "blackbox": ["iron_ingot", "redstone", "obsidian"],
}
for reg, ing in RECIPES.items():
    json.dump({"type": "minecraft:crafting_shapeless", "category": "misc", "ingredients": [f"minecraft:{i}" for i in ing],
               "result": {"id": f"kamikaze_skies:part_{reg}", "count": 1}},
              open(os.path.join(D, "recipe", f"part_{reg}.json"), "w"), indent=2)
json.dump({"type": "minecraft:crafting_shapeless", "category": "misc", "ingredients": ["minecraft:crafting_table", "minecraft:iron_ingot", "minecraft:iron_ingot", "minecraft:redstone", "minecraft:redstone"],
           "result": {"id": "kamikaze_skies:drone_workbench", "count": 1}}, open(os.path.join(D, "recipe", "drone_workbench.json"), "w"), indent=2)
json.dump({"type": "kamikaze_skies:drone_assembly"}, open(os.path.join(D, "recipe", "drone_assembly.json"), "w"), indent=2)

# ---- language ----------------------------------------------------------------------------------------
lp = os.path.join(A, "lang", "en_us.json"); lang = json.load(open(lp))
for k in [k for k in lang if k.startswith("item.kamikaze_skies.fpv_strike_drone") or k.startswith("item.kamikaze_skies.fpv_shaped_drone")]: del lang[k]
lang.update(names)
lang.update({
    "block.kamikaze_skies.drone_workbench": "Drone Workbench",
    "container.kamikaze_skies.workbench": "Drone Workbench",
    "item.kamikaze_skies.fpv_drone": "FPV Drone",
    "tooltip.kamikaze_skies.tier": "Tier %s",
    "tooltip.kamikaze_skies.mass": "Mass: %s g",
    "tooltip.kamikaze_skies.stat.frame": "Survives impacts up to %s m/s",
    "tooltip.kamikaze_skies.stat.motors": "%s N thrust per motor",
    "tooltip.kamikaze_skies.stat.props": "Motor response: %s ms",
    "tooltip.kamikaze_skies.stat.battery": "%s mAh, 4S",
    "tooltip.kamikaze_skies.stat.flight_controller": "Stabilisation gain x%s",
    "tooltip.kamikaze_skies.stat.camera": "%s degree field of view",
    "tooltip.kamikaze_skies.stat.antenna": "Radio range x%s",
    "tooltip.kamikaze_skies.stat.warhead_impact": "Impact-fused blast warhead",
    "tooltip.kamikaze_skies.stat.warhead_shaped": "Shaped-charge warhead: deep, narrow jet",
    "tooltip.kamikaze_skies.stat.blackbox": "Records the last 30 s of flight. Needs a tier 2+ controller.",
    "tooltip.kamikaze_skies.log.cause.0": "Lost: crashed",
    "tooltip.kamikaze_skies.log.cause.1": "Lost: warhead detonated",
    "tooltip.kamikaze_skies.log.cause.2": "Lost: radio link lost",
    "tooltip.kamikaze_skies.log.cause.3": "Lost: battery empty",
    "tooltip.kamikaze_skies.log.flight": "Flight time: %s",
    "tooltip.kamikaze_skies.log.speed": "Top speed: %s km/h",
    "tooltip.kamikaze_skies.log.altitude": "Max altitude: %s m",
    "tooltip.kamikaze_skies.log.volts": "Lowest voltage: %s V",
    "tooltip.kamikaze_skies.log.signal": "Weakest signal: %s%%",
    "tooltip.kamikaze_skies.drone.stats": "Mass %s kg  Thrust/weight %s  Hover %s min",
    "tooltip.kamikaze_skies.drone.parts": "Parts: frame %s, motors %s, props %s, battery %s, controller %s, camera %s, antenna %s",
    "tooltip.kamikaze_skies.drone.warhead.1": "Warhead: impact",
    "tooltip.kamikaze_skies.drone.warhead.2": "Warhead: shaped charge",
    "tooltip.kamikaze_skies.drone.blackbox": "Blackbox installed",
})
json.dump(lang, open(lp, "w"), indent=2)
# ---- docs/RECIPES.md (generated from the same table) ----------------------------------------------------
from collections import Counter
rows = ["# Part recipes (generated)", "",
        "Generated by `scripts/gen_assets.py` from the recipe table; do not edit by hand. All are shapeless: put the ingredients in any "
        "arrangement in a crafting table or Drone Workbench.", "", "| Part | Ingredients |", "|---|---|"]
for reg, ing in RECIPES.items():
    label = names.get(f"item.kamikaze_skies.part_{reg}", reg)
    rows.append(f"| {label} | " + ", ".join(f"{n}x {k.replace('_', ' ')}" if n > 1 else k.replace('_', ' ') for k, n in Counter(ing).items()) + " |")
rows.append("| Drone Workbench | crafting table, 2x iron ingot, 2x redstone |")
open(os.path.join(os.path.dirname(__file__), "..", "docs", "RECIPES.md"), "w").write("\n".join(rows) + "\n")
print(f"generated {len(names)} part items, workbench, {len(RECIPES) + 2} recipes")
