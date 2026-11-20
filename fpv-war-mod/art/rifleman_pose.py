"""Pose solver for the rifleman: a rifle held by the right hand, the left hand glued to the foregrip by IK."""
import math, os, sys
sys.path.insert(0, os.path.dirname(__file__))
from lib.pose import *
import rifleman as R

M = R.m
BONES = {n: (b.pivot, b.parent.name if b.parent else None) for n, b in M.bones.items()}
RIG = Rig(BONES)
G = R.G
SH_R, EL_R, HAND = (6, 22, 0), (6, 16, 0), (6, 9, 0)
SH_L, EL_L, HAND_L = (-6, 22, 0), (-6, 16, 0), (-6, 9, 0)
L1, L2 = 6.0, 7.0                       # shoulder-elbow, elbow-hand-centre

def rifle_point(local):                  # a rifle landmark in the bind (authored) frame
    return [G[0] + local[0], G[1] + local[1], G[2] + local[2]]

def frame_from_forward(f, up=(0, 1, 0), roll=0.0):
    """Rifle rotation matrix: bind forward is -z, up is +y. f = desired forward direction (torso frame)."""
    f = unit(f)
    right = unit(cross(f, up)); yv = cross(right, f)
    Rm = [[right[0], yv[0], -f[0]], [right[1], yv[1], -f[1]], [right[2], yv[2], -f[2]]]   # columns: right, up, back
    return mm(Rm, rz(roll)) if roll else Rm

def solve_arm(pose, side, target_world, pole):
    """Set arm_<side> and arm_<side>_lower rotations so the hand centre reaches target_world. Returns the hand's achieved world position."""
    sh0, el0, hd0 = (SH_R, EL_R, HAND) if side == "r" else (SH_L, EL_L, HAND_L)
    S = RIG.point("torso", sh0, pose)
    E = two_bone_ik(S, target_world, L1, L2, pole)
    s_l = RIG.to_local("torso", S, pose); e_l = RIG.to_local("torso", E, pose); t_l = RIG.to_local("torso", target_world, pose)
    Ru = rotation_between([0, -1, 0], sub(e_l, s_l))
    Rt = rotation_between([0, -1, 0], sub(t_l, e_l))
    pose[f"arm_{side}"] = {"rot": matrix_to_euler(Ru)}
    pose[f"arm_{side}_lower"] = {"rot": matrix_to_euler(mm(tr(Ru), Rt))}

def solve(torso=(0, 0, 0), torso_pos=(0, 0, 0), body=(0, 0, 0), body_pos=(0, 0, 0), root=(0, 0, 0), root_pos=(0, 0, 0), head=(0, 0, 0),
          legs=None, grip=None, butt=None, forward=(0, 0, -1), up=(0, 1, 0), roll=0.0, recoil=0.0, left="foregrip",
          right_free=None, left_free=None, rifle_local=None, mag=None):
    """Return {bone: {'rot': [...], 'pos': [...]}} (absolute; bind pose = zeros).

    Rifle placement: either `grip` (torso-frame point for the right hand) or `butt` (torso-frame point for the shoulder pad), with `forward`/`up`/`roll`.
    left: 'foregrip' | torso-frame point | None (use left_free = (arm_l_euler, arm_l_lower_euler)).
    right_free: (arm_r_euler, arm_r_lower_euler, rifle_euler) to hang the arm limply instead of holding the rifle in a posed way."""
    pose = {"root": {"rot": list(root), "pos": list(root_pos)}, "body": {"rot": list(body), "pos": list(body_pos)},
            "torso": {"rot": list(torso), "pos": list(torso_pos)}, "head": {"rot": list(head)}}
    for k, v in (legs or {}).items(): pose[k] = {"rot": list(v)}
    to_world = lambda pt: RIG.point("torso", pt, pose)                          # torso-frame point -> world
    if right_free is not None:
        pose["arm_r"] = {"rot": list(right_free[0])}; pose["arm_r_lower"] = {"rot": list(right_free[1])}
        pose["rifle"] = {"rot": list(right_free[2])}
    else:
        Rr = frame_from_forward(forward, up, roll)                                # desired rifle orientation, torso frame
        f = unit(forward)
        if butt is not None:
            bl = [R.BUTT[i] for i in range(3)]
            g = sub(butt, mv(Rr, bl))                                             # grip = butt - R * (butt local)
        else:
            g = list(grip)
        g = add(g, scale(f, -recoil))                                             # recoil slides the rifle back along its axis
        solve_arm(pose, "r", to_world(g), [0.5, -1, 0.35])
        Rarm = mm(euler_to_matrix(pose["arm_r"]["rot"]), euler_to_matrix(pose["arm_r_lower"]["rot"]))
        pose["rifle"] = {"rot": matrix_to_euler(mm(tr(Rarm), Rr))}
    if left_free is not None:
        pose["arm_l"] = {"rot": list(left_free[0])}; pose["arm_l_lower"] = {"rot": list(left_free[1])}
    else:
        if left == "foregrip": world = RIG.point("rifle", rifle_point(R.FOREGRIP), pose)
        elif left[0] == "rifle": world = RIG.point("rifle", rifle_point(left[1]), pose)      # ("rifle", local landmark)
        else: world = to_world(left)
        solve_arm(pose, "l", world, [-0.55, -1, 0.3])
    if mag is not None:
        mg = dict(mag)
        if mg.get("follow") is not None and left_free is None:             # mag rides in the left hand: offset so the hand holds it at `follow`
            hand = RIG.point("rifle", rifle_point(left[1]), pose) if (left != "foregrip" and left[0] == "rifle") else (
                RIG.point("rifle", rifle_point(R.FOREGRIP), pose) if left == "foregrip" else to_world(left))
            loc = RIG.to_local("rifle", hand, pose)
            home = rifle_point(mg["follow"])
            mg["pos"] = [loc[i] - home[i] for i in range(3)]
        mg.pop("follow", None)
        pose["mag"] = mg
    return pose

# ---- reference poses -------------------------------------------------------------------------------------
LOW_READY = dict(grip=(1.9, 15.8, -2.6), forward=(-0.22, -0.24, -0.95))
AIM = dict(butt=(3.4, 20.8, -2.8), forward=(-0.18, 0.0, -0.98))
