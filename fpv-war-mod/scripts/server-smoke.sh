#!/usr/bin/env bash
# Boot the real dedicated server (with the mod + GeckoLib), wait for "Done", then stop it.
# Hard-capped by a timeout and always cleans up its process group, so it can never hang.
# Usage: scripts/server-smoke.sh [timeout_seconds]   (default 240)
set -uo pipefail
cd "$(dirname "$0")/.."
export JAVA_HOME="${JAVA_HOME:-/opt/jdk-25}"; export PATH="$JAVA_HOME/bin:$PATH"
LIMIT="${1:-240}"; LOG="$(mktemp /tmp/ks-server-smoke.XXXXXX.log)"
mkdir -p run && echo "eula=true" > run/eula.txt

setsid ./gradlew --no-daemon --max-workers=1 -Dorg.gradle.parallel=false runServer > "$LOG" 2>&1 &
PGID=$!
cleanup() { kill -TERM -- "-$PGID" 2>/dev/null; sleep 5; kill -KILL -- "-$PGID" 2>/dev/null; }
trap cleanup EXIT

ok=0
for ((i = 0; i < LIMIT; i++)); do
  if grep -q 'Done (' "$LOG"; then ok=1; break; fi
  if ! kill -0 "$PGID" 2>/dev/null; then break; fi   # gradle died early
  sleep 1
done

if [[ $ok -eq 1 ]]; then
  grep -E 'GeckoLib 5|Kamikaze Skies 0|Done \(' "$LOG" | grep -v DEBUG | sed 's/^/  /'
  # a mod-caused failure would show as a Kamikaze/crash line besides the benign missing server.properties
  if grep -E 'Exception|Crash' "$LOG" | grep -vE 'server.properties|UnixException|NoSuchFile' | grep -q .; then
    echo "SERVER SMOKE: FAIL (exceptions in log: $LOG)"; exit 1
  fi
  echo "SERVER SMOKE: PASS (log: $LOG)"; exit 0
fi
echo "SERVER SMOKE: FAIL (no 'Done' within ${LIMIT}s; log: $LOG)"; tail -20 "$LOG"; exit 1
