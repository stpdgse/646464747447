"""Rifleman animation set. Every frame is solved with IK (hands stay on the rifle) and exported as a delta from the baked low-ready rest pose."""
import json, math, os, sys
sys.path.insert(0, os.path.dirname(__file__))
from lib.anim import Anim, save, lerp, smooth
import rifleman as R
import rifleman_pose as RP

S = lambda ph, cyc=1, off=0.0: math.sin(2 * math.pi * (ph * cyc + off))
C = lambda ph, cyc=1, off=0.0: math.cos(2 * math.pi * (ph * cyc + off))
LR, AIMP = RP.LOW_READY, RP.AIM
AIM_HEAD = (3, -9, -2)
AIM_LEGS = {"leg_l": (12, 0, 0), "leg_r": (-9, 0, 0), "leg_r_lower": (-4, 0, 0), "leg_l_lower": (-6, 0, 0)}
def vl(a, b, t): return tuple(lerp(a[i], b[i], t) for i in range(len(a)))
def mixd(a, b, t, keys):
    return {k: vl(a[k], b[k], t) for k in keys}
def norm(v):
    n = math.sqrt(sum(x * x for x in v)); return tuple(x / n for x in v)

# ---------------- pose generators: ph (0..1) -> solve kwargs -------------------------------------------------
def idle(ph):
    return dict(LR, torso=(S(ph) * 0.8, S(ph, 1, .25) * 1.2, 0), head=(S(ph, 1, .3) * 1.2, S(ph, 1, 0.1) * 6, 0), body_pos=(0, S(ph, 2) * 0.12, 0),
                legs={"leg_l": (2, 0, 0), "leg_r": (-2, 0, 0)})

def gait(ph, amp, knee, lean, bob, sway, arm_pose=LR, free=False):
    a = 2 * math.pi * ph
    legs = {"leg_r": (amp * math.sin(a), 0, 0), "leg_l": (-amp * math.sin(a), 0, 0),
            "leg_r_lower": (-knee * max(0, math.cos(a)), 0, 0), "leg_l_lower": (-knee * max(0, -math.cos(a)), 0, 0)}
    kw = dict(arm_pose, legs=legs, torso=(lean, -sway * math.sin(a), sway * 0.4 * math.cos(a)), head=(-lean * 0.8, sway * math.sin(a) * 0.8, 0),
              body_pos=(0, -bob * (0.5 + 0.5 * math.cos(2 * a)), 0))
    return kw

def walk(ph): return gait(ph, 28, 42, 3, 0.9, 3.5)
def run(ph):
    kw = gait(ph, 52, 85, 9, 1.8, 6, dict(grip=(2.4, 15.0, -3.2), forward=(-0.2, 0.12, -0.97)))
    return kw

def aim_in(ph):                                   # low-ready -> shouldered (hold)
    t = smooth(min(1, ph))
    kw = dict(butt=vl(RP.AIM["butt"], (RP.AIM["butt"][0], RP.AIM["butt"][1] - 5, RP.AIM["butt"][2] + 1), 1 - t) if False else None)
    # blend by interpolating rifle placement goals: grip from low-ready, butt from aim -> use grip->butt crossfade via intermediate butt goal
    bl = vl((2.0, 17.0, -3.5), RP.AIM["butt"], t)
    fw = norm(vl(LR["forward"], AIMP["forward"], t))
    return dict(butt=bl, forward=fw, head=vl((0, 0, 0), AIM_HEAD, t), legs=mixd({k: (0, 0, 0) for k in AIM_LEGS}, AIM_LEGS, t, AIM_LEGS), torso=(0, 0, 0))

def aim_idle(ph):
    return dict(AIMP, head=AIM_HEAD, legs=AIM_LEGS, torso=(S(ph) * 0.5, 0, 0), body_pos=(0, S(ph, 2) * 0.08, 0),
                forward=norm((AIMP["forward"][0] + S(ph, 1, .1) * .006, AIMP["forward"][1] + S(ph, 1, .35) * .008, AIMP["forward"][2])))

