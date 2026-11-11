#!/usr/bin/env python3
"""Records how the official server places blocks.

For each block item and a sample of contexts (clicked face, hit position, the player's yaw and
pitch), a scripted client (`ferrite-interact`) puts the item in its hand and right-clicks a stone
block on a frozen vanilla server. What ends up in the world around the click is written to a text
file that `crates/ferrite-blocks/tests/placement.rs` replays against Ferrite.

    scripts/record-placement.py --world /path/to/server/world --interact-bin target/debug/ferrite-interact \
        --out crates/ferrite-blocks/tests/data/placement.txt [--per-block 20] [--only minecraft:oak_stairs]

Server requirements: enable-rcon=true, pause-when-empty-seconds=-1, offline mode, the whitelist off.
Running the server requires accepting Mojang's EULA yourself.
"""
import argparse, hashlib, json, os, random, struct, subprocess, sys, time, zlib, io, socket

TARGET = (8, 101, 8)
WINDOW = (5, 99, 5, 11, 105, 11)  # x0 y0 z0 x1 y1 z1 (inclusive), cleared before every case
BOT = "InteractBot"
BOT_POS = (8.5, 99.0, 5.5)

sys.path.insert(0, os.path.dirname(__file__))
exec(open(os.path.join(os.path.dirname(__file__), "record-physics.py")).read().split("def scenarios")[0].split("ARENA =")[0].replace('if __name__ == "__main__":', "if False:"))  # Rcon, nbt, chunk reader, Bot


