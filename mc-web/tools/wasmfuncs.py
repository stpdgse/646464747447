"""List the largest functions in a .wasm file (code section sizes + names from the name section).
usage: wasmfuncs.py FILE.wasm [N]"""
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
    n = int(sys.argv[2]) if len(sys.argv) > 2 else 15
    p = 8
    sizes = []
    names = {}
    nimports = 0
    while p < len(b):
        sid = b[p]
        ln, q = leb(b, p + 1)
        end = q + ln
        if sid == 2:
            cnt, r = leb(b, q)
            for _ in range(cnt):
                l1, r = leb(b, r); r += l1
                l2, r = leb(b, r); r += l2
                k = b[r]; r += 1
                if k == 0:
                    _, r = leb(b, r); nimports += 1
                elif k == 1:
                    r += 1; fl = b[r]; r += 1; _, r = leb(b, r)
                    if fl & 1: _, r = leb(b, r)
                elif k == 2:
                    fl = b[r]; r += 1; _, r = leb(b, r)
                    if fl & 1: _, r = leb(b, r)
                elif k == 3:
                    r += 2
                else:
                    break
        if sid == 10:
            cnt, r = leb(b, q)
            for i in range(cnt):
                sz, r2 = leb(b, r)
                sizes.append((sz, i + nimports, r))
                r = r2 + sz
        if sid == 0:
            nl, r = leb(b, q)
            nm = b[r:r + nl].decode()
            r += nl
            if nm == 'name':
                while r < end:
                    sub = b[r]
                    sl, r2 = leb(b, r + 1)
                    if sub == 1:
                        c, t = leb(b, r2)
                        for _ in range(c):
                            idx, t = leb(b, t)
                            l, t = leb(b, t)
                            names[idx] = b[t:t + l].decode('utf8', 'replace')
                            t += l
                    r = r2 + sl
        p = end
    sizes.sort(reverse=True)
    print('functions:', len(sizes), 'imports:', nimports, 'file:', len(b))
    for sz, idx, off in sizes[:n]:
        print('%9d  idx=%d off=%d %s' % (sz, idx, off, names.get(idx, '?')))


main()
