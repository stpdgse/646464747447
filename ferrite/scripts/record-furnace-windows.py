#!/usr/bin/env python3
"""Records inventory clicks inside furnace, blast furnace and smoker windows.

The scripted client opens a furnace holding random items, clicks at random, and after every click
we read the window as the client sees it plus the furnace's NBT and the experience orbs it spawned.
`crates/ferrite-game/tests/furnace_windows.rs` replays the clicks.

    scripts/record-furnace-windows.py --interact-bin target/debug/ferrite-interact \
        --out crates/ferrite-game/tests/data/furnace_windows.txt

Needs the local vanilla server (offline, frozen ticks, RCON). Local oracle testing only.
"""
import argparse, glob, json, os, random, re, sys, time

BOT = "InteractBot"
sys.path.insert(0, os.path.dirname(__file__))
exec(open(os.path.join(os.path.dirname(__file__), "record-physics.py")).read().split("def scenarios")[0].split("ARENA =")[0].replace('if __name__ == "__main__":', "if False:").replace("stderr=subprocess.DEVNULL", "stderr=open(os.environ.get('FERRITE_BOT_LOG', os.devnull), 'w')"))

DATA = "/tmp/claude-0/gen-work/out"
POS = (8, 101, 8)
MAX = {"bucket": 16, "lava_bucket": 1, "water_bucket": 1, "ender_pearl": 16, "iron_helmet": 1, "diamond_sword": 1}
INV = ["raw_iron", "coal", "stick", "bucket", "lava_bucket", "iron_ingot", "beef", "cooked_beef", "stone", "oak_log",
       "oak_planks", "wet_sponge", "sand", "blaze_rod", "diamond", "raw_gold", "potato", "iron_helmet", "dirt"]


def target(slot):
    if 9 <= slot <= 35: return f"inventory.{slot - 9}"
    if 36 <= slot <= 44: return f"hotbar.{slot - 36}"
    return None


def read_inv(bot):
    bot.p.stdin.write("dump\n"); bot.p.stdin.flush()
    while True:
        line = bot._read()
        if line.startswith("inv "):
            parts = line.split()
            return parts[2], parts[3:]


def furnace_nbt(r):
    text = r.cmd(f"data get block {POS[0]} {POS[1]} {POS[2]}")
    g = lambda k: int(re.search(k + r": (-?\d+)", text).group(1))
    slots = ["-", "-", "-"]
    for c, sl, name in re.findall(r'\{count: (\d+), Slot: (\d)b, id: "minecraft:(\w+)"', text):
        slots[int(sl)] = f"{name}*{c}"
    used = re.search(r"RecipesUsed: \{(.*?)\}", text)
    used = ",".join(f"{k}={v}" for k, v in re.findall(r'"([\w:/.]+)": (\d+)', used.group(1))) if used and used.group(1) else "-"
    return [g("lit_time_remaining"), g("lit_total_time"), g("cooking_time_spent"), g("cooking_total_time"), *slots, used]


