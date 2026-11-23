"""Show which functions a wasm function calls (to find why something is compiled as a coroutine).
usage: wasmcalls.py FILE.wasm FUNCTION_NAME [N]"""
import collections
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


def main():
    b = open(sys.argv[1], 'rb').read()
    want = sys.argv[2]
    top = int(sys.argv[3]) if len(sys.argv) > 3 else 25
    p = 8
    sizes = {}
    names = {}
    nimp = 0
    while p < len(b):
        sid = b[p]
        ln, q = leb(b, p + 1)
        end = q + ln
        if sid == 2:
            cnt, r = leb(b, q)
            for _ in range(cnt):
                l1, r = leb(b, r); mod = b[r:r + l1].decode(); r += l1
                l2, r = leb(b, r); nm = b[r:r + l2].decode(); r += l2
                k = b[r]; r += 1
                if k == 0:
                    _, r = leb(b, r); names[nimp] = mod + '.' + nm; nimp += 1
                elif k in (1, 2):
                    if k == 1:
                        r += 1
                    fl = b[r]; r += 1; _, r = leb(b, r)
                    if fl & 1: _, r = leb(b, r)
                elif k == 3:
                    r += 2
        if sid == 10:
            cnt, r = leb(b, q)
            for i in range(cnt):
                sz, r2 = leb(b, r); sizes[i + nimp] = (r2, sz); r = r2 + sz
        if sid == 0:
            nl, r = leb(b, q); nm = b[r:r + nl].decode(); r += nl
            if nm == 'name':
                while r < end:
                    sub = b[r]; sl, r2 = leb(b, r + 1)
                    if sub == 1:
                        c, t = leb(b, r2)
                        for _ in range(c):
                            idx, t = leb(b, t); l, t = leb(b, t)
                            names.setdefault(idx, b[t:t + l].decode('utf8', 'replace')); t += l
                    r = r2 + sl
        p = end
    tgt = [i for i, n in names.items() if n == want]
    if not tgt:
        print('not found:', want)
        return
    off, sz = sizes[tgt[0]]
    body = b[off:off + sz]
    c = collections.Counter()
    i = 0
    while i < len(body) - 1:
        if body[i] == 0x10:
            idx, j = leb(body, i + 1)
            if idx in names and j - i <= 6:
                c[names[idx]] += 1
                i = j
                continue
        i += 1
    print('%s: %d bytes' % (want, sz))
    for n, k in c.most_common(top):
        print('%5d %s' % (k, n))


main()
