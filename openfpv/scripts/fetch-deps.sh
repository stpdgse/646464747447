#!/usr/bin/env bash
# Downloads the OpenComputers jars used for compiling (API) and for dev-client runs (full jar).
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p libs
source <(grep -E '^(oc_api_version|oc_runtime_version)=' gradle.properties)
API="libs/OpenComputers-${oc_api_version}-api.jar"
[ -f "$API" ] || curl -fsSL -o "$API" "https://maven.cil.li/li/cil/oc/OpenComputers/${oc_api_version}/OpenComputers-${oc_api_version}-api.jar"
FULL="libs/OpenComputers-MC1.12.2-${oc_runtime_version}.jar"
if [ ! -f "$FULL" ]; then
  url=$(curl -fsSL 'https://api.modrinth.com/v2/project/opencomputers/version?game_versions=%5B%221.12.2%22%5D' \
    | python3 -c "import json,sys;v=[x for x in json.load(sys.stdin) if x['version_number']=='${oc_runtime_version}'][0];print(v['files'][0]['url'])")
  curl -fsSL -o "$FULL" "$url"
fi
ls -la libs
