import socket, struct, zlib, io, os, time, sys
class Rcon:
    def __init__(s, port=25622, pw="ferrite-oracle"):
        s.s = socket.create_connection(("127.0.0.1", port)); s.i = 0
        s._send(3, pw); s._read()
    def _send(s, kind, body):
        s.i += 1; b = body.encode()
        p = struct.pack("<ii", s.i, kind) + b + b"\0\0"
        s.s.sendall(struct.pack("<i", len(p)) + p); return s.i
    def _read(s):
        def rd(n):
            d = b""
            while len(d) < n:
                c = s.s.recv(n - len(d)); assert c; d += c
            return d
        n, = struct.unpack("<i", rd(4)); d = rd(n)
        return struct.unpack("<ii", d[:8]), d[8:-2].decode()
    def cmd(s, c):
        s._send(2, c); return s._read()[1]

def nbt(buf):
    f = io.BytesIO(buf)
    def rd(fmt):
        n = struct.calcsize(fmt); return struct.unpack(fmt, f.read(n))[0]
    def st(): return f.read(rd(">H")).decode("utf8", "replace")
    def pay(t):
        if t == 1: return rd(">b")
        if t == 2: return rd(">h")
        if t == 3: return rd(">i")
        if t == 4: return rd(">q")
        if t == 5: return rd(">f")
        if t == 6: return rd(">d")
        if t == 7: return f.read(rd(">i"))
        if t == 8: return st()
        if t == 9:
            et = rd(">b"); n = rd(">i"); return [pay(et) for _ in range(n)]
        if t == 10:
            d = {}
            while True:
                tt = rd(">b")
                if tt == 0: return d
                k = st(); d[k] = pay(tt)
        if t == 11: n = rd(">i"); return [rd(">i") for _ in range(n)]
        if t == 12: n = rd(">i"); return list(struct.unpack(">%dQ" % n, f.read(8 * n)))
        raise Exception(t)
    t = rd(">b"); st(); return pay(t)

def read_chunk(world, cx, cz):
    path = f"{world}/region/r.{cx>>5}.{cz>>5}.mca"
    if not os.path.exists(path):
        path = f"{world}/dimensions/minecraft/overworld/region/r.{cx>>5}.{cz>>5}.mca"
    with open(path, "rb") as f:
        hdr = f.read(4096); i = 4 * ((cx & 31) + (cz & 31) * 32)
        off = int.from_bytes(hdr[i:i+3], "big")
        f.seek(off * 4096); n = int.from_bytes(f.read(4), "big"); c = f.read(1)[0]
        return nbt(zlib.decompress(f.read(n - 1)))

def blocks(chunk):
    out = {}
    for sec in chunk["sections"]:
        bs = sec.get("block_states")
        if not bs: continue
        pal = bs["palette"]; data = bs.get("data")
        names = []
        for p in pal:
            if isinstance(p, str): n = p.replace("minecraft:", "")
            else:
                n = (p.get("") or p.get("id") or p.get("Name")).replace("minecraft:", "")
                pr = p.get("properties") or p.get("Properties")
                if pr: n += "[" + ",".join(f"{k}={v}" for k, v in sorted(pr.items())) + "]"
            names.append(n)
        y0 = sec["Y"] * 16
        if not data:
            if names[0] != "air":
                for i in range(4096): out[(i & 15, y0 + (i >> 8), (i >> 4) & 15)] = names[0]
            continue
        bits = max(4, (len(pal) - 1).bit_length()); per = 64 // bits; mask = (1 << bits) - 1
        for i in range(4096):
            v = (data[i // per] >> ((i % per) * bits)) & mask
            if names[v] != "air": out[(i & 15, y0 + (i >> 8), (i >> 4) & 15)] = names[v]
    return out

W = "/opt/mcdata/phys/world"
def reset(r, floor=True):
    r.cmd("fill 0 96 0 15 99 15 air"); r.cmd("fill 0 100 0 15 110 15 air")
    if floor: r.cmd("fill 0 99 0 15 99 15 stone")
def step(r, n):
    out = r.cmd(f"tick step {n}")
    time.sleep(0.06 * n + 0.15)
    while "sprint" in r.cmd("tick query") and False: pass
def snap(r, ymin=96):
    r.cmd("save-all flush")
    b = blocks(read_chunk(W, 0, 0))
    return {k: v for k, v in b.items() if k[1] >= ymin}
def show(b, y, kind="water"):
    for z in range(16):
        row = ""
        for x in range(16):
            v = b.get((x, y, z))
            if v is None: row += " ."
            elif v == kind: row += " 0"
            elif v.startswith(kind + "["):
                row += " " + "0123456789ABCDEF"[int(v.split("level=")[1].rstrip("]"))]
            else: row += " #"
        print(row)

def ticks(kind="fluid_ticks"):
    c = read_chunk(W, 0, 0)
    return [(t["x"], t["y"], t["z"], t["i"].replace("minecraft:", ""), t["t"], t["p"]) for t in c[kind]]
