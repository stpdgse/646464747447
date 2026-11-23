#!/bin/bash
# Rebuild everything and run the game in Node. usage: cycle.sh [seconds]
cd "$(dirname "$0")/.."
export JAVA_HOME=$PWD/work/jdk25
(cd classlib-ext && ../work/gradle-9.2.1/bin/gradle --no-daemon -q jar 2>&1 | grep -E 'error' -A3 | head -20)
(cd driver && ./run.sh out > run.log 2>&1)
grep -E 'GENERAT' driver/run.log | head -1
grep -E '^PROBLEM:' driver/run.log | cut -c1-200 | sort | uniq -c | sort -rn | head -6
grep -q 'GENERATED OK' driver/run.log || exit 1
python3 -I tools/wasmsplit.py driver/out/game.wasm driver/out/game-split.wasm 1000000 | tail -1
python3 -I tools/make_runtime.py driver/libs/tool/teavm-core-0.16.0.jar driver/out/game.wasm-runtime.js >/dev/null
python3 -I tools/make_archive.py work/client.jar driver/out/client-resources.zip work/asset-index-34.json work/assets driver/libs/game/icu4j-*.jar | tail -1
cp tools/index.html tools/mcweb.js tools/mcshader.js tools/mcaudio.js work/shaderc/mod/mcshader.wasm driver/out/
cd driver && node --max-old-space-size=8000 --stack-size=8000 ../tools/runnode.js out game-split.wasm ${1:-60} 2>&1 | grep -v -i 'warning\|trace-warnings' | cut -c1-260
