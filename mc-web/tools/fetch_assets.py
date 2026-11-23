#!/usr/bin/env python3
"""Download the asset-index objects the game reads from /launcher-assets (textures, fonts, resource packs ...).

usage: fetch_assets.py INDEX_JSON OUT_DIR [--sounds] [--lang]
Sounds and non-English language files are large and are skipped unless asked for. Objects are stored as
OUT_DIR/objects/<first two hex digits>/<hash>, the layout the game expects. Everything stays under work/ (never committed).
"""
import json, os, sys, urllib.request
from concurrent.futures import ThreadPoolExecutor

index_path, out = sys.argv[1], sys.argv[2]
want_sounds = '--sounds' in sys.argv
want_lang = '--lang' in sys.argv
objects = json.load(open(index_path))['objects']

def wanted(name):
    if name.startswith('minecraft/sounds/'):
        return want_sounds
    if name.startswith('minecraft/lang/'):
        return want_lang
    if name.startswith('realms/'):
        return False
    return True

todo = [(n, o['hash'], o['size']) for n, o in objects.items() if wanted(n)]

def fetch(item):
    name, h, size = item
    path = os.path.join(out, 'objects', h[:2], h)
    if os.path.exists(path) and os.path.getsize(path) == size:
        return 0
    os.makedirs(os.path.dirname(path), exist_ok=True)
    tmp = path + '.part'
    for attempt in range(4):
        try:
            with urllib.request.urlopen('https://resources.download.minecraft.net/%s/%s' % (h[:2], h), timeout=60) as r, open(tmp, 'wb') as f:
                f.write(r.read())
            os.replace(tmp, path)
            return size
        except Exception as e:
            err = e
    print('FAILED', name, err, file=sys.stderr)
    return 0

with ThreadPoolExecutor(8) as pool:
    total = sum(pool.map(fetch, todo))
print('fetched %.1f MB for %d objects' % (total / 1e6, len(todo)))