def blocks_in_window(world):
    chunk = read_chunk(world, 0, 0)
    out = {}
    for sec in chunk["sections"]:
        bs = sec.get("block_states")
        y0 = sec["Y"] * 16
        if not bs or y0 + 15 < WINDOW[1] or y0 > WINDOW[4]:
            continue
        names = [palette_name(p) for p in bs["palette"]]
        data = bs.get("data")
        bits = max(4, (len(names) - 1).bit_length())
        per, mask = 64 // bits, (1 << bits) - 1
        for i in range(4096):
            v = (data[i // per] >> ((i % per) * bits)) & mask if data else 0
            x, y, z = i & 15, y0 + (i >> 8), (i >> 4) & 15
            if names[v] != "air" and WINDOW[0] <= x <= WINDOW[3] and WINDOW[1] <= y <= WINDOW[4] and WINDOW[2] <= z <= WINDOW[5]:
                out[(x, y, z)] = names[v]
    return out


def contexts(item, count, oriented):
    if os.environ.get("FERRITE_PLACEMENT_TIES"):
        # Exact ties of the nearest-direction computation: every yaw and pitch multiple of 45
        # (and a few others), clicking the top face.
        yaws = [-180.0, -135.0, -90.0, -45.0, 0.0, 45.0, 90.0, 135.0, 180.0]
        pitches = [-90.0, -80.0, -45.0, -20.0, 0.0, 20.0, 45.0, 80.0, 90.0]
        return [(1, (0.5, 1.0, 0.5), y, p) for y in yaws for p in pitches]
    rng = random.Random(int(hashlib.sha1(item.encode()).hexdigest()[:12], 16))
    out = []
    for _ in range(count if oriented else min(count, 3)):
        face = rng.randrange(6)  # down up north south west east
        a, b = rng.choice([0.1, 0.3, 0.5, 0.7, 0.9]), rng.choice([0.1, 0.3, 0.5, 0.7, 0.9])
        cursor = {0: (a, 0.0, b), 1: (a, 1.0, b), 2: (a, b, 0.0), 3: (a, b, 1.0), 4: (0.0, a, b), 5: (1.0, a, b)}[face]
        yaw = rng.choice([i * 22.5 - 180 for i in range(16)])
        pitch = rng.choice([-80, -45, -20, 0, 0, 20, 45, 80])
        out.append((face, cursor, yaw, float(pitch)))
    return out


def support_contexts(item):
    """The top face, the bottom face and one side, with random look directions."""
    rng = random.Random(int(hashlib.sha1(("support" + item).encode()).hexdigest()[:12], 16))
    out = []
    for face in (1, 0, rng.choice([2, 3, 4, 5])):
        a, b = rng.choice([0.1, 0.3, 0.5, 0.7, 0.9]), rng.choice([0.1, 0.3, 0.5, 0.7, 0.9])
        cursor = {0: (a, 0.0, b), 1: (a, 1.0, b), 2: (a, b, 0.0), 3: (a, b, 1.0), 4: (0.0, a, b), 5: (1.0, a, b)}[face]
        out.append((face, cursor, rng.choice([i * 22.5 - 180 for i in range(16)]), float(rng.choice([-80, -45, -20, 0, 20, 45, 80]))))
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--world", required=True)
    ap.add_argument("--rcon", default="127.0.0.1:25622")
    ap.add_argument("--password", default=os.environ.get("FERRITE_ORACLE_RCON_PASSWORD", "ferrite-oracle"))
    ap.add_argument("--game", default="127.0.0.1:25620")
    ap.add_argument("--interact-bin", required=True)
    ap.add_argument("--blocks-json", required=True, help="blocks.json from the data generator")
    ap.add_argument("--out", required=True)
    ap.add_argument("--per-block", type=int, default=20)
    ap.add_argument("--only", default="", help="record only items whose name starts with this")
    ap.add_argument("--limit", type=int, default=0)
    ap.add_argument("--items", default="", help="comma-separated item names; only these are recorded")
    ap.add_argument("--support", default="stone", help="the block state that is clicked (default stone)")
    ap.add_argument("--contexts", choices=["random", "support"], default="random",
                    help="'support' = three contexts per item: the top face, the bottom face and a side")
    args = ap.parse_args()

    blocks = json.load(open(args.blocks_json))
    oriented_props = {"facing", "axis", "half", "type", "shape", "face", "rotation", "hinge", "part", "attachment",
                      "orientation", "hanging", "north", "east", "south", "west", "up", "down", "bottom"}
    registries = json.load(open(os.path.join(os.path.dirname(args.blocks_json), "registries.json")))
    item_names = set(registries["minecraft:item"]["entries"])
    items = []
    for name, b in blocks.items():
        if name not in item_names:
            continue  # air, water, piston_head, ...: no item places them
        short = name.replace("minecraft:", "")
        if not short.startswith(args.only.replace("minecraft:", "")):
            continue
        if args.items and short not in args.items.split(","):
            continue
        props = set(b.get("properties", {}))
        items.append((short, bool(props & oriented_props)))
    items.sort()
    if args.limit:
        items = items[: args.limit]

    host, port = args.rcon.rsplit(":", 1)
    r = Rcon(args.rcon, args.password)
    r.cmd("forceload add 0 0 15 15")
    r.cmd("tick freeze")
    r.cmd("gamerule random_tick_speed 0")
    r.cmd("fill 0 96 0 15 98 15 stone")
    bot = Bot(args.interact_bin, args.game)
    r.cmd(f"gamemode creative {BOT}")
    x0, y0, z0, x1, y1, z1 = WINDOW
    support_name = args.support if "[" not in args.support else args.support  # as the chunk reader prints it
    seq = 0
    n_cases = 0
    t0 = time.time()
    with open(args.out, "w") as f:
        f.write("# recorded from the official server by scripts/record-placement.py\n")
        f.write(f"# target {args.support} at {TARGET}; the player stands at {BOT_POS}; cells are relative to the target\n")
        f.write(f"# support {args.support}\n")
        for item, oriented in items:
            ctxs = support_contexts(item) if args.contexts == "support" else contexts(item, args.per_block, oriented)
            for (face, cursor, yaw, pitch) in ctxs:
                r.cmd(f"fill {x0} {y0} {z0} {x1} {y1} {z1} air")
                r.cmd(f"setblock {TARGET[0]} {TARGET[1]} {TARGET[2]} {args.support}")
                bot.teleport(r, f"tp {BOT} {BOT_POS[0]} {BOT_POS[1]} {BOT_POS[2]} {yaw} {pitch}")
                seq += 1
                bot.p.stdin.write(f"place minecraft:{item} {TARGET[0]} {TARGET[1]} {TARGET[2]} {face} {cursor[0]} {cursor[1]} {cursor[2]} {seq}\n")
                bot.p.stdin.flush()
                while bot._read() != f"ack {seq}":
                    pass
                r.cmd("setblock 0 200 0 stone" if seq % 2 else "setblock 0 200 0 air")
                r.cmd("save-all flush")
                got = blocks_in_window(args.world)
                f.write(f"case {item} {face} {cursor[0]} {cursor[1]} {cursor[2]} {yaw} {pitch}\n")
                for (x, y, z), s in sorted(got.items()):
                    d = (x - TARGET[0], y - TARGET[1], z - TARGET[2])
                    if d == (0, 0, 0) and s == support_name:
                        continue
                    f.write(f"b {d[0]} {d[1]} {d[2]} {s}\n")
                f.write("end\n")
                n_cases += 1
            if n_cases and n_cases % 200 == 0:
                print(f"{n_cases} cases, {time.time() - t0:.0f}s, at {item}", file=sys.stderr, flush=True)
    bot.close()
    print(f"recorded {n_cases} cases for {len(items)} blocks")


if __name__ == "__main__":
    main()
