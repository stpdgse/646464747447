#!/usr/bin/env python3
"""Records how the official server's furnaces, blast furnaces and smokers tick.

Random furnaces (ingredient, fuel, output, burn and cooking timers) are placed in a frozen
server, stepped, and read back through `data get block` at a series of ticks.
`crates/ferrite-game/tests/furnace.rs` replays them through the Rust model.

    scripts/record-furnace.py --out crates/ferrite-game/tests/data/furnace.txt

The vanilla server must run with `pause-when-empty-seconds=-1` and RCON on 127.0.0.1:25632, and
chunk 0,0 force loaded (`forceload add 0 0`). Local oracle testing only.
"""
import argparse, glob, json, random, re, socket, struct, sys, time

DATA = "/tmp/claude-0/gen-work/out"
CHECKPOINTS = [1, 2, 3, 4, 5, 7, 10, 20, 50, 100, 150, 200, 201, 202, 300, 401, 402, 420]


class Rcon:
    def __init__(s, addr, pw):
        h, p = addr.rsplit(":", 1); s.s = socket.create_connection((h, int(p))); s.i = 0
        s._send(3, pw); s._read()

    def _send(s, k, b):
        s.i += 1; b = b.encode(); p = struct.pack("<ii", s.i, k) + b + b"\0\0"; s.s.sendall(struct.pack("<i", len(p)) + p)

    def _read(s):
        def rd(n):
            d = b""
            while len(d) < n:
                c = s.s.recv(n - len(d)); assert c; d += c
            return d
        n = struct.unpack("<i", rd(4))[0]; d = rd(n); return struct.unpack("<ii", d[:8])[0], d[8:-2].decode()

    def cmd(s, c):
        s._send(2, c); return s._read()[1]


def tag_items(tag, seen=None):
    seen = seen if seen is not None else set()
    if tag in seen: return []
    seen.add(tag)
    try: vals = json.load(open(f"{DATA}/data/minecraft/tags/item/{tag.split(':')[1]}.json"))["values"]
    except FileNotFoundError: return []
    out = []
    for v in vals:
        v = v["id"] if isinstance(v, dict) else v
        out += tag_items(v[1:], seen) if v.startswith("#") else [v]
    return out


def options(ing):
    out = []
    for x in ([ing] if isinstance(ing, str) else ing):
        out += tag_items(x[1:]) if x.startswith("#") else [x]
    return out


