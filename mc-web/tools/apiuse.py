"""List the members of given JDK classes that jars actually reference.

usage: apiuse.py CLASS[,CLASS...] JAR [JAR...]   (a trailing * makes a name a prefix)
Parses the constant pool of every .class in the jars (no JVM needed).
"""
import struct
import sys
import zipfile
from collections import defaultdict


def refs(data):
    pos = 10
    n = struct.unpack('>H', data[8:10])[0]
    cp = [None] * n
    i = 1
    while i < n:
        t = data[pos]
        if t == 1:
            ln = struct.unpack('>H', data[pos + 1:pos + 3])[0]
            cp[i] = ('u', data[pos + 3:pos + 3 + ln].decode('utf8', 'replace'))
            pos += 3 + ln
        elif t in (3, 4):
            pos += 5
        elif t in (5, 6):
            pos += 9
            i += 1
        elif t in (7, 8, 16, 19, 20):
            cp[i] = ('r', struct.unpack('>H', data[pos + 1:pos + 3])[0])
            pos += 3
        elif t in (9, 10, 11, 12, 17, 18):
            a, b = struct.unpack('>HH', data[pos + 1:pos + 5])
            cp[i] = ('p', t, a, b)
            pos += 5
        elif t == 15:
            pos += 4
        else:
            raise ValueError(t)
        i += 1
    def utf(k):
        return cp[k][1]
    out = []
    for e in cp:
        if e and e[0] == 'p' and e[1] in (9, 10, 11):
            cls = cp[e[2]]
            nt = cp[e[3]]
            cname = utf(cls[1]).replace('/', '.')
            out.append((cname, utf(cp[nt[2]][1]) if False else utf(cp[e[3]][2] if False else 0) if False else None, None))
    res = []
    for e in cp:
        if e and e[0] == 'p' and e[1] in (9, 10, 11):
            cname = utf(cp[e[2]][1]).replace('/', '.')
            nt = cp[e[3]]
            res.append((cname, utf(nt[2]), utf(nt[3])))
    # NameAndType is stored as ('p',12,nameIdx,descIdx)
    return res, None


def main():
    wanted = set(sys.argv[1].split(','))
    prefixes = tuple(w[:-1] for w in wanted if w.endswith('*'))
    use = defaultdict(set)
    for jar in sys.argv[2:]:
        try:
            z = zipfile.ZipFile(jar)
        except Exception:
            continue
        for nm in z.namelist():
            if not nm.endswith('.class'):
                continue
            try:
                r, _ = refs(z.read(nm))
            except Exception:
                continue
            for c, m, d in r:
                if c in wanted or (prefixes and c.startswith(prefixes)):
                    use[c].add((m, d))
    for c in sorted(use):
        print('##', c)
        for m, d in sorted(use[c]):
            print('  ', m, d)


main()
