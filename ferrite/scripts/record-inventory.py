#!/usr/bin/env python3
"""Records how the official server handles inventory clicks.

A scripted client (`ferrite-interact`) sets up a random inventory, sends random clicks (pickup,
quick move, swap, throw, drag, double click) and reports the inventory it ends up believing in
after each one. `crates/ferrite-game/tests/inventory.rs` replays the clicks against Ferrite's
inventory logic and requires identical results.

    scripts/record-inventory.py --interact-bin target/debug/ferrite-interact \
        --out crates/ferrite-game/tests/data/inventory.txt [--scenarios 200]
"""
import argparse, json, os, random, sys, time

BOT = "InteractBot"
sys.path.insert(0, os.path.dirname(__file__))
exec(open(os.path.join(os.path.dirname(__file__), "record-physics.py")).read().split("def scenarios")[0].split("ARENA =")[0].replace('if __name__ == "__main__":', "if False:"))

PALETTE = ["stone", "dirt", "ender_pearl", "iron_helmet", "iron_chestplate", "iron_leggings", "iron_boots", "diamond_sword", "shield", "oak_log"]
MAX = {"stone": 64, "dirt": 64, "ender_pearl": 16, "oak_log": 64}


def target(slot):
    if 9 <= slot <= 35: return f"inventory.{slot - 9}"
    if 36 <= slot <= 44: return f"hotbar.{slot - 36}"
    return {5: "armor.head", 6: "armor.chest", 7: "armor.legs", 8: "armor.feet", 45: "weapon.offhand"}.get(slot)


def read_inv(bot):
    bot.p.stdin.write("dump\n"); bot.p.stdin.flush()
    while True:
        line = bot._read()
        if line.startswith("inv "):
            parts = line.split()
            return int(parts[1]), parts[2], parts[3:]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--rcon", default="127.0.0.1:25622")
    ap.add_argument("--password", default="ferrite-oracle")
    ap.add_argument("--game", default="127.0.0.1:25620")
    ap.add_argument("--interact-bin", required=True)
    ap.add_argument("--registries", default="/tmp/claude-0/gen-work/out/reports/registries.json")
    ap.add_argument("--out", required=True)
    ap.add_argument("--scenarios", type=int, default=100)
    ap.add_argument("--clicks", type=int, default=8)
    ap.add_argument("--seed", type=int, default=1)
    args = ap.parse_args()
    ids = {k.replace("minecraft:", ""): v["protocol_id"] for k, v in json.load(open(args.registries))["minecraft:item"]["entries"].items()}
    r = Rcon(args.rcon, args.password)
    r.cmd("tick freeze")
    bot = Bot(args.interact_bin, args.game)
    r.cmd(f"gamemode survival {BOT}")
    bot.teleport(r, f"tp {BOT} 8.5 99.0 5.5 0 -20")
    rng = random.Random(args.seed)
    with open(args.out, "w") as f:
        f.write("# recorded from the official server by scripts/record-inventory.py\n# item ids are protocol ids of this version\n")
        for n in range(args.scenarios):
            r.cmd(f"clear {BOT}")
            r.cmd("kill @e[type=item]")
            placed = {}
            for _ in range(rng.randint(3, 14)):
                slot = rng.choice([5, 6, 7, 8, 45] + list(range(9, 45)) * 2)
                item = rng.choice(PALETTE)
                if slot in (5, 6, 7, 8) and rng.random() < 0.7:
                    item = ["iron_helmet", "iron_chestplate", "iron_leggings", "iron_boots"][slot - 5]
                count = 1 if item not in MAX else rng.choice([1, 2, 5, MAX[item] // 2, MAX[item] - 1, MAX[item]])
                placed[slot] = (item, count)
            for slot, (item, count) in placed.items():
                r.cmd(f"item replace entity {BOT} {target(slot)} with minecraft:{item} {count}")
            r.cmd("tick step 2"); time.sleep(0.4)
            state, carried, slots = read_inv(bot)
            f.write(f"scenario {n}\ninit {carried} {' '.join(slots)}\n")
            clicks = []
            for _ in range(args.clicks):
                kind = rng.choice(["pickup"] * 5 + ["quick"] * 3 + ["swap"] * 2 + ["throw", "all", "drag", "drag"])
                s = rng.choice(list(range(1, 46)) + [-999])
                if kind == "pickup": clicks.append((s, rng.choice([0, 1]), 0))
                elif kind == "quick" and s >= 0: clicks.append((s, 0, 1))
                elif kind == "swap" and s >= 0: clicks.append((s, rng.choice(list(range(9)) + [40]), 2))
                elif kind == "throw" and s >= 0: clicks.append((s, rng.choice([0, 1]), 4))
                elif kind == "all" and s >= 0: clicks.append((s, 0, 6))
                elif kind == "drag":
                    right = rng.choice([0, 4])
                    clicks.append((-999, right, 5))
                    for ds in rng.sample(range(9, 45), rng.randint(1, 4)): clicks.append((ds, right + 1, 5))
                    clicks.append((-999, right + 2, 5))
            for (slot, button, kind) in clicks:
                bot.p.stdin.write(f"click {slot} {button} {kind}\n"); bot.p.stdin.flush()
                time.sleep(0.12)
                r.cmd("kill @e[type=item]")
                state, carried, slots = read_inv(bot)
                f.write(f"click {slot} {button} {kind}\nafter {carried} {' '.join(slots)}\n")
            f.write("end\n")
            f.flush()
            if n % 10 == 9: print(f"{n + 1} scenarios", file=sys.stderr, flush=True)
    bot.close()


main()