def fuels():
    out = []
    for f in sorted(glob.glob(f"{DATA}/reports/minecraft/components/item/*.json")):
        c = json.load(open(f))["components"]
        if "minecraft:cooking_fuel" in c:
            out.append((f.rsplit("/", 1)[1][:-5], c.get("minecraft:max_stack_size", 64)))
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--rcon", default="127.0.0.1:25632")
    ap.add_argument("--password", default="ferrite-oracle")
    ap.add_argument("--out", required=True)
    ap.add_argument("--batches", type=int, default=3)
    ap.add_argument("--seed", type=int, default=11)
    args = ap.parse_args()
    rng = random.Random(args.seed)
    r = Rcon(args.rcon, args.password)

    types = {"furnace": "minecraft:smelting", "blast_furnace": "minecraft:blasting", "smoker": "minecraft:smoking"}
    recipes = {c: [] for c in types}
    for f in sorted(glob.glob(f"{DATA}/data/minecraft/recipe/*.json")):
        d = json.load(open(f))
        for c, t in types.items():
            if d["type"] == t:
                for item in options(d["ingredient"]):
                    recipes[c].append((item.split(":")[1], d["result"]["id"].split(":")[1], d.get("cookingtime", 200)))
    fuel_items = fuels()
    junk = ["stone", "bucket", "diamond", "gold_ingot", "iron_ingot", "dirt"]

    def gt():
        return int(re.search(r"(\d+) tick", r.cmd("time query gametime")).group(1))

    def step(n):
        t = gt() + n; r.cmd(f"tick step {n}")
        end = time.time() + 120
        while time.time() < end:
            time.sleep(0.05)
            if gt() >= t: return
        sys.exit("tick step stalled")

    def stack(name, n): return f'{{id:"minecraft:{name}",count:{n}}}'

    out = open(args.out, "w")
    out.write("# Recorded from the official server by scripts/record-furnace.py. Format:\n"
              "# S <cooker> <lit> <lit_total> <spent> <ctotal> <slot0> <slot1> <slot2>   initial state\n"
              "# T <tick> <lit> <lit_total> <spent> <ctotal> <slot0> <slot1> <slot2> <recipes used>\n")
    r.cmd("tick freeze")
    for batch in range(args.batches):
        scenarios = []
        for i in range(200):
            x, z = i % 16, i // 16
            cooker = rng.choice(list(types))
            rs = recipes[cooker]
            roll = rng.random()
            if roll < 0.75: ing, res, time_ = rng.choice(rs)
            elif roll < 0.9: ing, res, time_ = rng.choice(junk), None, None
            else: ing, res, time_ = None, None, None
            speed = 2 if cooker != "furnace" else 1
            count = rng.choice([1, 1, 2, 3, 5, 8, 16, 64])
            roll = rng.random()
            if roll < 0.7: fuel, fmax = rng.choice(fuel_items)
            elif roll < 0.85: fuel, fmax = rng.choice(junk), 64
            else: fuel, fmax = None, 1
            fcount = rng.randint(1, min(3, fmax)) if fuel else 0
            roll = rng.random()
            if roll < 0.5 or res is None: outs = None
            elif roll < 0.75: outs = (res, rng.choice([1, 10, 62, 63, 64]))
            else: outs = (rng.choice(["gold_ingot", "stick", "dirt"]), rng.choice([1, 20, 64]))
            ctotal = (time_ // speed) if time_ else 0
            if rng.random() < 0.1: ctotal = 0
            elif rng.random() < 0.08: ctotal = rng.choice([50, 150, 400])
            lit = rng.choice([0, 0, 0, rng.randint(1, 40), rng.randint(1, 400)])
            lit_total = lit + rng.choice([0, 50, 1600]) if lit else rng.choice([0, 1600, 200])
            spent = rng.choice([0, 0, 0, rng.randint(0, max(1, ctotal - 1))]) if ctotal else 0
            scenarios.append(dict(x=x, z=z, cooker=cooker, ing=ing, count=count, fuel=fuel, fcount=fcount, outs=outs,
                                  ctotal=ctotal, lit=lit, lit_total=lit_total, spent=spent))
        pos = lambda s: f"{s['x']} -50 {s['z']}"
        for s in scenarios:
            items = []
            if s["ing"]: items.append(f'{{Slot:0b,{stack(s["ing"], s["count"])[1:-1]}}}')
            if s["fuel"]: items.append(f'{{Slot:1b,{stack(s["fuel"], s["fcount"])[1:-1]}}}')
            if s["outs"]: items.append(f'{{Slot:2b,{stack(*s["outs"])[1:-1]}}}')
            nbt = (f'{{Items:[{",".join(items)}],lit_time_remaining:{s["lit"]}s,lit_total_time:{s["lit_total"]}s,'
                   f'cooking_time_spent:{s["spent"]}s,cooking_total_time:{s["ctotal"]}s}}')
            r.cmd(f"setblock {pos(s)} minecraft:air")
            reply = r.cmd(f"setblock {pos(s)} minecraft:{s['cooker']}[facing=north]{nbt}")
            assert "Changed" in reply, reply

        def read(s):
            text = r.cmd(f"data get block {pos(s)}")
            g = lambda k: int(re.search(k + r": (-?\d+)", text).group(1))
            slots = ["-", "-", "-"]
            for c, sl, name in re.findall(r'\{count: (\d+), Slot: (\d)b, id: "minecraft:(\w+)"', text):
                slots[int(sl)] = f"{name}*{c}"
            used = re.search(r"RecipesUsed: \{(.*?)\}", text)
            used = ",".join(f"{k}={v}" for k, v in re.findall(r'"([\w:/.]+)": (\d+)', used.group(1))) if used and used.group(1) else "-"
            return (g("lit_time_remaining"), g("lit_total_time"), g("cooking_time_spent"), g("cooking_total_time"), *slots, used)

        def fmt(it, c): return "-" if not it else f"{it}*{c}"
        init = []
        for s in scenarios:
            init.append(f"S {s['cooker']} {s['lit']} {s['lit_total']} {s['spent']} {s['ctotal']} "
                        f"{fmt(s['ing'], s['count'])} {fmt(s['fuel'], s['fcount'])} {fmt(*s['outs']) if s['outs'] else '-'}")
        logs = [[] for _ in scenarios]
        last = 0
        for t in CHECKPOINTS:
            step(t - last); last = t
            for k, s in enumerate(scenarios):
                logs[k].append(f"T {t} " + " ".join(str(v) for v in read(s)))
        for k in range(len(scenarios)):
            out.write("\n" + init[k] + "\n" + "\n".join(logs[k]) + "\n")
        r.cmd("tick step 0")
        print(f"batch {batch} done", file=sys.stderr)
    out.close()


if __name__ == "__main__":
    main()
