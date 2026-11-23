"""Build the browser resource archive from the game's client jar (local use only; never committed).

Keeps everything except Java classes and jar signing metadata: assets, data packs, pack.mcmeta,
version.json and the like. The result is what `Class.getResourceAsStream` and the "jar" file
system read from in the browser.

usage: make_archive.py CLIENT_JAR OUT_ZIP [ASSET_INDEX_JSON [ASSETS_DIR [ICU4J_JAR]]]

The optional asset index (the launcher's indexes/<id>.json) is stored as launcher-assets/indexes/34.json and
the optional ICU4J_JAR contributes the top-level ICU data files (bidi, normalization, root locale) that its
classes load as resources; the optional ASSETS_DIR (see fetch_assets.py) contributes its objects/<xx>/<hash> files as launcher-assets/objects/....
"""
import os
import sys
import zipfile

src, dst = sys.argv[1], sys.argv[2]
index = sys.argv[3] if len(sys.argv) > 3 else None
assets = sys.argv[4] if len(sys.argv) > 4 else None
icu = sys.argv[5] if len(sys.argv) > 5 else None
kept = skipped = size = 0
with zipfile.ZipFile(src) as zin, zipfile.ZipFile(dst, 'w', zipfile.ZIP_DEFLATED, compresslevel=6) as zout:
    for info in zin.infolist():
        name = info.filename
        top = name.split('/')[0]
        if name.endswith('.class') or name.startswith('META-INF/') or (name.endswith('/') and top not in ('assets', 'data')):
            skipped += 1
            continue
        zout.writestr(info, zin.read(name))
        kept += 1
        size += info.file_size
    if index:
        zout.write(index, 'launcher-assets/indexes/34.json')
        kept += 1
    if assets:
        for root, _, files in os.walk(os.path.join(assets, 'objects')):
            for f in files:
                if f.endswith('.part'):
                    continue
                full = os.path.join(root, f)
                zout.write(full, 'launcher-assets/objects/%s/%s' % (f[:2], f))
                kept += 1
                size += os.path.getsize(full)
if icu:
    prefix = 'com/ibm/icu/impl/data/icudata/'
    with zipfile.ZipFile(icu) as zicu, zipfile.ZipFile(dst, 'a', zipfile.ZIP_DEFLATED, compresslevel=6) as zout:
        for info in zicu.infolist():
            rest = info.filename[len(prefix):]
            if info.filename.startswith(prefix) and rest and '/' not in rest:
                zout.writestr(info, zicu.read(info.filename))
                kept += 1
                size += info.file_size
print('kept %d entries (%.1f MB uncompressed), skipped %d' % (kept, size / 1048576, skipped))
