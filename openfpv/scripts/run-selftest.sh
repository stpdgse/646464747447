#!/usr/bin/env bash
# One-command end-to-end self test: boots a real 1.12.2 client headless (Xvfb + Mesa software GL) with
# OpenComputers and OpenFPV, runs the scripted scenario in src/main/java/com/openfpv/dev/DevHarness.java
# and prints run/autotest/summary.txt. Screenshots land in run/autotest/screenshots/.
#
# Needs: JDK 8 (set JAVA_HOME), Xvfb, Mesa llvmpipe (libgl1-mesa-dri), OpenAL, curl, python3, unzip, zip.
set -euo pipefail
cd "$(dirname "$0")/.."
: "${JAVA_HOME:?set JAVA_HOME to a JDK 8}"
export PATH="$JAVA_HOME/bin:$PATH"

./scripts/fetch-deps.sh >/dev/null

# The Gradle cache with the Forge dev jars only exists after the first build; then apply the Side fix.
./gradlew classes --no-daemon -q
./scripts/fix-dev-side-enum.sh

mkdir -p run/mods
cp libs/OpenComputers-MC1.12.2-*[0-9].jar run/mods/ 2>/dev/null || cp libs/OpenComputers-MC1.12.2-1.8.*.jar run/mods/
[ -f run/options.txt ] || cat > run/options.txt <<'OPT'
version:1343
renderDistance:8
guiScale:2
maxFps:30
fboEnable:true
pauseOnLostFocus:false
fullscreen:false
enableVsync:false
tutorialStep:none
soundCategory_master:0.0
lang:en_us
OPT
rm -rf run/autotest run/saves/openfpv_autotest run/crash-reports

# LWJGL 2 enumerates display modes through XRandR, which plain `xvfb-run` does not enable.
DISP=":$((RANDOM % 200 + 100))"
Xvfb "$DISP" -screen 0 1280x720x24 +extension RANDR +extension GLX +render -noreset >/dev/null 2>&1 &
XVFB=$!
trap 'kill $XVFB 2>/dev/null || true' EXIT
sleep 2

DISPLAY="$DISP" ./gradlew runClient --no-daemon -Pautotest=full || true

echo; echo "=========== self-test summary ==========="
cat run/autotest/summary.txt
grep -q "FAIL 0$" run/autotest/summary.txt
