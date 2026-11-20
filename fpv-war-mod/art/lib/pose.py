"""Forward/inverse kinematics for box-model bones, matching Blockbench/GeckoLib conventions (verified with bb-probe.js).

Convention: bone rotation in degrees [x, y, z]; R = Rz * Ry * Rx (X applied first); positive rotation is right-handed about the axis.
A point q authored in model coordinates and attached to bone b is moved by: q' = pivot + R (q - pivot) + offset, applied leaf bone first,
then up through the parents.
"""
import math

# ---- 3x3 matrices / vectors as plain lists -------------------------------------------------------------
def rx(a):
    c, s = math.cos(math.radians(a)), math.sin(math.radians(a)); return [[1, 0, 0], [0, c, -s], [0, s, c]]
def ry(a):
    c, s = math.cos(math.radians(a)), math.sin(math.radians(a)); return [[c, 0, s], [0, 1, 0], [-s, 0, c]]
def rz(a):
    c, s = math.cos(math.radians(a)), math.sin(math.radians(a)); return [[c, -s, 0], [s, c, 0], [0, 0, 1]]
def mm(a, b): return [[sum(a[i][k] * b[k][j] for k in range(3)) for j in range(3)] for i in range(3)]
def mv(a, v): return [sum(a[i][k] * v[k] for k in range(3)) for i in range(3)]
def tr(a): return [[a[j][i] for j in range(3)] for i in range(3)]
def add(a, b): return [a[i] + b[i] for i in range(3)]
def sub(a, b): return [a[i] - b[i] for i in range(3)]
def scale(a, k): return [x * k for x in a]
def dot(a, b): return sum(a[i] * b[i] for i in range(3))
def cross(a, b): return [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
def norm(a): return math.sqrt(dot(a, a))
def unit(a):
    n = norm(a); return [x / n for x in a] if n > 1e-12 else [0, 0, 0]

def euler_to_matrix(e): return mm(rz(e[2]), mm(ry(e[1]), rx(e[0])))

def matrix_to_euler(R):
    """Inverse of euler_to_matrix (degrees), in the principal range."""
    sy = -R[2][0]
    sy = max(-1.0, min(1.0, sy))
    y = math.asin(sy)
    if abs(sy) < 0.99999:
        x = math.atan2(R[2][1], R[2][2]); z = math.atan2(R[1][0], R[0][0])
    else:                                              # gimbal lock
        x = math.atan2(-R[1][2], R[1][1]); z = 0.0
    return [math.degrees(x), math.degrees(y), math.degrees(z)]

def rotation_between(a, b):
    """Smallest rotation matrix taking unit vector a to unit vector b."""
    a, b = unit(a), unit(b)
    v = cross(a, b); c = dot(a, b); s = norm(v)
    if s < 1e-9:
        if c > 0: return [[1, 0, 0], [0, 1, 0], [0, 0, 1]]
        # opposite: rotate 180 degrees about any perpendicular axis
        ax = unit(cross(a, [1, 0, 0]) if abs(a[0]) < 0.9 else cross(a, [0, 1, 0]))
        return axis_angle(ax, 180)
    return axis_angle(unit(v), math.degrees(math.atan2(s, c)))

def axis_angle(ax, deg):
    x, y, z = ax; c, s = math.cos(math.radians(deg)), math.sin(math.radians(deg)); t = 1 - c
    return [[t * x * x + c, t * x * y - s * z, t * x * z + s * y], [t * x * y + s * z, t * y * y + c, t * y * z - s * x], [t * x * z - s * y, t * y * z + s * x, t * z * z + c]]


class Rig:
    """A bone tree with rest pivots. pose = {bone: {'rot': [x,y,z], 'pos': [x,y,z]}} (animation deltas added to rest)."""
    def __init__(self, bones, rest_rot=None):
        self.bones = bones                      # {name: (pivot, parent_name_or_None)}
        self.rest_rot = rest_rot or {}

    def chain(self, bone):
        out = []
        while bone: out.append(bone); bone = self.bones[bone][1]
        return out                              # leaf first

    def rot(self, bone, pose):
        r = list(self.rest_rot.get(bone, [0, 0, 0]))
        d = pose.get(bone, {}).get("rot")
        if d: r = [r[i] + d[i] for i in range(3)]
        return r

    def point(self, bone, q, pose):
        """World position of model-space point q attached to `bone`."""
        for b in self.chain(bone):
            p = self.bones[b][0]
            q = add(add(p, mv(euler_to_matrix(self.rot(b, pose)), sub(q, p))), pose.get(b, {}).get("pos", [0, 0, 0]))
        return q

    def frame(self, bone, pose):
        """Rotation matrix (world) of a bone."""
        R = [[1, 0, 0], [0, 1, 0], [0, 0, 1]]
        for b in reversed(self.chain(bone)): R = mm(R, euler_to_matrix(self.rot(b, pose)))
        return R

    def to_local(self, bone, w, pose):
        """Inverse of `point` for the chain above `bone` (i.e. express world point w in `bone`'s authored frame)."""
        for b in reversed(self.chain(bone)):
            p = self.bones[b][0]
            w = add(p, mv(tr(euler_to_matrix(self.rot(b, pose))), sub(sub(w, pose.get(b, {}).get("pos", [0, 0, 0])), p)))
        return w


def two_bone_ik(shoulder, target, l1, l2, pole):
    """Elbow position for a 2-bone chain reaching target (clamped to reach). pole = preferred elbow direction (not necessarily perpendicular)."""
    d = sub(target, shoulder); dist = norm(d)
    dist = max(abs(l1 - l2) + 1e-3, min(l1 + l2 - 1e-3, dist))
    dirn = unit(d)
    a = (l1 * l1 - l2 * l2 + dist * dist) / (2 * dist)      # distance along the line to the elbow's foot
    h = math.sqrt(max(0.0, l1 * l1 - a * a))
    pole_perp = sub(pole, scale(dirn, dot(pole, dirn)))
    if norm(pole_perp) < 1e-6: pole_perp = cross(dirn, [1, 0, 0])
    return add(add(shoulder, scale(dirn, a)), scale(unit(pole_perp), h))