def fire(ph):
    t = ph * 0.25
    kick = (t / 0.03) if t < 0.03 else math.exp(-(t - 0.03) / 0.07)          # sharp rise, fast decay
    f = norm((AIMP["forward"][0] + 0.01 * kick, AIMP["forward"][1] + 0.075 * kick, AIMP["forward"][2]))
    return dict(AIMP, forward=f, recoil=1.7 * kick, head=(AIM_HEAD[0] - 3.5 * kick, AIM_HEAD[1], AIM_HEAD[2]), legs=AIM_LEGS,
                torso=(-3.2 * kick, 1.2 * kick, 0.6 * kick), body_pos=(0, 0, 0.45 * kick))

def torso_point(kw, left):
    """Torso-frame position of the left hand when it targets `left` (used to blend from a rifle-frame target to a torso-frame one)."""
    pose = RP.solve(**dict(kw, left=left, mag=None))
    return tuple(RP.RIG.to_local("torso", RP.RIG.point("rifle", RP.rifle_point(left[1]), pose), pose))

def reload(ph):
    base = dict(LR, head=(10 * smooth(min(1, ph / 0.1)) * (1 if ph < 0.9 else 0), 0, 0))
    T = RP.R
    well_low = ("rifle", (0, -2.5, -2.0)); well_in = ("rifle", (0, -0.5, -2.0))
    dump, pouch = (-5.0, 10.0, -3.5), (-2.5, 15.0, -4.6)
    kw = dict(base, grip=(2.0, 14.6, -2.2), forward=norm((-0.2, 0.18, -0.96)), torso=(4, 6, 0), mag={"scale": [1, 1, 1]})
    seg = lambda a, b: smooth(min(1, max(0, (ph - a) / (b - a))))
    if ph < 0.12:
        kw["left"] = "foregrip"
    elif ph < 0.28:
        kw["left"] = ("rifle", tuple(vl((0, 0.6, -2.5), well_low[1], seg(.12, .28))))
    elif ph < 0.42:                                           # pull the empty magazine down and out
        kw["left"] = ("rifle", tuple(vl(well_low[1], (0, -8.5, -2.0), seg(.28, .42))))
        kw["mag"] = {"follow": (0, -2.5, -2.0), "scale": [1, 1, 1]}
    elif ph < 0.58:                                           # stuff it into the dump pouch (disappears)
        t = seg(.42, .58)
        start = torso_point(kw, ("rifle", (0, -8.5, -2.0)))
        kw["left"] = tuple(vl(start, dump, t))
        kw["mag"] = {"follow": (0, -2.5, -2.0), "scale": [1, 1, 1] if t < 0.85 else [0, 0, 0]}
        if t >= 0.85: kw["mag"] = {"scale": [0, 0, 0], "pos": [0, 0, 0]}
    elif ph < 0.72:
        kw["left"] = tuple(vl(dump, pouch, seg(.58, .72))); kw["mag"] = {"scale": [0, 0, 0], "pos": [0, 0, 0]}
    elif ph < 0.84:                                           # fresh magazine out of the pouch up to the well
        kw["left"] = ("rifle", tuple(vl((-1.5, -9.0, -5.0), (0, -8.5, -2.0), seg(.72, .84))))
        if ph < 0.74: kw["left"] = tuple(vl(pouch, (-2.5, 13.0, -4.2), seg(.72, .74)))
        kw["mag"] = {"follow": (0, -2.5, -2.0), "scale": [1, 1, 1]}
    elif ph < 0.93:                                           # seat it
        kw["left"] = ("rifle", tuple(vl((0, -8.5, -2.0), (0, -2.0, -2.0), seg(.84, .93))))
        kw["mag"] = {"follow": (0, -2.5, -2.0), "scale": [1, 1, 1]}
    else:
        kw["left"] = ("rifle", tuple(vl((0, -2.0, -2.0), (0, 0.6, -2.5), seg(.93, 1.0))))
        kw["mag"] = {"scale": [1, 1, 1], "pos": [0, 0, 0]}
        t = seg(.93, 1.0); kw["grip"] = vl((2.0, 14.6, -2.2), LR["grip"], t); kw["forward"] = norm(vl(kw["forward"], LR["forward"], t)); kw["torso"] = vl((4, 6, 0), (0, 0, 0), t)
    return kw

