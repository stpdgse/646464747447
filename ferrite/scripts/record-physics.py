#!/usr/bin/env python3
"""Records what the official server does in small block-physics scenarios.

The scenarios run on a real vanilla server (frozen, stepped tick by tick over RCON) and the
resulting blocks and pending fluid ticks are read back from its world save. The recordings are
committed under crates/ferrite-blocks/tests/fixtures and replayed against Ferrite by
`cargo test -p ferrite-blocks`, so the comparison needs no server at test time.

    scripts/record-physics.py --world /path/to/server/world [--rcon 127.0.0.1:25622] \
        [--password ...] [--only NAME_PREFIX]

The script disables random ticks (a game rule) on the server it records from.
Server requirements (server.properties): enable-rcon=true, level-type=minecraft:flat,
pause-when-empty-seconds=-1. Running the server requires accepting Mojang's EULA yourself.

Everything happens in chunk (0, 0) between y=96 and y=111.
"""
import argparse, io, os, random, socket, struct, sys, time, zlib

Y0, Y1 = 96, 111

class Rcon:
    def __init__(self, addr, password):
        host, port = addr.rsplit(":", 1)
        self.s = socket.create_connection((host, int(port)))
        self.i = 0
        self._send(3, password)
        if self._read()[0] == -1:
            sys.exit("RCON password rejected")

    def _send(self, kind, body):
        self.i += 1
        b = body.encode()
        p = struct.pack("<ii", self.i, kind) + b + b"\0\0"
        self.s.sendall(struct.pack("<i", len(p)) + p)

    def _read(self):
        def rd(n):
            d = b""
            while len(d) < n:
                c = self.s.recv(n - len(d))
                if not c:
                    sys.exit("server closed the RCON connection")
                d += c
            return d
        (n,) = struct.unpack("<i", rd(4))
        d = rd(n)
        return struct.unpack("<i", d[:4])[0], d[8:-2].decode()

    def cmd(self, c):
        self._send(2, c)
        return self._read()[1]


def parse_nbt(buf):
    f = io.BytesIO(buf)
    def rd(fmt):
        return struct.unpack(fmt, f.read(struct.calcsize(fmt)))[0]
    def string():
        return f.read(rd(">H")).decode("utf8", "replace")
    def payload(t):
        if t == 1: return rd(">b")
        if t == 2: return rd(">h")
        if t == 3: return rd(">i")
        if t == 4: return rd(">q")
        if t == 5: return rd(">f")
        if t == 6: return rd(">d")
        if t == 7: return f.read(rd(">i"))
        if t == 8: return string()
        if t == 9:
            et, n = rd(">b"), rd(">i")
            return [payload(et) for _ in range(n)]
        if t == 10:
            d = {}
            while True:
                tt = rd(">b")
                if tt == 0:
                    return d
                k = string()
                d[k] = payload(tt)
        if t == 11:
            return [rd(">i") for _ in range(rd(">i"))]
        if t == 12:
            n = rd(">i")
            return list(struct.unpack(">%dQ" % n, f.read(8 * n)))
        raise ValueError(f"bad tag {t}")
    t = rd(">b")
    string()
    return payload(t)


def read_chunk(world, cx, cz):
    name = f"r.{cx >> 5}.{cz >> 5}.mca"
    for sub in ("dimensions/minecraft/overworld/region", "region"):
        path = os.path.join(world, sub, name)
        if os.path.exists(path):
            break
    else:
        sys.exit(f"no region file {name} under {world}")
    with open(path, "rb") as f:
        header = f.read(4096)
        i = 4 * ((cx & 31) + (cz & 31) * 32)
        f.seek(int.from_bytes(header[i:i + 3], "big") * 4096)
        n = int.from_bytes(f.read(4), "big")
        if f.read(1)[0] != 2:
            sys.exit("unexpected chunk compression")
        return parse_nbt(zlib.decompress(f.read(n - 1)))


def palette_name(p):
    if isinstance(p, str):
        return p.replace("minecraft:", "")
    name = (p.get("") or p.get("id") or p.get("Name")).replace("minecraft:", "")
    props = p.get("properties") or p.get("Properties")
    if props:
        name += "[" + ",".join(f"{k}={v}" for k, v in sorted(props.items())) + "]"
    return name


