#!/usr/bin/env python3
"""Derives block-placement rules from a recording made by scripts/record-placement.py.

For every block and every property whose value depends on how the block was placed, it finds a
formula of the click context (clicked face, hit height, look direction) that reproduces what the
official server did in *every* recorded context, and writes them as a Rust table. Properties that
no formula explains (they depend on neighbouring blocks or need several blocks) are listed as
residuals and implemented by hand in `placement.rs`.

    scripts/gen-placement.py recording.txt[:more.txt...] blocks.json crates/ferrite-blocks/src/generated_placement.rs
"""
import collections, json, math, re, struct, sys

DIRS = ["down", "up", "north", "south", "west", "east"]
VEC = {"down": (0, -1, 0), "up": (0, 1, 0), "north": (0, 0, -1), "south": (0, 0, 1), "west": (-1, 0, 0), "east": (1, 0, 0)}
OPP = {"down": "up", "up": "down", "north": "south", "south": "north", "west": "east", "east": "west"}


def horiz(yaw):
    return ["south", "west", "north", "east"][int(math.floor(yaw / 90.0 + 0.5)) & 3]


def f32(x):
    return struct.unpack("f", struct.pack("f", x))[0]


_SIN = [f32(math.sin(i * math.pi * 2.0 / 65536.0)) for i in range(65536)]


def tsin(x):
    return _SIN[int(x * 10430.378350470453) & 65535]


def tcos(x):
    return _SIN[int(x * 10430.378350470453 + 16384.0) & 65535]


def nearest(yaw, pitch):
    """Direction.orderedByNearest, with the game's table sine (65536 float entries, indexed in
    double precision) and float arithmetic -- bit for bit, which matters for exact ties."""
    k180 = f32(f32(math.pi) / 180.0)
    f, g = f32(f32(pitch) * k180), f32(f32(-yaw) * k180)
    h, i, j, k = tsin(f), tcos(f), tsin(g), tcos(g)
    bl, bl2, bl3 = j > 0, h < 0, k > 0
    l, m, n = (j if bl else -j), (-h if bl2 else h), (k if bl3 else -k)
    o, p = f32(l * i), f32(n * i)
    d1, d2, d3 = ("east" if bl else "west"), ("up" if bl2 else "down"), ("south" if bl3 else "north")
    if l > n:
        if m > o:
            return [d2, d1, d3]
        return [d1, d3, d2] if p > m else [d1, d2, d3]
    if m > p:
        return [d2, d3, d1]
    return [d3, d1, d2] if o > m else [d3, d2, d1]


CW = {"north": "east", "east": "south", "south": "west", "west": "north"}
CCW = {v: k for k, v in CW.items()}


def axis_of(d):
    return {"down": "y", "up": "y", "north": "z", "south": "z", "west": "x", "east": "x"}[d]


def features(c):
    f = c["face"]
    st = VEC[f]
    nl = nearest(c["yaw"], c["pitch"])
    h = horiz(c["yaw"])
    return {"F": f, "F_": OPP[f], "H": h, "H_": OPP[h], "N": nl[0], "N_": OPP[nl[0]], "cy": c["cursor"][1] - st[1], "yaw": c["yaw"]}


# name -> (rust variant, function)
FORMULAS = {
    "DirF": lambda ft: ft["F"],
    "DirFOpp": lambda ft: ft["F_"],
    "DirH": lambda ft: ft["H"],
    "DirHOpp": lambda ft: ft["H_"],
    "DirHCw": lambda ft: CW[ft["H"]],
    "DirHCcw": lambda ft: CCW[ft["H"]],
    "DirN": lambda ft: ft["N"],
    "DirNOpp": lambda ft: ft["N_"],
    "AxisF": lambda ft: axis_of(ft["F"]),
    "AxisN": lambda ft: axis_of(ft["N"]),
    "AxisH": lambda ft: axis_of(ft["H"]),
    "HalfSlab": lambda ft: "bottom" if (ft["F"] != "down" and (ft["F"] == "up" or not ft["cy"] > 0.5)) else "top",
    "FaceAttach": lambda ft: {"up": "floor", "down": "ceiling"}.get(ft["F"], "wall"),
    "RailShape": lambda ft: "east_west" if ft["H"] in ("east", "west") else "north_south",
    "Rot16": lambda ft: str(int(math.floor((180 + ft["yaw"]) * 16.0 / 360.0 + 0.5)) & 15),
}


