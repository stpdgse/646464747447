"""Procedural texture shaders. A shader is f(face, px, py, w, h, ctx) -> (r, g, b, a); px,py are local face pixels."""
import math

def _hash(ix, iy, seed):
    n = (ix * 374761393 + iy * 668265263 + seed * 1442695040888963407) & 0xFFFFFFFFFFFFFFFF
    n = (n ^ (n >> 13)) * 1274126177 & 0xFFFFFFFFFFFFFFFF
    return ((n ^ (n >> 16)) & 0xFFFF) / 65535.0

def vnoise(x, y, seed=0):
    """Smooth value noise in [0, 1]."""
    ix, iy = math.floor(x), math.floor(y)
    fx, fy = x - ix, y - iy
    fx = fx * fx * (3 - 2 * fx); fy = fy * fy * (3 - 2 * fy)
    a, b = _hash(ix, iy, seed), _hash(ix + 1, iy, seed)
    c, d = _hash(ix, iy + 1, seed), _hash(ix + 1, iy + 1, seed)
    return (a * (1 - fx) + b * fx) * (1 - fy) + (c * (1 - fx) + d * fx) * fy

def clamp(v): return max(0, min(255, int(round(v))))

def shade(rgb, k):
    return (clamp(rgb[0] * k), clamp(rgb[1] * k), clamp(rgb[2] * k))

def mix(a, b, t):
    return tuple(clamp(a[i] * (1 - t) + b[i] * t) for i in range(3))

FACE_LIGHT = {"up": 1.14, "down": 0.72, "north": 1.0, "south": 0.92, "east": 0.88, "west": 0.96}

def _finish(rgb, face, px, py, w, h, edge=0.82, alpha=255):
    k = FACE_LIGHT.get(face, 1.0)
    if px == 0 or py == 0 or px == w - 1 or py == h - 1: k *= edge          # seam darkening gives panels and volume
    r, g, b = shade(rgb, k)
    return (r, g, b, alpha)

def solid(color, grain=0.05, scale=1.7, seed=0):
    def f(face, px, py, w, h, ctx):
        n = vnoise(px / scale + ctx["ox"], py / scale + ctx["oy"], seed) - 0.5
        return _finish(shade(color, 1 + n * grain * 2), face, px, py, w, h)
    return f

def camo(base, spots, scale=3.2, seed=1, grain=0.06, threshold=(0.56, 0.7)):
    """Blotchy military camouflage: base colour with soft spots of other colours."""
    def f(face, px, py, w, h, ctx):
        x, y = px + ctx["ox"], py + ctx["oy"]
        col = base
        for i, sp in enumerate(spots):
            n = vnoise(x / scale + i * 17.3, y / scale + i * 9.1, seed + i * 31)
            if n > threshold[min(i, len(threshold) - 1)]: col = sp
        n2 = vnoise(x * 1.3, y * 1.3, seed + 99) - 0.5
        return _finish(shade(col, 1 + n2 * grain * 2), face, px, py, w, h)
    return f

def metal(color, grain=0.08, seed=3, brushed=True):
    def f(face, px, py, w, h, ctx):
        x, y = px + ctx["ox"], py + ctx["oy"]
        n = vnoise(x * (0.25 if brushed else 1.0), y * 1.6, seed) - 0.5
        return _finish(shade(color, 1 + n * grain * 2), face, px, py, w, h, edge=0.7)
    return f

def with_decals(base, decals):
    """decals: {face: g(px, py, w, h) -> (r,g,b[,a]) or None}; drawn over the base shader."""
    def f(face, px, py, w, h, ctx):
        out = base(face, px, py, w, h, ctx)
        g = decals.get(face)
        if g:
            d = g(px, py, w, h)
            if d is not None:
                a = d[3] if len(d) > 3 else 255
                t = a / 255.0
                return (clamp(out[0] * (1 - t) + d[0] * t), clamp(out[1] * (1 - t) + d[1] * t), clamp(out[2] * (1 - t) + d[2] * t), out[3])
        return out
    return f

def rect_decal(rects):
    """rects: [(x0, y0, x1, y1, colour)] in local face pixels (inclusive)."""
    def g(px, py, w, h):
        for x0, y0, x1, y1, col in rects:
            if x0 <= px <= x1 and y0 <= py <= y1: return col
        return None
    return g

def weathering(base, dirt=(70, 58, 44), amount=0.35, seed=11):
    """Mud and dust that collects toward the bottom of a face."""
    def f(face, px, py, w, h, ctx):
        out = base(face, px, py, w, h, ctx)
        n = vnoise((px + ctx["ox"]) / 2.2, (py + ctx["oy"]) / 2.2, seed)
        grad = (py / max(1, h - 1)) if face not in ("up", "down") else 0.3
        t = max(0, min(1, (n - 0.45) * 2.2 * (0.35 + grad) * amount * 2))
        return (*mix(out[:3], dirt, t), out[3])
    return f
