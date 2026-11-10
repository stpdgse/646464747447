#!/usr/bin/env bash
# Downloads the official Minecraft server jar for a release and verifies it against the SHA-1
# published in Mojang's version manifest.
#
#   scripts/fetch-server-jar.sh 26.3 /path/to/dir     # writes /path/to/dir/server-26.3.jar
#
# The jar is Mojang's property: it is used locally as a data source and is never committed.
set -euo pipefail

version="${1:?usage: fetch-server-jar.sh <version> <output-dir>}"
outdir="${2:?usage: fetch-server-jar.sh <version> <output-dir>}"
manifest_url="https://launchermeta.mojang.com/mc/game/version_manifest_v2.json"

mkdir -p "$outdir"
read -r jar_url expected_sha1 < <(
  python3 -I - "$version" "$manifest_url" <<'PY'
import json, sys, urllib.request
version, manifest_url = sys.argv[1], sys.argv[2]
manifest = json.load(urllib.request.urlopen(manifest_url, timeout=60))
entry = next((v for v in manifest["versions"] if v["id"] == version), None)
if entry is None:
    sys.exit(f"version {version} not found in the Mojang manifest")
meta = json.load(urllib.request.urlopen(entry["url"], timeout=60))
server = meta["downloads"]["server"]
print(server["url"], server["sha1"])
PY
)

dest="$outdir/server-$version.jar"
curl -fsSL --retry 4 --retry-delay 2 -o "$dest" "$jar_url"
actual_sha1="$(sha1sum "$dest" | cut -d' ' -f1)"
if [[ "$actual_sha1" != "$expected_sha1" ]]; then
  echo "SHA-1 mismatch for $dest: expected $expected_sha1, got $actual_sha1" >&2
  rm -f "$dest"
  exit 1
fi
echo "$dest ($actual_sha1)"
