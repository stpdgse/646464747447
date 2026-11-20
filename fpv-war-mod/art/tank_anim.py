#!/usr/bin/env python3
"""Tank animations (authoring convention; lib/anim converts to Bedrock axes). Bind pose is neutral, so values are absolute."""
import math, os, sys
sys.path.insert(0, os.path.dirname(__file__))
from lib.anim import Anim, save
import tank as T

W = [f"wheel_{s}{i}" for s in "lr" for i in range(6)]
S = lambda ph, c=1, o=0.0: math.sin(2 * math.pi * (ph * c + o))

def build(outdir=T.OUT):
    A = []
    a = Anim("idle", 2.0, True)                               # engine rumble, mast sway
    a.sample("chassis", "position", lambda ph: (0, 0.07 * S(ph, 8), 0), 32)
    a.sample("chassis", "rotation", lambda ph: (0.12 * S(ph, 6, .2), 0, 0.1 * S(ph, 5)), 32)
    a.sample("antenna", "rotation", lambda ph: (1.2 * S(ph, 2), 0, 2.0 * S(ph, 1, .3)), 24)
    A.append(a)
    a = Anim("drive", 1.0, True)                              # wheels roll (3 turns a second), hull pitches and bounces on the suspension
    for w in W: a.sample(w, "rotation", lambda ph: (-360.0 * 3 * ph, 0, 0), 12)
    a.sample("chassis", "position", lambda ph: (0, 0.22 * abs(S(ph, 2.5)) + 0.05 * S(ph, 11), 0), 40)
    a.sample("chassis", "rotation", lambda ph: (0.7 * S(ph, 2, .1), 0, 0.45 * S(ph, 3)), 40)
    a.sample("antenna", "rotation", lambda ph: (4 * S(ph, 2, .3), 0, 6 * S(ph, 1, .1)), 24)
    a.sample("turret", "rotation", lambda ph: (0, 0.3 * S(ph, 2), 0), 24)
    A.append(a)
    a = Anim("turret_scan", 8.0, True)                        # the commander sweeps for targets; the MG sweeps on its own
    a.sample("turret", "rotation", lambda ph: (0, 38 * S(ph), 0), 48)
    a.sample("gun", "rotation", lambda ph: (-1.5 + 2.2 * S(ph, 2, .1), 0, 0), 48)
    a.sample("mg", "rotation", lambda ph: (0, 55 * S(ph, 1, .35), 0), 48)
    a.sample("chassis", "position", lambda ph: (0, 0.07 * S(ph, 40), 0), 64)
    A.append(a)
    a = Anim("fire", 0.9, False)                              # recoil: barrel slides back and returns, hull rocks, gun kicks up
    for t, v in [(0, 0), (.04, 7), (.12, 6), (.35, 2), (.6, .4), (.9, 0)]: a.key("barrel", "position", t, (0, 0, v))
    for t, v in [(0, 0), (.04, 1.1), (.2, .45), (.9, 0)]: a.key("chassis", "position", t, (0, -0.15 * v, v))
    for t, v in [(0, 0), (.05, 1.6), (.3, -.5), (.6, .15), (.9, 0)]: a.key("chassis", "rotation", t, (v, 0, 0))
    for t, v in [(0, 0), (.05, 2.4), (.4, .6), (.9, 0)]: a.key("gun", "rotation", t, (v, 0, 0))
    for t, v in [(0, 0), (.05, 1.5), (.3, -.6), (.9, 0)]: a.key("antenna", "rotation", t, (v * 8, 0, v * 3))
    A.append(a)
    a = Anim("destroyed", 1.5, "hold")                        # catastrophic kill: hull sags, turret thrown clear and lands askew, gun droops
    for t, y, z in [(0, 0, 0), (.15, -1.0, 2), (1.5, -1.6, 3.2)]: a.key("chassis", "position", t, (0, y, 0)); a.key("chassis", "rotation", t, (0, 0, z))
    for t, pos, rot in [(0, (0, 0, 0), (0, 0, 0)), (.12, (2, 8, 0), (-6, 8, 10)), (.35, (4, 16, 1), (-10, 15, 17)), (.8, (5, 4, 3), (-8, 20, 21)), (1.5, (5, 1.8, 3), (-6, 22, 23))]:
        a.key("turret", "position", t, pos); a.key("turret", "rotation", t, rot)
    for t, v in [(0, 0), (.5, 8), (1.5, 14)]: a.key("gun", "rotation", t, (v, 0, 0))
    for t, v in [(0, 0), (.5, 20), (1.5, 55)]: a.key("antenna", "rotation", t, (v * .4, 0, v))
    for t, v in [(0, 0), (.6, 25), (1.5, 40)]: a.key("mg", "rotation", t, (v, 0, 0))
    A.append(a)
    os.makedirs(outdir, exist_ok=True)
    save(os.path.join(outdir, "tank.animation.json"), "tank", A)
    print("tank animations:", ", ".join(f"{x.name}({x.length}s)" for x in A))

if __name__ == "__main__":
    build()
