"""Build the browser resource archive from the game's client jar (local use only; never committed).

Keeps everything except Java classes and jar signing metadata: assets, data packs, pack.mcmeta,
version.json and the like. The result is what `Class.getResourceAsStream` and the "jar" file
system read from in the browser.

usage: make_archive.py CLIENT_JAR OUT_ZIP [ASSET_INDEX_JSON]

The optional asset index (the launcher's indexes/<id>.json) is stored as launcher-assets/indexes/34.json.
"""
import sys
import zipfile

src, dst = sys.argv[1], sys.argv[2]
index = sys.argv[3] if len(sys.argv) > 3 else None
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
print('kept %d entries (%.1f MB uncompressed), skipped %d' % (kept, size / 1048576, skipped))
