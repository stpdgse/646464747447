"""Approximate class-level reachability from an entry class, via constant-pool class references.

usage: reach.py JAR ENTRY_CLASS [PREFIX_TO_REPORT ...]
Prints reachable-class counts per package and, for each report prefix, one shortest path of
referencing classes explaining why it is reachable.
"""
import struct
import sys
import zipfile
from collections import deque, Counter


def class_refs(data):
    n = struct.unpack('>H', data[8:10])[0]
    pos = 10
    cp = [None] * n
    i = 1
    while i < n:
        t = data[pos]
        if t == 1:
            ln = struct.unpack('>H', data[pos + 1:pos + 3])[0]
            cp[i] = (1, data[pos + 3:pos + 3 + ln].decode('utf8', 'replace'))
            pos += 3 + ln
        elif t in (3, 4):
            pos += 5
        elif t in (5, 6):
            pos += 9
            i += 1
        elif t == 7:
            cp[i] = (7, struct.unpack('>H', data[pos + 1:pos + 3])[0])
            pos += 3
        elif t in (8, 16, 19, 20):
            pos += 3
        elif t in (9, 10, 11, 12, 17, 18):
            pos += 5
        elif t == 15:
            pos += 4
        else:
            raise ValueError(t)
        i += 1
    out = set()
    for e in cp:
        if e and e[0] == 7:
            nm = cp[e[1]][1]
            while nm.startswith('['):
                nm = nm[1:]
            if nm.startswith('L') and nm.endswith(';'):
                nm = nm[1:-1]
            out.add(nm)
    # descriptors may reference more classes (field/method types); include Utf8 'L...;' tokens
    for e in cp:
        if e and e[0] == 1 and 'L' in e[1] and ';' in e[1] and '/' in e[1]:
            for part in e[1].split('L')[1:]:
                if ';' in part:
                    out.add(part.split(';')[0])
    return out


def main():
    jar, entry = sys.argv[1], sys.argv[2].replace('.', '/')
    reports = [r.replace('.', '/') for r in sys.argv[3:]]
    z = zipfile.ZipFile(jar)
    names = {n[:-6] for n in z.namelist() if n.endswith('.class')}
    parent = {entry: None}
    q = deque([entry])
    while q:
        c = q.popleft()
        if c not in names:
            continue
        try:
            refs = class_refs(z.read(c + '.class'))
        except Exception:
            continue
        for r in refs:
            if r in names and r not in parent:
                parent[r] = c
                q.append(r)
    print('reachable classes in jar:', len(parent), 'of', len(names))
    pk = Counter('/'.join(c.split('/')[:-1]) for c in parent)
    for p, n in pk.most_common(40):
        print('%6d %s' % (n, p.replace('/', '.')))
    for r in reports:
        hits = [c for c in parent if c.startswith(r)]
        print('\n##', r.replace('/', '.'), len(hits), 'classes')
        if hits:
            c = hits[0]
            path = []
            while c:
                path.append(c)
                c = parent[c]
            print('  e.g. ' + ' <- '.join(x.split('/')[-1] for x in path[:12]))


main()