def parse_state(s, blocks):
    m = re.match(r"([a-z_0-9]+)(?:\[(.*)\])?$", s)
    props = dict(kv.split("=") for kv in m.group(2).split(",")) if m.group(2) else {}
    if not props:
        b = blocks["minecraft:" + m.group(1)]
        d = next((st for st in b["states"] if st.get("default")), b["states"][0])
        props = dict(d.get("properties", {}))
    return m.group(1), props


def load(path):
    cases, cur = [], None
    for line in open(path):
        line = line.rstrip("\n")
        if line.startswith("#") or not line:
            continue
        if line.startswith("case "):
            p = line.split(" ")
            cur = {"item": p[1], "face": DIRS[int(p[2])], "cursor": tuple(map(float, p[3:6])), "yaw": float(p[6]), "pitch": float(p[7]), "cells": {}}
        elif line == "end":
            cases.append(cur)
        else:
            p = line.split(" ", 4)
            cur["cells"][(int(p[1]), int(p[2]), int(p[3]))] = p[4]
    return cases


def main():
    cases = [c for path in sys.argv[1].split(":") for c in load(path)]
    blocks = json.load(open(sys.argv[2]))
    groups = collections.defaultdict(list)  # placed block -> [(features, props)]
    for c in cases:
        d = VEC[c["face"]]
        cell = c["cells"].get(d)
        if cell is None:
            continue  # nothing placed
        name, props = parse_state(cell, blocks)
        if name != c["item"]:
            continue  # the item placed a different block (torch -> wall torch): handled by hand
        groups[name].append((features(c), props))
    rules, residual, ambiguous = {}, collections.defaultdict(list), 0
    for name, obs in sorted(groups.items()):
        domain = blocks["minecraft:" + name].get("properties", {})
        default = next((st for st in blocks["minecraft:" + name]["states"] if st.get("default")), blocks["minecraft:" + name]["states"][0]).get("properties", {})
        out = []
        for prop in sorted(domain):
            seen = {o[1][prop] for o in obs}
            if len(seen) == 1:
                v = next(iter(seen))
                if v != default.get(prop):
                    out.append((prop, f'Const("{v}")'))
                continue
            fits = [fn for fn, fx in FORMULAS.items() if all(fx(ft) == props[prop] for ft, props in obs)]
            if fits:
                if len(fits) > 1:
                    ambiguous += 1
                out.append((prop, fits[0]))
            else:
                residual[prop].append(name)
        if out:
            rules[name] = out
    with open(sys.argv[3], "w") as f:
        f.write("// @generated by scripts/gen-placement.py from a recording of the official server. DO NOT EDIT.\n")
        f.write("//\n// Placement rules for properties that depend on the click context. Blocks whose properties\n")
        f.write("// depend on neighbours or that place several blocks are handled in `placement.rs`.\n\n")
        f.write("use crate::placement::Formula::*;\nuse crate::placement::Formula;\n\n")
        f.write(f"/// `(block, [(property, formula)])`, sorted by block name.\npub static RULES: [(&str, &[(&str, Formula)]); {len(rules)}] = [\n")
        for name, out in rules.items():
            f.write(f'    ("{name}", &[' + ", ".join(f'("{p}", {fm})' for p, fm in out) + "]),\n")
        f.write("];\n")
    print(f"{len(rules)} blocks with rules; {ambiguous} ambiguous fits (first used)")
    print("residual properties (not explained by a formula):")
    for p, names in sorted(residual.items(), key=lambda kv: -len(kv[1])):
        print(f"  {p}: {len(names)} blocks, e.g. {sorted(set(names))[:5]}")


main()
