#!/usr/bin/env python3
"""Records what the official server's 2x2 crafting grid makes from concrete items.

Every crafting recipe that fits a 2x2 grid is laid out with real items (the first item of each
tag, in both orientations for shaped recipes), plus random grids that should usually make
nothing. A scripted client reports the result slot. `crates/ferrite-crafting/tests/vanilla.rs`
replays them.

    scripts/record-crafting.py --interact-bin target/debug/ferrite-interact \
        --out crates/ferrite-crafting/tests/data/crafting.txt
"""
import argparse, glob, json, os, random, sys, time

BOT = "InteractBot"
DATA = "/tmp/claude-0/gen-work/out/data/minecraft"
sys.path.insert(0, os.path.dirname(__file__))
exec(open(os.path.join(os.path.dirname(__file__), "record-physics.py")).read().split("def scenarios")[0].split("ARENA =")[0].replace('if __name__ == "__main__":', "if False:"))


def tag_items(tag, seen=None):
    seen = seen or set()
    if tag in seen: return []
    seen.add(tag)
    path = f"{DATA}/tags/item/{tag.split(':')[1]}.json"
    if not os.path.exists(path): return []
    out = []
    for v in json.load(open(path))["values"]:
        v = v["id"] if isinstance(v, dict) else v
        out += tag_items(v[1:], seen) if v.startswith("#") else [v]
    return out


def options(ing):
    """All items an ingredient accepts (a string or a list)."""
    out = []
    for x in ([ing] if isinstance(ing, str) else ing):
        out += tag_items(x[1:]) if x.startswith("#") else [x]
    return out


def read_inv(bot):
    bot.p.stdin.write("dump\n"); bot.p.stdin.flush()
    while True:
        line = bot._read()
        if line.startswith("inv "):
            return line.split()[3:]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--rcon", default="127.0.0.1:25622")
    ap.add_argument("--password", default="ferrite-oracle")
    ap.add_argument("--game", default="127.0.0.1:25620")
    ap.add_argument("--interact-bin", required=True)
    ap.add_argument("--registries", default="/tmp/claude-0/gen-work/out/reports/registries.json")
    ap.add_argument("--out", required=True)
    ap.add_argument("--random", type=int, default=150)
    args = ap.parse_args()
    ids = {k: v["protocol_id"] for k, v in json.load(open(args.registries))["minecraft:item"]["entries"].items()}
    rng = random.Random(3)
    cases = []  # list of 4 item names (or None)
    for f in sorted(glob.glob(f"{DATA}/recipe/*.json")):
        d = json.load(open(f))
        t = d["type"]
        if t == "minecraft:crafting_shaped":
            pat = d["pattern"]; h, w = len(pat), len(pat[0])
            if w > 2 or h > 2: continue
            for _ in range(2):
                grid = [[None] * 2 for _ in range(2)]
                pick = {k: rng.choice(options(v)) for k, v in d["key"].items() if options(v)}
                for y, row in enumerate(pat):
                    for x, ch in enumerate(row):
                        if ch != " ": grid[y][x] = pick.get(ch)
                # shift/mirror
                if rng.random() < 0.5: grid = [r[::-1] for r in grid]
                cases.append([c for r in grid for c in r])
        elif t == "minecraft:crafting_shapeless":
            ing = d["ingredients"]
            if len(ing) > 4: continue
            picks = [rng.choice(options(i)) for i in ing if options(i)]
            slots = rng.sample(range(4), len(picks))
            grid = [None] * 4
            for s, p in zip(slots, picks): grid[s] = p
            cases.append(grid)
    pool = sorted({i for c in cases for i in c if i})
    for _ in range(args.random):
        cases.append([rng.choice(pool + [None] * 6) for _ in range(4)])
    r = Rcon(args.rcon, args.password)
    r.cmd("tick freeze")
    bot = Bot(args.interact_bin, args.game)
    r.cmd(f"gamemode survival {BOT}")
    bot.teleport(r, f"tp {BOT} 8.5 99.0 5.5 0 -20")
    with open(args.out, "w") as f:
        f.write("# recorded from the official server by scripts/record-crafting.py: grid slots 1-4 -> result\n")
        for n, grid in enumerate(cases):
            r.cmd(f"clear {BOT}")
            for i in range(4):
                if grid[i]:
                    r.cmd(f"item replace entity {BOT} player.crafting.{i} with {grid[i]}")
            r.cmd("tick step 2"); time.sleep(0.25)
            slots = read_inv(bot)
            gr = " ".join(f"{ids[g]}" if g else "-" for g in grid)
            f.write(f"grid {gr} -> {slots[0]}\n")
            f.flush()
            if n % 50 == 49: print(f"{n + 1}/{len(cases)}", file=sys.stderr, flush=True)
    bot.close()


main()
