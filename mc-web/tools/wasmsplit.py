"""Split an oversized WebAssembly start function into several smaller functions.

V8 refuses to compile any single function larger than 7,654,321 bytes. TeaVM emits one start
function (`teavm@initializer`) that initialises every class; for the full game it is ~8.6 MB.
That function is straight-line code (no locals, no branches), so it can be cut after any complete
`struct.set` / `global.set` statement and re-emitted as a sequence of calls.

usage: wasmsplit.py IN.wasm OUT.wasm [CHUNK_BYTES]
"""
import sys


def leb(b, p):
    r = s = 0
    while True:
        x = b[p]
        p += 1
        r |= (x & 0x7f) << s
        s += 7
        if not x & 0x80:
            return r, p


def enc(n):
    out = bytearray()
    while True:
        x = n & 0x7f
        n >>= 7
        if n:
            out.append(x | 0x80)
        else:
            out.append(x)
            return bytes(out)


def sleb(b, p):
    r = s = 0
    while True:
        x = b[p]
        p += 1
        r |= (x & 0x7f) << s
        s += 7
        if not x & 0x80:
            return r, p


def boundaries(body):
    """Offsets (into body) just after each complete top-level store statement."""
    p = 0
    n = len(body)
    out = []
    while p < n:
        op = body[p]
        p += 1
        if op in (0x23, 0x10, 0xD2):
            _, p = leb(body, p)
        elif op == 0x24:
            _, p = leb(body, p)
            out.append(p)
        elif op in (0x41, 0x42, 0xD0):
            _, p = sleb(body, p)
        elif op == 0xFB:
            sub, p = leb(body, p)
            if sub == 5:
                _, p = leb(body, p)
                _, p = leb(body, p)
                out.append(p)
            elif sub == 8:
                _, p = leb(body, p)
                _, p = leb(body, p)
            else:
                raise SystemExit('unexpected GC opcode 0xFB %d in initializer' % sub)
        elif op == 0x6b:
            pass
        else:
            raise SystemExit('unexpected opcode 0x%02x in initializer' % op)
    return out


def main():
    src, dst = sys.argv[1], sys.argv[2]
    chunk = int(sys.argv[3]) if len(sys.argv) > 3 else 1_000_000
    b = open(src, 'rb').read()
    p = 8
    secs = []
    while p < len(b):
        sid = b[p]
        ln, q = leb(b, p + 1)
        secs.append([sid, b[q:q + ln]])
        p = q + ln
    sec = {s[0]: s for s in secs}
    # imports: count function imports
    nimp = 0
    d = sec[2][1]
    cnt, r = leb(d, 0)
    for _ in range(cnt):
        l1, r = leb(d, r); r += l1
        l2, r = leb(d, r); r += l2
        k = d[r]; r += 1
        if k == 0:
            _, r = leb(d, r); nimp += 1
        elif k == 1:
            r += 1; fl = d[r]; r += 1; _, r = leb(d, r)
            if fl & 1: _, r = leb(d, r)
        elif k == 2:
            fl = d[r]; r += 1; _, r = leb(d, r)
            if fl & 1: _, r = leb(d, r)
        elif k == 3:
            r += 2
        else:
            raise SystemExit('unsupported import kind %d' % k)
    start, _ = leb(sec[8][1], 0)
    local = start - nimp
    # function section
    fd = sec[3][1]
    nfun, r = leb(fd, 0)
    types = []
    for _ in range(nfun):
        t, r = leb(fd, r)
        types.append(t)
    # code section
    cd = sec[10][1]
    ncode, r = leb(cd, 0)
    assert ncode == nfun
    entries = []
    for _ in range(ncode):
        sz, r2 = leb(cd, r)
        entries.append(cd[r2:r2 + sz])
        r = r2 + sz
    body = entries[local]
    assert body[0] == 0, 'start function has locals'
    inner = body[1:-1]
    assert body[-1] == 0x0b
    print('start function: index %d, %d bytes' % (start, len(body)))
    if len(body) <= 7_000_000:
        print('already small enough; copying unchanged')
        open(dst, 'wb').write(b)
        return
    cuts = boundaries(inner)
    parts = []
    last = 0
    for c in cuts:
        if c - last >= chunk:
            parts.append(inner[last:c])
            last = c
    parts.append(inner[last:])
    print('splitting into %d parts, largest %d bytes' % (len(parts), max(len(x) for x in parts)))
    base = nimp + nfun
    for i, part in enumerate(parts):
        types.append(types[local])
        entries.append(b'\x00' + part + b'\x0b')
    newbody = b'\x00' + b''.join(b'\x10' + enc(base + i) for i in range(len(parts))) + b'\x0b'
    entries[local] = newbody
    sec[3][1] = enc(len(types)) + b''.join(enc(t) for t in types)
    sec[10][1] = enc(len(entries)) + b''.join(enc(len(e)) + e for e in entries)
    out = bytearray(b[:8])
    for sid, data in secs:
        out += bytes([sid]) + enc(len(data)) + data
    open(dst, 'wb').write(out)
    print('wrote', dst, len(out), 'bytes')


main()
