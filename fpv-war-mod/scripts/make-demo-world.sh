#!/usr/bin/env bash
# Generate a small flat peaceful singleplayer world at run/saves/DemoWorld (hard-capped, fail-fast).
# Uses the dedicated server to write the world, then copies it into the client's saves folder.
set -uo pipefail
cd "$(dirname "$0")/.."
export JAVA_HOME="${JAVA_HOME:-/opt/jdk-25}"; export PATH="$JAVA_HOME/bin:$PATH"
LIMIT="${1:-180}"
rm -rf run/world run/saves/DemoWorld
mkdir -p run
cat > run/server.properties <<'PROPS'
level-type=minecraft\:flat
generate-structures=false
difficulty=peaceful
spawn-monsters=false
spawn-animals=false
online-mode=false
gamemode=creative
force-gamemode=true
PROPS
scripts/server-smoke.sh "$LIMIT" >/tmp/ks-make-world.log 2>&1 || { echo "MAKE-DEMO-WORLD: FAIL (server did not boot)"; tail -15 /tmp/ks-make-world.log; exit 1; }
[[ -f run/world/level.dat ]] || { echo "MAKE-DEMO-WORLD: FAIL (no level.dat; log /tmp/ks-make-world.log)"; exit 1; }
mkdir -p run/saves && cp -r run/world run/saves/DemoWorld
echo "MAKE-DEMO-WORLD: PASS -> run/saves/DemoWorld"