def orbs(r):
    """The experience of the orbs the server spawned since the last call (they are removed)."""
    total = 0
    for _ in range(100):
        text = r.cmd("data get entity @e[type=experience_orb,limit=1,sort=arbitrary] Value")
        m = re.search(r"(-?\d+)s?\s*$", text.strip())
        if "No entity" in text or not m: break
        total += int(m.group(1))
        r.cmd("kill @e[type=experience_orb,limit=1,sort=arbitrary]")
    return total


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--rcon", default="127.0.0.1:25632")
    ap.add_argument("--password", default="ferrite-oracle")
    ap.add_argument("--game", default="127.0.0.1:25630")
    ap.add_argument("--interact-bin", required=True)
    ap.add_argument("--registries", default=f"{DATA}/reports/registries.json")
    ap.add_argument("--out", required=True)
    ap.add_argument("--scenarios", type=int, default=45)
    ap.add_argument("--clicks", type=int, default=10)
    ap.add_argument("--seed", type=int, default=21)
    args = ap.parse_args()
    r = Rcon(args.rcon, args.password)
    r.cmd("tick freeze")
    r.cmd("forceload add 0 0")
    bot = Bot(args.interact_bin, args.game)
    r.cmd(f"gamemode survival {BOT}")
    bot.teleport(r, f"tp {BOT} 8.5 99.0 5.5 0 -20")
    rng = random.Random(args.seed)
    cookers = ["furnace", "blast_furnace", "smoker"]
    recipe_names = ["minecraft:iron_ingot_from_smelting_raw_iron", "minecraft:cooked_beef", "minecraft:baked_potato", "minecraft:stone"]
    with open(args.out, "w") as f:
        f.write("# recorded from the official server by scripts/record-furnace-windows.py\n")
        for n in range(args.scenarios):
            cooker = cookers[n % 3]
            r.cmd("fill 5 99 5 11 105 11 air")
            r.cmd(f"clear {BOT}")
            r.cmd("kill @e[type=item]")
            r.cmd("kill @e[type=experience_orb]")
            for slot in rng.sample(range(9, 45), rng.randint(4, 14)):
                item = rng.choice(INV)
                count = 1 if item in ("lava_bucket", "iron_helmet") else rng.choice([1, 2, 5, MAX.get(item, 64) // 2 or 1, MAX.get(item, 64)])
                r.cmd(f"item replace entity {BOT} {target(slot)} with minecraft:{item} {count}")
            items = []
            for sl, pool in ((0, INV), (1, INV), (2, INV)):
                if rng.random() < 0.7:
                    item = rng.choice(pool)
                    count = 1 if item in ("lava_bucket", "iron_helmet") else rng.choice([1, 3, MAX.get(item, 64) // 2 or 1, MAX.get(item, 64)])
                    items.append(f'{{Slot:{sl}b,id:"minecraft:{item}",count:{count}}}')
            used = ""
            if rng.random() < 0.5:
                used = ",RecipesUsed:{" + ",".join(f'"{k}":{rng.randint(1, 9)}' for k in rng.sample(recipe_names, rng.randint(1, 2))) + "}"
            lit = rng.choice([0, 0, 120, 1500]); total = rng.choice([0, 100, 200]); spent = rng.randint(0, 90) if total else 0
            r.cmd(f"setblock {POS[0]} {POS[1]} {POS[2]} minecraft:{cooker}[facing=north]"
                  f"{{Items:[{','.join(items)}],lit_time_remaining:{lit}s,lit_total_time:{max(lit, 1600)}s,"
                  f"cooking_time_spent:{spent}s,cooking_total_time:{total}s{used}}}")
            r.cmd("tick step 2"); time.sleep(0.3)
            bot.seq += 1
            bot.p.stdin.write(f"use {POS[0]} {POS[1]} {POS[2]} 1 {bot.seq}\n"); bot.p.stdin.flush()
            while bot._read() != f"ack {bot.seq}": pass
            time.sleep(0.4)
            carried, slots = read_inv(bot)
            if len(slots) != 39:
                print(f"scenario {n}: window did not open ({len(slots)} slots)", file=sys.stderr)
                bot.p.stdin.write("close\n"); bot.p.stdin.flush(); time.sleep(0.2)
                continue
            f.write(f"scenario {n} {cooker}\nfurnace {' '.join(str(v) for v in furnace_nbt(r))} orbs={orbs(r)}\n")
            f.write(f"init {carried} {' '.join(slots)}\n")
            size = len(slots)
            clicks = []
            for _ in range(args.clicks):
                kind = rng.choice(["pickup"] * 5 + ["quick"] * 4 + ["swap"] * 2 + ["throw", "all", "drag"])
                s = rng.randrange(size) if rng.random() < 0.9 else -999
                if rng.random() < 0.5: s = rng.randrange(3)  # most action is in the furnace's own slots
                if kind == "pickup": clicks.append((s, rng.choice([0, 1]), 0))
                elif kind == "quick" and s >= 0: clicks.append((s, 0, 1))
                elif kind == "swap" and s >= 0: clicks.append((s, rng.randrange(9), 2))
                elif kind == "throw" and s >= 0: clicks.append((s, rng.choice([0, 1]), 4))
                elif kind == "all" and s >= 0: clicks.append((s, 0, 6))
                elif kind == "drag":
                    right = rng.choice([0, 4])
                    clicks.append((-999, right, 5))
                    for ds in rng.sample(range(size), rng.randint(1, 3)): clicks.append((ds, right + 1, 5))
                    clicks.append((-999, right + 2, 5))
            for (slot, button, kind) in clicks:
                bot.p.stdin.write(f"click {slot} {button} {kind}\n"); bot.p.stdin.flush()
                time.sleep(0.15)
                r.cmd("kill @e[type=item]")
                carried, slots = read_inv(bot)
                f.write(f"click {slot} {button} {kind}\nafter {carried} {' '.join(slots)}\n")
                f.write(f"furnace {' '.join(str(v) for v in furnace_nbt(r))} orbs={orbs(r)}\n")
            f.write("end\n")
            f.flush()
            bot.p.stdin.write("close\n"); bot.p.stdin.flush(); time.sleep(0.3)
            r.cmd("kill @e[type=item]")
            if n % 10 == 9: print(f"{n + 1} scenarios", file=sys.stderr, flush=True)
    bot.close()


main()