CROUCH_LEGS = {"leg_r": (68, 0, 0), "leg_l": (62, 0, 0), "leg_r_lower": (-78, 0, 0), "leg_l_lower": (-72, 0, 0)}
def crouch(ph):
    t = smooth(min(1, ph))
    legs = {k: vl((0, 0, 0), v, t) for k, v in CROUCH_LEGS.items()}
    return dict(LR, legs=legs, root_pos=(0, -3.9 * t, 2.8 * t), torso=(18 * t, 0, 0), head=(-14 * t, 0, 0))

def crouch_idle(ph):
    kw = crouch(1.0)
    kw["head"] = (-14 + S(ph, 1, .1) * 2, S(ph) * 10, 0); kw["body_pos"] = (0, S(ph, 2) * 0.1, 0)
    return kw

def panic(ph):
    sh = 1.5 * math.sin(ph * 2 * math.pi * 6)
    legs = dict(CROUCH_LEGS)
    return dict(legs=legs, root_pos=(0, -3.9, 2.8), torso=(26 + sh, 0, 0), head=(-8, sh * 2, 0),
                right_free=((118 + sh, 0, -22), (112, 0, 0), (-70, 0, 0)), left_free=((120 - sh, 0, 22), (118, 0, 0)), body_pos=(0, 0, 0))

def flee(ph):
    a = 2 * math.pi * ph
    legs = {"leg_r": (58 * math.sin(a), 0, 0), "leg_l": (-58 * math.sin(a), 0, 0), "leg_r_lower": (-95 * max(0, math.cos(a)), 0, 0), "leg_l_lower": (-95 * max(0, -math.cos(a)), 0, 0)}
    return dict(legs=legs, torso=(14, -7 * math.sin(a), 0), head=(-6, 0, 0), body_pos=(0, -2.1 * (0.5 + 0.5 * math.cos(2 * a)), 0),
                right_free=((-60 * math.sin(a) + 10, 0, -8), (-40, 0, 0), (-75, 0, 0)), left_free=((60 * math.sin(a) + 10, 0, 8), (-60 + 20 * math.sin(a), 0, 0)))

def flinch(ph):
    k = math.sin(math.pi * min(1, ph)) ** 0.7
    return dict(LR, torso=(-7 * k, 10 * k, 3 * k), head=(-14 * k, -16 * k, 5 * k), body_pos=(0, -0.6 * k, 0.6 * k),
                legs={"leg_r": (-6 * k, 0, 0), "leg_l": (8 * k, 0, 0)})

def death_a(ph):                                             # buckle at the knees, fall onto the back
    e = smooth(ph)
    knee = smooth(min(1, ph / 0.35)); fall = smooth(max(0, (ph - 0.25) / 0.75)) ** 1.4
    legs = {"leg_r": (50 * knee * (1 - fall) + 4 * fall, 0, 0), "leg_l": (38 * knee * (1 - fall) - 6 * fall, 0, 0), "leg_r_lower": (-70 * knee * (1 - fall) - 8 * fall, 0, 0), "leg_l_lower": (-60 * knee * (1 - fall) - 10 * fall, 0, 0)}
    arms_r = ((-30 * fall - 5, 0, -75 * fall), (-20 * fall, 0, 0), (-10 + 60 * fall, 0, 0))
    arms_l = ((-40 * fall + 10, 0, 90 * fall), (-25 * fall, 0, 0))
    return dict(legs=legs, root=(-92 * fall, 0, 0), root_pos=(0, -3.0 * knee * (1 - fall) + 3.1 * fall, 3.0 * knee * (1 - fall) + 1.0 * fall), torso=(10 * knee * (1 - fall), 0, 0),
                head=(-14 * fall + 8 * knee * (1 - fall), 8 * fall, 0), right_free=arms_r, left_free=arms_l)

