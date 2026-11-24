#!/usr/bin/env python3
"""Records inventory clicks inside container windows (crafting table, chest, double chest).

Like record-inventory.py, but the scripted client first opens the window by right-clicking the
block. `crates/ferrite-game/tests/windows.rs` replays the clicks against `Menu`.

    scripts/record-windows.py --interact-bin target/debug/ferrite-interact \
        --out crates/ferrite-game/tests/data/windows.txt
"""
import argparse, json, os, random, sys, time

BOT = "InteractBot"
sys.path.insert(0, os.path.dirname(__file__))
exec(open(os.path.join(os.path.dirname(__file__), "record-physics.py")).read().split("def scenarios")[0].split("ARENA =")[0].replace('if __name__ == "__main__":', "if False:"))

PALETTE = ["stone", "dirt", "ender_pearl", "iron_helmet", "diamond_sword", "oak_log", "oak_planks", "stick", "cobblestone"]
MAX = {"stone": 64, "dirt": 64, "ender_pearl": 16, "oak_log": 64, "oak_planks": 64, "stick": 64, "cobblestone": 64}
POS = (8, 101, 8)


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


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--rcon", default="127.0.0.1:25622")
    ap.add_argument("--password", default="ferrite-oracle")
    ap.add_argument("--game", default="127.0.0.1:25620")
    ap.add_argument("--interact-bin", required=True)
    ap.add_argument("--registries", default="/tmp/claude-0/gen-work/out/reports/registries.json")
    ap.add_argument("--out", required=True)
    ap.add_argument("--scenarios", type=int, default=40)
    ap.add_argument("--clicks", type=int, default=10)
    ap.add_argument("--seed", type=int, default=5)
    args = ap.parse_args()
    r = Rcon(args.rcon, args.password)
    r.cmd("tick freeze")
    bot = Bot(args.interact_bin, args.game)
    r.cmd(f"gamemode survival {BOT}")
    bot.teleport(r, f"tp {BOT} 8.5 99.0 5.5 0 -20")
    rng = random.Random(args.seed)
    modes = ["crafting", "chest", "double"]
    with open(args.out, "w") as f:
        f.write("# recorded from the official server by scripts/record-windows.py\n")
        for n in range(args.scenarios):
            mode = modes[n % 3]
            r.cmd("fill 5 99 5 11 105 11 air")
            r.cmd(f"clear {BOT}")
            r.cmd("kill @e[type=item]")
            chests = []
            if mode == "crafting":
                r.cmd(f"setblock {POS[0]} {POS[1]} {POS[2]} crafting_table")
            elif mode == "chest":
                r.cmd(f"setblock {POS[0]} {POS[1]} {POS[2]} chest[facing=north,type=single]")
                chests = [POS]
            else:
                r.cmd(f"setblock {POS[0]} {POS[1]} {POS[2]} chest[facing=north,type=left]")
                r.cmd(f"setblock {POS[0] + 1} {POS[1]} {POS[2]} chest[facing=north,type=right]")
                chests = [POS, (POS[0] + 1, POS[1], POS[2])]
            for slot in rng.sample(range(9, 45), rng.randint(3, 12)):
                item = rng.choice(PALETTE)
                count = 1 if item not in MAX else rng.choice([1, 2, 5, MAX[item] // 2, MAX[item]])
                r.cmd(f"item replace entity {BOT} {target(slot)} with minecraft:{item} {count}")
            filled = []
            for ci, c in enumerate(chests):
                mine = {}
                for slot in rng.sample(range(27), rng.randint(2, 8)):
                    item = rng.choice(PALETTE)
                    count = 1 if item not in MAX else rng.choice([1, 3, MAX[item] // 2, MAX[item]])
                    mine[slot] = (item, count)
                    r.cmd(f"item replace block {c[0]} {c[1]} {c[2]} container.{slot} with minecraft:{item} {count}")
                filled.append(mine)
            r.cmd("tick step 2"); time.sleep(0.3)
            bot.seq += 1
            bot.p.stdin.write(f"use {POS[0]} {POS[1]} {POS[2]} 1 {bot.seq}\n"); bot.p.stdin.flush()
            while bot._read() != f"ack {bot.seq}": pass
            time.sleep(0.4)
            carried, slots = read_inv(bot)
            layout = {"crafting": "crafting", "chest": "chest3", "double": "chest6"}[mode]
            if len(slots) != {"crafting": 46, "chest3": 63, "chest6": 90}[layout]:
                print(f"scenario {n}: window did not open ({len(slots)} slots)", file=sys.stderr)
                bot.p.stdin.write("close\n"); bot.p.stdin.flush(); time.sleep(0.2)
                continue
            f.write(f"scenario {n} {layout}\ninit {carried} {' '.join(slots)}\n")
            if mode == "double":
                # Which chest's slots come first: the one placed as "left" (at x) or "right" (x+1)?
                occupied_first = {i for i in range(27) if slots[i] != "-"}
                first = 0 if occupied_first == set(filled[0]) else 1 if occupied_first == set(filled[1]) else -1
                f.write(f"# double chest: first half is chest {first} (0 = left, 1 = right)\n")
            size = len(slots)
            clicks = []
            for _ in range(args.clicks):
                kind = rng.choice(["pickup"] * 5 + ["quick"] * 3 + ["swap"] * 2 + ["throw", "all", "drag"])
                s = rng.randrange(size) if rng.random() < 0.92 else -999
                if kind == "pickup": clicks.append((s, rng.choice([0, 1]), 0))
                elif kind == "quick" and s >= 0: clicks.append((s, 0, 1))
                elif kind == "swap" and s >= 0: clicks.append((s, rng.randrange(9), 2))
                elif kind == "throw" and s >= 0: clicks.append((s, rng.choice([0, 1]), 4))
                elif kind == "all" and s >= 0: clicks.append((s, 0, 6))
                elif kind == "drag":
                    right = rng.choice([0, 4])
                    clicks.append((-999, right, 5))
                    for ds in rng.sample(range(size), rng.randint(1, 4)): clicks.append((ds, right + 1, 5))
                    clicks.append((-999, right + 2, 5))
            for (slot, button, kind) in clicks:
                bot.p.stdin.write(f"click {slot} {button} {kind}\n"); bot.p.stdin.flush()
                time.sleep(0.12)
                r.cmd("kill @e[type=item]")
                carried, slots = read_inv(bot)
                f.write(f"click {slot} {button} {kind}\nafter {carried} {' '.join(slots)}\n")
            f.write("end\n")
            f.flush()
            bot.p.stdin.write("close\n"); bot.p.stdin.flush(); time.sleep(0.3)
            r.cmd("kill @e[type=item]")
            if n % 10 == 9: print(f"{n + 1} scenarios", file=sys.stderr, flush=True)
    bot.close()


main()