def chunk_blocks(chunk):
    out = {}
    for sec in chunk["sections"]:
        bs = sec.get("block_states")
        y0 = sec["Y"] * 16
        if not bs or y0 + 15 < Y0 or y0 > Y1:
            continue
        names = [palette_name(p) for p in bs["palette"]]
        data = bs.get("data")
        bits = max(4, (len(names) - 1).bit_length())
        per, mask = 64 // bits, (1 << bits) - 1
        for i in range(4096):
            v = (data[i // per] >> ((i % per) * bits)) & mask if data else 0
            y = y0 + (i >> 8)
            if names[v] != "air" and Y0 <= y <= Y1:
                out[(i & 15, y, (i >> 4) & 15)] = names[v]
    return out


def rows(blocks):
    """Run-length encodes blocks along x: `r y z x_first x_last state`."""
    out = []
    for (y, z) in sorted({(y, z) for (_, y, z) in blocks}):
        x = 0
        while x < 16:
            state = blocks.get((x, y, z))
            end = x
            while end + 1 < 16 and blocks.get((end + 1, y, z)) == state:
                end += 1
            if state is not None:
                out.append(f"r {y} {z} {x} {end} {state}")
            x = end + 1
    return out


class Recorder:
    def __init__(self, rcon, world):
        self.r, self.world = rcon, world

    def step(self, n):
        before = self.gametime()
        self.r.cmd(f"tick step {n}")
        deadline = time.time() + 30
        while self.gametime() < before + n:
            if time.time() > deadline:
                sys.exit("the server did not step (is it paused or not frozen?)")
            time.sleep(0.005)

    def gametime(self):
        return int(self.r.cmd("time query gametime").split("is ")[1].split(" ")[0])

    def snapshot(self):
        # An unchanged chunk is not rewritten, which would leave stale tick delays on disk:
        # flip a marker block far above the arena so the chunk is always dirty.
        self.marker = not getattr(self, "marker", False)
        self.r.cmd(f"setblock 0 200 0 {'stone' if self.marker else 'air'}")
        self.r.cmd("save-all flush")
        chunk = read_chunk(self.world, 0, 0)
        blocks = chunk_blocks(chunk)
        ticks = [(t["x"], t["y"], t["z"], t["i"].replace("minecraft:", ""), t["t"], t["p"])
                 for t in chunk["fluid_ticks"] if Y0 <= t["y"] <= Y1]
        block_ticks = [(t["x"], t["y"], t["z"], t["i"].replace("minecraft:", ""), t["t"], t["p"])
                       for t in chunk["block_ticks"] if Y0 <= t["y"] <= Y1]
        return blocks, ticks, block_ticks

    def clear(self):
        for y in range(Y0, Y1 + 1, 4):
            self.r.cmd(f"fill 0 {y} 0 15 {min(y + 3, Y1)} 15 air")
        self.step(130)  # let every stale scheduled tick run out
        blocks, ticks, btk = self.snapshot()
        if blocks or ticks or btk:
            sys.exit(f"arena did not clear: {len(blocks)} blocks, {len(ticks) + len(btk)} ticks")

    def record(self, name, setup, actions, out_dir):
        """setup: commands building inert scenery. actions: ("set", x, y, z, state) | ("step", n)."""
        self.clear()
        for c in setup:
            reply = self.r.cmd(c)
            if "Incorrect" in reply or "Unknown" in reply or "Invalid" in reply:
                sys.exit(f"{name}: command {c!r} failed: {reply}")
        self.step(130)
        blocks, ticks, btk = self.snapshot()
        if ticks or btk:
            sys.exit(f"{name}: scenery left scheduled ticks behind: {(ticks + btk)[:3]}")
        lines = [f"# recorded from the official server by scripts/record-physics.py", "init"]
        lines += rows(blocks)
        lines.append("end")
        initial = blocks
        for a in actions:
            if a[0] == "set":
                _, x, y, z, state = a
                reply = self.r.cmd(f"setblock {x} {y} {z} {state}")
                # "Could not set the block" means it already was that state: a no-op to replay.
                if "Changed" not in reply and "Could not set" not in reply:
                    sys.exit(f"{name}: setblock failed: {reply}")
                lines.append(f"set {x} {y} {z} {state}")
            else:
                self.step(a[1])
                lines.append(f"step {a[1]}")
                blocks, ticks, btk = self.snapshot()
                # Only what differs from the initial scenery is written down.
                lines.append("expect")
                changed = {p: s for p, s in blocks.items() if initial.get(p) != s}
                changed.update({p: "air" for p in initial if p not in blocks})
                lines += [f"b {x} {y} {z} {s}" for (x, y, z), s in sorted(changed.items())]
                lines += [f"f {x} {y} {z} {i} {t} {p}" for (x, y, z, i, t, p) in ticks]
                lines += [f"k {x} {y} {z} {i} {t} {p}" for (x, y, z, i, t, p) in btk]
                lines.append("end")
        with open(os.path.join(out_dir, name + ".txt"), "w") as f:
            f.write("\n".join(lines) + "\n")
        print(f"recorded {name}")


ARENA = ["fill 0 96 0 15 108 15 stone hollow", "fill 1 108 1 14 108 14 air",
         "fill 1 97 1 14 99 14 stone"]  # walls, open top, floor surface at y=99

def steps(*ns):
    return [("step", n) for n in ns]

def scenarios():
    s = {}
    s["water_flat"] = (ARENA, [("set", 8, 100, 8, "water")] + steps(1, 4, 1, 4, 5, 5, 10, 30))
    s["water_removed"] = (ARENA, [("set", 8, 100, 8, "water")] + steps(22)
                          + [("set", 8, 100, 8, "air")] + steps(3, 5, 5, 5, 20, 40))
    s["water_two_sources_make_a_third"] = (ARENA, [("set", 6, 100, 8, "water"), ("set", 8, 100, 8, "water")]
                                           + steps(5, 5, 5, 40))
    s["water_falls_into_pit"] = (ARENA + ["fill 4 97 4 6 99 6 air"],
                                 [("set", 9, 100, 5, "water")] + steps(5, 5, 5, 5, 5, 5, 30, 60))
    s["water_prefers_the_nearest_drop"] = (ARENA + ["setblock 12 99 8 air", "setblock 8 99 3 air"],
                                           [("set", 8, 100, 8, "water")] + steps(5, 5, 5, 5, 5, 40))
    s["water_from_a_height"] = (ARENA + ["fill 7 100 7 9 103 9 stone"],
                                [("set", 8, 104, 8, "water")] + steps(5, 5, 5, 5, 5, 5, 5, 5, 60))
    s["water_and_slabs"] = (ARENA + ["fill 5 100 5 11 100 5 stone_slab", "fill 5 100 11 11 100 11 stone_slab[type=top]",
                                     "fill 5 100 6 5 100 10 oak_stairs[facing=east]", "fill 11 100 6 11 100 10 glass_pane"],
                            [("set", 8, 100, 8, "water")] + steps(5, 5, 5, 5, 30))
    s["water_washes_things_away"] = (ARENA + ["setblock 8 100 6 torch", "setblock 6 100 8 snow",
                                              "setblock 10 99 8 grass_block", "setblock 10 100 8 short_grass",
                                              "setblock 8 100 11 stone", "setblock 8 100 10 ladder[facing=north]", "setblock 9 100 9 oak_sign", "setblock 7 100 7 cobweb"],
                                     [("set", 8, 100, 8, "water")] + steps(5, 5, 5, 30))
    s["waterlogged_slab_source"] = (ARENA, [("set", 8, 100, 8, "stone_slab[waterlogged=true]")] + steps(5, 5, 5, 30)
                                    + [("set", 8, 100, 9, "stone"), ("set", 8, 100, 9, "air")] + steps(5, 5, 5, 30))
    s["waterlogged_stairs_direct_flow"] = (ARENA, [("set", 8, 100, 8, "oak_stairs[facing=north,waterlogged=true]"),
                                                   ("set", 9, 100, 8, "stone"), ("set", 9, 100, 8, "air")] + steps(5, 5, 5, 30))
    # A carpet seals the top of the pit under it, so the pit does not attract the flow until the
    # carpet has been washed away.
    s["water_ignores_a_covered_pit"] = (ARENA + ["setblock 10 99 8 air", "setblock 10 100 8 white_carpet"],
                                        [("set", 8, 100, 8, "water")] + steps(5, 5, 5, 5, 30))
    # A source over a hole with three source neighbours also spreads sideways.
    s["water_three_sources_spread_sideways_over_a_hole"] = (
        ARENA + ["fill 7 100 7 9 100 9 stone", "setblock 8 100 8 air", "setblock 8 99 8 air", "setblock 8 98 8 air"],
        [("set", 8, 101, 7, "water"), ("set", 8, 101, 9, "water"), ("set", 7, 101, 8, "water"),
         ("set", 8, 101, 8, "water")] + steps(5, 5, 5, 5, 30))
    s["water_falling_column_spreads_at_the_bottom"] = (ARENA, [("set", 8, 106, 8, "water")] + steps(5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 40))
    # A complete door: placing an upper half that disagrees with the lower half makes it adopt the
    # lower half's state; breaking the floor takes the whole door with it.
    s["door_halves_adapt_to_each_other"] = (
        ARENA + ["setblock 11 100 8 oak_door[half=lower]", "setblock 11 101 8 oak_door[half=upper]"],
        [("set", 8, 100, 8, "oak_door[half=lower,open=true,hinge=left,facing=east]"),
         ("set", 8, 101, 8, "oak_door[half=upper,open=false,hinge=right,facing=north]"),
         ("set", 11, 99, 8, "air"), ("set", 8, 100, 6, "water")] + steps(5, 5, 5, 20))
    # Lava. Scenarios avoid lava levels *rising*, the one place vanilla consults its shared random
    # source (a rising level is delayed at random), which no recording could pin down.
    s["lava_flat"] = (ARENA, [("set", 8, 100, 8, "lava")] + steps(1, 29, 30, 30, 30, 60))
    s["lava_removed"] = (ARENA, [("set", 8, 100, 8, "lava")] + steps(95)
                         + [("set", 8, 100, 8, "air")] + steps(30, 30, 30, 30, 60))
    s["lava_falls_into_pit"] = (ARENA + ["fill 4 97 4 6 99 6 air"],
                                [("set", 8, 100, 5, "lava")] + steps(30, 30, 30, 30, 30, 30, 120))
    # Lava looks only two blocks ahead for a drop: the pit two away attracts it, the one three
    # away does not.
    s["lava_prefers_a_drop_two_away"] = (ARENA + ["setblock 10 99 8 air"],
                                         [("set", 8, 100, 8, "lava")] + steps(30, 30, 30, 60))
    s["lava_ignores_a_drop_three_away"] = (ARENA + ["setblock 11 99 8 air"],
                                           [("set", 8, 100, 8, "lava")] + steps(30, 30, 30, 60))
    s["lava_source_beside_water_source"] = (ARENA, [("set", 8, 100, 8, "lava"), ("set", 9, 100, 8, "water")]
                                            + steps(1, 4, 5, 30, 60))
    s["water_reaches_lava_source"] = (ARENA, [("set", 8, 100, 8, "lava"), ("set", 12, 100, 8, "water")]
                                      + steps(5, 5, 5, 5, 5, 5, 30, 60))
    s["water_reaches_flowing_lava"] = (ARENA, [("set", 4, 100, 8, "lava")] + steps(70)
                                       + [("set", 10, 100, 8, "water")] + steps(5, 5, 5, 5, 5, 5, 30, 60, 120))
    s["lava_falls_onto_water"] = (ARENA + ["fill 6 100 6 10 100 10 stone", "fill 7 100 7 9 100 9 air"],
                                  [("set", 7, 100, 7, "water"), ("set", 9, 100, 9, "water"), ("set", 7, 100, 9, "water"),
                                   ("set", 9, 100, 7, "water")] + steps(20)
                                  + [("set", 8, 103, 8, "lava")] + steps(30, 30, 30, 30, 60))
    s["water_falls_onto_lava"] = (ARENA + ["fill 6 100 6 10 100 10 stone", "fill 7 100 7 9 100 9 lava"],
                                  [("set", 8, 103, 8, "water")] + steps(5, 5, 5, 5, 5, 5, 30))
    s["lava_on_soul_soil_beside_blue_ice"] = (ARENA + ["fill 5 99 5 11 99 11 soul_soil", "setblock 11 100 8 blue_ice"],
                                              [("set", 8, 100, 8, "lava")] + steps(30, 30, 30, 30, 60))
    return s


def redstone_scenarios():
    """Redstone components driven one tick at a time. Levers and buttons are placed with
    /setblock, which notifies only the six neighbours, so power sources that matter are
    redstone blocks and torches."""
    s = {}
    one = steps(1)
    s["rs_block_powers_a_lamp"] = (ARENA + ["setblock 8 100 8 redstone_lamp"],
        [("set", 9, 100, 8, "redstone_block")] + steps(1, 1, 1)
        + [("set", 9, 100, 8, "air")] + steps(1, 1, 1, 1, 1, 1, 1))
    s["rs_block_powers_a_lamp_through_stone"] = (ARENA + ["setblock 8 100 8 redstone_lamp", "setblock 9 100 8 stone"],
        [("set", 10, 100, 8, "redstone_block")] + steps(1, 1, 1))
    s["rs_lever_beside_a_lamp"] = (ARENA + ["setblock 8 100 8 redstone_lamp"],
        [("set", 9, 100, 8, "lever[face=wall,facing=east,powered=true]")] + steps(1, 1)
        + [("set", 9, 100, 8, "lever[face=wall,facing=east,powered=false]")] + steps(1, 1, 1, 1, 1, 1))
    s["rs_lever_powers_the_block_it_is_on"] = (ARENA + ["setblock 8 100 8 stone", "setblock 8 101 8 redstone_lamp",
                                                       "setblock 9 100 8 redstone_lamp", "setblock 8 99 8 stone"],
        [("set", 8, 100, 7, "lever[face=wall,facing=north,powered=true]")] + steps(1, 1, 5))
    # A torch is switched off by a *powered block it is attached to*: here a redstone block
    # under it (a redstone block is a weak source, so every direction counts for a torch).
    s["rs_torch_off_when_its_block_is_powered"] = (ARENA + ["setblock 8 100 8 stone", "setblock 8 101 8 redstone_torch[lit=true]"],
        [("set", 8, 100, 8, "redstone_block")] + steps(1, 1, 1, 1, 1)
        + [("set", 8, 100, 8, "stone")] + steps(1, 1, 1, 1, 1))
    s["rs_wall_torch_off_when_its_block_is_powered"] = (ARENA + ["setblock 8 100 8 stone", "setblock 9 100 8 redstone_wall_torch[facing=east,lit=true]",
                                                                 "setblock 10 100 8 redstone_lamp", "setblock 9 101 8 redstone_lamp"],
        [("set", 8, 100, 8, "redstone_block")] + steps(1, 1, 1, 1, 1, 1)
        + [("set", 8, 100, 8, "stone")] + steps(1, 1, 1, 1, 1, 1))
    s["rs_floor_torch_powers_the_block_above"] = (ARENA + ["setblock 8 101 8 stone", "setblock 8 102 8 redstone_lamp",
                                                           "setblock 7 101 8 redstone_lamp", "setblock 8 100 8 stone"],
        [("set", 8, 100, 8, "redstone_torch[lit=true]")] + steps(1, 1, 1, 1)
        + [("set", 8, 100, 8, "air")] + steps(1, 1, 1, 1, 1, 1, 1))
    s["rs_torch_does_not_power_what_it_stands_on"] = (ARENA + ["setblock 8 100 8 redstone_lamp", "setblock 9 101 8 redstone_lamp",
                                                               "setblock 9 100 8 stone", "setblock 7 100 8 stone", "setblock 7 101 8 redstone_lamp"],
        [("set", 8, 101, 8, "redstone_torch[lit=true]")] + steps(1, 1, 1, 1, 1))
    s["rs_wall_torch_does_not_power_its_wall"] = (ARENA + ["setblock 8 100 8 redstone_lamp", "setblock 8 101 8 redstone_lamp",
                                                           "setblock 7 100 8 redstone_lamp"],
        [("set", 9, 100, 8, "redstone_wall_torch[facing=east,lit=true]")] + steps(1, 1, 1, 1, 1))
    s["rs_torch_is_lit_when_placed_unpowered"] = (ARENA + ["setblock 8 100 8 stone", "setblock 8 102 8 redstone_lamp"],
        [("set", 8, 101, 8, "redstone_torch[lit=false]")] + steps(1, 1, 1, 1, 1))
    s["rs_torch_burns_out"] = (ARENA + ["setblock 8 100 8 stone", "setblock 8 101 8 redstone_torch[lit=true]"],
        sum([[("set", 8, 100, 8, "redstone_block")] + steps(1, 1, 1) + [("set", 8, 100, 8, "stone")] + steps(1, 1, 1)
             for _ in range(10)], []) + steps(100, 100))
    s["rs_repeater_delay"] = {}
    for d in (1, 2, 3, 4):
        s[f"rs_repeater_delay_{d}"] = (ARENA + [f"setblock 9 100 8 repeater[facing=west,delay={d}]", "setblock 10 100 8 redstone_lamp"],
            [("set", 8, 100, 8, "redstone_block")] + steps(*[1] * (2 * d + 3))
            + [("set", 8, 100, 8, "air")] + steps(*[1] * (2 * d + 3)))
    del s["rs_repeater_delay"]
    s["rs_repeater_short_pulse_is_extended"] = (ARENA + ["setblock 9 100 8 repeater[facing=west,delay=2]", "setblock 10 100 8 redstone_lamp"],
        [("set", 8, 100, 8, "redstone_block")] + steps(1) + [("set", 8, 100, 8, "air")] + steps(*[1] * 12))
    s["rs_repeater_chain"] = (ARENA + ["setblock 9 100 8 repeater[facing=west,delay=1]", "setblock 10 100 8 repeater[facing=west,delay=1]",
                                       "setblock 11 100 8 repeater[facing=west,delay=1]", "setblock 12 100 8 redstone_lamp"],
        [("set", 8, 100, 8, "redstone_block")] + steps(*[1] * 10) + [("set", 8, 100, 8, "air")] + steps(*[1] * 10))
    s["rs_repeater_locked_by_a_side_repeater"] = (ARENA + ["setblock 9 100 8 repeater[facing=west,delay=1]", "setblock 8 100 8 redstone_lamp"],
        [("set", 9, 100, 9, "repeater[facing=south,delay=1,powered=true]"), ("set", 10, 100, 8, "redstone_block"),
         ("set", 10, 100, 8, "air")] + steps(*[1] * 3) + [("set", 11, 100, 8, "redstone_block")] + steps(*[1] * 3)
        + [("set", 9, 100, 9, "air")] + steps(*[1] * 6))
    s["rs_repeater_lock_holds_the_output"] = (ARENA + ["setblock 9 100 8 repeater[facing=west,delay=1,powered=true]", "setblock 8 100 8 redstone_lamp[lit=true]",
                                                       "setblock 10 100 8 redstone_block"],
        [("set", 9, 100, 7, "repeater[facing=north,delay=1,powered=true]"), ("set", 10, 100, 8, "air")] + steps(*[1] * 4)
        + [("set", 9, 100, 7, "air")] + steps(*[1] * 6))
    W = "redstone_wire"
    line = lambda x0, x1, y=100, z=8: [("set", x, y, z, W) for x in range(x0, x1 + 1)]
    s["rs_wire_line_decays"] = (ARENA, line(2, 12) + [("set", 1, 100, 8, "redstone_block")] + steps(1)
                                + [("set", 1, 100, 8, "air")] + steps(1))
    s["rs_wire_placed_in_other_order"] = (ARENA, [("set", 1, 100, 8, "redstone_block")]
                                          + [("set", x, 100, 8, W) for x in (6, 4, 5, 2, 3, 9, 8, 7)] + steps(1))
    s["rs_wire_long_line"] = (ARENA, [("set", 1, 100, 8, "redstone_block")] + line(2, 14) + steps(1)
                              + [("set", 1, 100, 8, "air")] + steps(1))
    s["rs_wire_corner_and_t_and_cross"] = (ARENA, [("set", 8, 100, 8, "redstone_block")]
        + [("set", x, 100, 8, W) for x in (9, 10)] + [("set", 10, 100, z, W) for z in (9, 10, 7, 6)]
        + [("set", 11, 100, 8, W), ("set", 10, 100, 8, "air"), ("set", 10, 100, 8, W)] + steps(1))
    s["rs_wire_dot_and_neighbours"] = (ARENA, [("set", 8, 100, 8, W), ("set", 8, 100, 9, W), ("set", 9, 100, 8, W),
                                               ("set", 8, 100, 9, "air"), ("set", 9, 100, 8, "air")] + steps(1))
    s["rs_wire_climbs_stone"] = (ARENA + ["setblock 5 100 8 stone", "setblock 6 101 8 stone", "setblock 8 100 8 stone"],
        [("set", 4, 100, 8, "redstone_block")] + [("set", 4 + i, 100 + i, 8, W) for i in (1, 2)]
        + [("set", 7, 102, 8, W), ("set", 3, 100, 8, W)] + steps(1))
    s["rs_wire_climb_and_descend"] = (ARENA + ["setblock 8 100 8 stone", "setblock 9 100 8 stone", "setblock 10 100 8 stone"],
        [("set", 6, 100, 8, "redstone_block"), ("set", 7, 100, 8, W), ("set", 8, 101, 8, W), ("set", 9, 101, 8, W),
         ("set", 10, 101, 8, W), ("set", 11, 100, 8, W), ("set", 12, 100, 8, W)] + steps(1)
        + [("set", 6, 100, 8, "air")] + steps(1))
    s["rs_wire_blocked_by_a_conductor_above"] = (ARENA + ["setblock 8 100 8 stone", "setblock 7 101 8 stone"],
        [("set", 6, 100, 8, "redstone_block"), ("set", 7, 100, 8, W), ("set", 8, 101, 8, W), ("set", 9, 100, 8, W)] + steps(1))
    s["rs_wire_to_repeater_to_wire"] = (ARENA + ["setblock 9 100 8 repeater[facing=west,delay=1]"],
        [("set", 7, 100, 8, "redstone_block"), ("set", 8, 100, 8, W), ("set", 10, 100, 8, W), ("set", 11, 100, 8, W)]
        + steps(1, 1, 1, 1) + [("set", 7, 100, 8, "air")] + steps(1, 1, 1, 1))
    s["rs_wire_only_connects_to_repeaters_ends"] = (ARENA + ["setblock 9 100 8 repeater[facing=west,delay=1]", "setblock 9 100 7 repeater[facing=north]"],
        [("set", 8, 100, 8, W), ("set", 9, 100, 9, W), ("set", 10, 100, 8, W), ("set", 8, 100, 7, W)] + steps(1))
    s["rs_wire_feeds_a_lamp"] = (ARENA + ["setblock 8 100 8 redstone_lamp"],
        [("set", 4, 100, 8, "redstone_block")] + line(5, 7) + steps(1, 1, 1, 1, 1, 1)
        + [("set", 4, 100, 8, "air")] + steps(1, 1, 1, 1, 1, 1))
    s["rs_wire_powers_a_block_it_points_at"] = (ARENA + ["setblock 8 100 8 stone", "setblock 9 100 8 redstone_lamp", "setblock 8 101 8 redstone_lamp"],
        [("set", 5, 100, 8, "redstone_block")] + line(6, 7) + steps(1, 1, 1))
    s["rs_wire_strong_power_through_a_block"] = (ARENA + ["setblock 8 100 8 stone", "setblock 8 99 8 redstone_torch[lit=true]"],
        [("set", 9, 100, 8, W), ("set", 8, 101, 8, W)] + steps(1, 1, 1))
    s["rs_wire_two_sources"] = (ARENA, [("set", 2, 100, 8, "redstone_block"), ("set", 14, 100, 8, "redstone_block")] + line(3, 13)
                                + [("set", 2, 100, 8, "air")] + steps(1) + [("set", 14, 100, 8, "air")] + steps(1))
    s["rs_wire_loop"] = (ARENA, [("set", 8, 100, 8, "redstone_block")]
        + [("set", x, 100, z, W) for (x, z) in [(9, 8), (10, 8), (10, 9), (10, 10), (9, 10), (8, 10), (8, 9), (7, 9), (7, 8)]] + steps(1)
        + [("set", 8, 100, 8, "air")] + steps(1))
    s["rs_wire_broken_by_support_removal"] = (ARENA + ["setblock 8 100 8 stone"], [("set", 8, 101, 8, W), ("set", 9, 101, 8, "redstone_block"),
        ("set", 8, 100, 8, "air")] + steps(1))
    s["rs_wire_powers_a_repeater_input"] = (ARENA + ["setblock 8 100 8 repeater[facing=east,delay=2]", "setblock 9 100 8 redstone_lamp"],
        [("set", 5, 100, 8, "redstone_block"), ("set", 6, 100, 8, W), ("set", 7, 100, 8, W)] + steps(*[1] * 6)
        + [("set", 5, 100, 8, "air")] + steps(*[1] * 6))
    s["rs_torch_pulse_into_wire"] = (ARENA + ["setblock 8 100 8 stone", "setblock 8 101 8 redstone_torch[lit=true]",
                                              "setblock 9 101 8 redstone_wire", "setblock 10 101 8 redstone_lamp", "setblock 9 100 8 stone"],
        [("set", 8, 100, 8, "redstone_block")] + steps(1, 1, 1, 1, 1) + [("set", 8, 100, 8, "stone")] + steps(1, 1, 1, 1, 1))
    return s


def random_redstone(count, seed):
    parts = ["stone", "stone", "redstone_wire", "redstone_wire", "redstone_wire", "redstone_wire", "redstone_lamp",
             "repeater[facing=north,delay=1]", "repeater[facing=east,delay=2]", "repeater[facing=south,delay=1]",
             "repeater[facing=west,delay=3]", "redstone_torch[lit=true]", "redstone_wall_torch[facing=east,lit=true]",
             "redstone_wall_torch[facing=north,lit=true]", "glass", "lever[face=floor,facing=north,powered=false]"]
    out = {}
    for n in range(count):
        rng = random.Random(seed * 7919 + n)
        setup = list(ARENA)
        actions = []
        cells = [(rng.randint(3, 12), rng.randint(100, 101), rng.randint(3, 12)) for _ in range(rng.randint(8, 30))]
        for x, y, z in cells:
            if y == 101 and not any(c == (x, 100, z) for c in cells):
                continue
            actions.append(("set", x, y, z, rng.choice(parts)))
        for _ in range(rng.randint(1, 3)):
            x, y, z = rng.randint(2, 13), 100, rng.randint(2, 13)
            actions.append(("set", x, y, z, "redstone_block"))
            actions += steps(*[rng.choice([1, 1, 2, 3]) for _ in range(rng.randint(1, 4))])
        if rng.random() < 0.6:
            actions.append(("set", *[next(a for a in actions if a[0] == "set" and a[4] == "redstone_block")[i] for i in (1, 2, 3)], "air"))
            actions += steps(*[rng.choice([1, 2, 4]) for _ in range(4)])
        actions += steps(20)
        out[f"rs_random_{n:03}"] = (setup, actions)
    return out


def random_circuits(count, seed):
    """A random walk away from a source: wire and repeaters oriented along the walk, with the
    occasional lamp tap, stone step and torch inverter. The source is placed last and toggled."""
    DIRS = {"north": (0, -1), "south": (0, 1), "west": (-1, 0), "east": (1, 0)}
    OPP = {"north": "south", "south": "north", "west": "east", "east": "west"}
    out = {}
    for n in range(count):
        rng = random.Random(seed * 104729 + n)
        x, z = rng.randint(4, 11), rng.randint(4, 11)
        sx, sz = x, z
        actions, taken = [], {(x, z)}
        d = rng.choice(list(DIRS))
        for _ in range(rng.randint(6, 22)):
            if rng.random() < 0.3:
                d = rng.choice([k for k in DIRS if k != OPP[d]])
            nx, nz = x + DIRS[d][0], z + DIRS[d][1]
            if not (1 <= nx <= 14 and 1 <= nz <= 14) or (nx, nz) in taken:
                break
            x, z = nx, nz
            taken.add((x, z))
            roll = rng.random()
            if roll < 0.6:
                actions.append(("set", x, 100, z, "redstone_wire"))
            elif roll < 0.85:  # input comes from the side we walked in from
                actions.append(("set", x, 100, z, f"repeater[facing={OPP[d]},delay={rng.randint(1, 4)}]"))
            elif roll < 0.92:
                actions.append(("set", x, 100, z, "redstone_lamp"))
                break
            else:
                actions.append(("set", x, 100, z, "stone"))
                actions.append(("set", x, 101, z, rng.choice(["redstone_torch[lit=true]", "redstone_wire"])))
            if rng.random() < 0.15:  # a lamp tap beside the path
                tx, tz = x + DIRS[d][1], z + DIRS[d][0]
                if 1 <= tx <= 14 and 1 <= tz <= 14 and (tx, tz) not in taken:
                    taken.add((tx, tz))
                    actions.append(("set", tx, 100, tz, "redstone_lamp"))
        actions.append(("set", sx, 100, sz, "redstone_block"))
        actions += steps(*[rng.choice([1, 1, 2, 3]) for _ in range(6)])
        actions.append(("set", sx, 100, sz, "air"))
        actions += steps(*[rng.choice([1, 1, 2, 3]) for _ in range(6)])
        actions += steps(30)
        out[f"rs_circuit_{n:03}"] = ([*ARENA], actions)
    return out


def random_scenarios(count, seed):
    scenery = ["stone", "stone", "stone", "stone_slab", "stone_slab[type=top]", "oak_stairs[facing=east]",
               "oak_stairs[facing=north,half=top]", "glass_pane", "oak_fence", "cobblestone_wall", "torch",
               "oak_trapdoor[half=top]", "oak_trapdoor[open=true]", "ladder[facing=north]", "iron_bars",
               "stone_slab[waterlogged=true]", "oak_leaves[persistent=true]", "glass", "sand_stone" if False else "sandstone",
               "snow[layers=3]", "oak_pressure_plate", "white_carpet", "chest", "end_rod", "lantern", "scaffolding",
               "oak_fence_gate[open=true]", "oak_door[half=lower]", "cobweb"]
    out = {}
    for n in range(count):
        rng = random.Random(seed * 1000 + n)
        setup = list(ARENA)
        for _ in range(rng.randint(0, 5)):  # pits and trenches
            x, z = rng.randint(1, 13), rng.randint(1, 13)
            setup.append(f"fill {x} {rng.randint(97, 99)} {z} {min(14, x + rng.randint(0, 3))} 99 {min(14, z + rng.randint(0, 3))} air")
        for _ in range(rng.randint(0, 4)):  # raised ground
            x, z = rng.randint(1, 13), rng.randint(1, 13)
            setup.append(f"fill {x} 100 {z} {min(14, x + rng.randint(0, 3))} {rng.randint(100, 101)} {min(14, z + rng.randint(0, 3))} stone")
        for _ in range(rng.randint(0, 40)):
            setup.append(f"setblock {rng.randint(1, 14)} {rng.randint(100, 102)} {rng.randint(1, 14)} {rng.choice(scenery)}")
        actions = []
        for _ in range(rng.randint(1, 3)):
            actions.append(("set", rng.randint(1, 14), rng.randint(100, 104), rng.randint(1, 14), "water"))
            actions += steps(*[rng.choice([1, 3, 5, 5, 7, 10]) for _ in range(rng.randint(1, 5))])
        if rng.random() < 0.5:  # remove a source again
            x, y, z = next((a[1], a[2], a[3]) for a in actions if a[0] == "set")
            actions.append(("set", x, y, z, "air"))
            actions += steps(5, 5, 10, 20)
        actions += steps(40, 80)
        out[f"water_random_{n:03}"] = (setup, actions)
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--world", required=True)
    ap.add_argument("--rcon", default="127.0.0.1:25622")
    ap.add_argument("--password", default=os.environ.get("FERRITE_ORACLE_RCON_PASSWORD", "ferrite-oracle"))
    ap.add_argument("--only", default="")
    ap.add_argument("--random", type=int, default=40)
    ap.add_argument("--random-redstone", type=int, default=30)
    ap.add_argument("--random-circuits", type=int, default=30)
    ap.add_argument("--seed", type=int, default=1)
    ap.add_argument("--out", default=os.path.join(os.path.dirname(__file__), "..", "crates", "ferrite-blocks", "tests", "fixtures"))
    args = ap.parse_args()

    r = Rcon(args.rcon, args.password)
    r.cmd("forceload add 0 0 31 31")
    r.cmd("tick freeze")
    r.cmd("tick rate 1000")
    # Random ticks (melting, growth, fire) are random by design; recordings must not contain them.
    r.cmd("gamerule random_tick_speed 0")
    if "set to 0" not in r.cmd("gamerule random_tick_speed"):
        sys.exit("could not disable random ticks")
    rec = Recorder(r, args.world)
    todo = scenarios()
    todo.update(redstone_scenarios())
    todo.update(random_redstone(args.random_redstone, args.seed))
    todo.update(random_circuits(args.random_circuits, args.seed))
    todo.update(random_scenarios(args.random, args.seed))
    for name, (setup, actions) in todo.items():
        if name.startswith(args.only):
            rec.record(name, setup, actions, args.out)


if __name__ == "__main__":
    main()