def death_b(ph):                                             # stagger, fall face-down
    knee = smooth(min(1, ph / 0.3)); fall = smooth(max(0, (ph - 0.2) / 0.8)) ** 1.5
    legs = {"leg_r": (-14 * fall + 24 * knee * (1 - fall), 0, 0), "leg_l": (26 * knee * (1 - fall), 0, 0), "leg_r_lower": (-48 * knee * (1 - fall), 0, 0), "leg_l_lower": (-35 * knee * (1 - fall), 0, 0)}
    arms_r = ((95 * fall + 5, 0, -14 * fall), (-10 * fall, 0, 0), (-10 + 30 * fall, 0, 0))
    arms_l = ((100 * fall + 5, 0, 14 * fall), (-12 * fall, 0, 0))
    return dict(legs=legs, root=(90 * fall, 0, 0), root_pos=(0, -2.0 * knee * (1 - fall) + 2.1 * fall, -1.0 * knee * (1 - fall) - 1.0 * fall),
                torso=(-8 * knee * (1 - fall) + 6 * fall, 4 * fall, 0), head=(-22 * fall, -18 * fall, 6 * fall), right_free=arms_r, left_free=arms_l)

# name, generator, length, loop, samples
SET = [("idle", idle, 3.0, True, 12), ("walk", walk, 1.0, True, 16), ("run", run, 0.7, True, 16), ("aim", aim_in, 0.35, "hold", 8),
       ("aim_idle", aim_idle, 3.0, True, 12), ("fire", fire, 0.25, False, 16), ("reload", reload, 2.8, False, 56),
       ("crouch", crouch, 0.4, "hold", 8), ("crouch_idle", crouch_idle, 3.0, True, 12), ("panic", panic, 1.2, True, 24),
       ("flee", flee, 0.6, True, 16), ("flinch", flinch, 0.4, False, 10), ("death_a", death_a, 1.5, "hold", 30), ("death_b", death_b, 1.3, "hold", 26)]

def unwrap(prev, cur):
    return [c + 360.0 * round((p - c) / 360.0) for p, c in zip(prev, cur)]

def frames(gen, n):
    return [RP.solve(**gen(i / n)) for i in range(n + 1)]

def build(REST, outdir, preview=None):
    anims, sheet = [], []
    names = list(R.m.bones)
    for name, gen, length, loop, n in SET:
        an = Anim(name, length, loop)
        seq = frames(gen, n)
        last = {}
        for i, pose in enumerate(seq):
            t = length * i / n
            for b in names:
                v = pose.get(b)
                if v is None: v = {}
                rest = REST.get(b, {})
                if "rot" in v or "rot" in rest or b in pose:
                    rot = [a - r for a, r in zip(v.get("rot", [0, 0, 0]), rest.get("rot", [0, 0, 0]))]
                    if b in last: rot = unwrap(last[b], rot)
                    last[b] = rot
                    if any(abs(x) > 1e-3 for x in rot) or b in ("torso", "body", "root", "head") or i == 0:
                        an.key(b, "rotation", t, rot)
                pos = [a - r for a, r in zip(v.get("pos", [0, 0, 0]), rest.get("pos", [0, 0, 0]))]
                if any(abs(x) > 1e-3 for x in pos): an.key(b, "position", t, pos)
                if "scale" in v: an.key(b, "scale", t, v["scale"])
        anims.append(an)
        if preview is not None and name in preview:
            for ph in preview[name]:
                p = RP.solve(**gen(ph))
                sheet.append({"name": f"{name}_{int(ph*100):03d}", "rot": {b: v["rot"] for b, v in p.items() if "rot" in v},
                              "pos": {b: v["pos"] for b, v in p.items() if "pos" in v}, "scale": {b: v["scale"] for b, v in p.items() if "scale" in v}})
    os.makedirs(outdir, exist_ok=True)
    save(os.path.join(outdir, "rifleman.animation.json"), "rifleman", anims)
    if preview is not None: json.dump(sheet, open(os.path.join(outdir, "sheet.json"), "w"))
    print("animations:", ", ".join(f"{a.name}({a.length}s)" for a in anims))
