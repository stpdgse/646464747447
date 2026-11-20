#!/usr/bin/env bash
# Launch the real client, auto-join run/saves/DemoWorld, let the scripted demo flight run, and screenshot the
# FPV view every few seconds. Software rendering (Mesa lavapipe/Vulkan on Xvfb); no GPU or human input needed.
#
# HARD CAP: killed after HARD seconds regardless (default 240). FAIL FAST on fatal log lines.
# Usage: [KS_DEMO_MODE=fly|model|strike] [KS_FRAME_DELAY=1] scripts/demo-screenshots.sh [out_dir] [hard_cap_seconds] [shots] [interval_seconds]
# KS_FRAME_DELAY: seconds (may be fractional) to sleep between polls; use 0.2 for dense frames (GIFs).
set -uo pipefail
cd "$(dirname "$0")/.."
OUT="$(realpath -m "${1:-build/screenshots/demo}")"; HARD="${2:-240}"; SHOTS="${3:-6}"; EVERY="${4:-4}"
export JAVA_HOME="${JAVA_HOME:-/opt/jdk-25}"; export PATH="$JAVA_HOME/bin:$PATH"
[[ -d run/saves/DemoWorld ]] || { echo "run scripts/make-demo-world.sh first"; exit 2; }
mkdir -p "$OUT"; rm -f "$OUT"/shot_*.png "$OUT"/fail.png
LOG="$(mktemp /tmp/ks-demo.XXXXXX.log)"; DISP=:98
export DISPLAY=$DISP XDG_RUNTIME_DIR=/tmp/xdg-rt LIBGL_ALWAYS_SOFTWARE=1 GALLIUM_DRIVER=llvmpipe \
       VK_ICD_FILENAMES=/opt/lavapipe/lvp_icd.json ALSOFT_DRIVERS=null
mkdir -p "$XDG_RUNTIME_DIR"; chmod 700 "$XDG_RUNTIME_DIR"
say() { printf '[%3ss] %s\n' "$SECONDS" "$*"; }

# Headless dev run: skip the first-launch onboarding, tutorial hints, and never pause when the window is unfocused.
ensure_options() {
  local f=run/options.txt; touch "$f"
  for kv in onboardAccessibility:false tutorialStep:none pauseOnLostFocus:false; do
    k=${kv%%:*}
    if grep -q "^$k:" "$f"; then sed -i "s/^$k:.*/$kv/" "$f"; else echo "$kv" >> "$f"; fi
  done
}
ensure_options

Xvfb $DISP -screen 0 1280x720x24 +extension GLX +render -noreset >/dev/null 2>&1 &
XPID=$!
setsid ./gradlew --no-daemon --max-workers=1 -Dorg.gradle.parallel=false runClient -Pks.world=DemoWorld -Pks.demo=${KS_DEMO_MODE:-fly} > "$LOG" 2>&1 &
PGID=$!
cleanup() { kill -TERM -- "-$PGID" 2>/dev/null; pkill -TERM -P "$PGID" 2>/dev/null; sleep 3; kill -KILL -- "-$PGID" 2>/dev/null; kill "$XPID" 2>/dev/null; }
trap cleanup EXIT

FATAL='Exception in thread|Crash report|Reported exception|FATAL|Could not (resolve|GET)|BUILD FAILED|Failed to initialize the mod loading|Failed to load world|Error loading'
EVERY_US=$(awk -v e="$EVERY" 'BEGIN { printf "%d", e * 1000000 }')
START_US=$(awk -v e="${KS_START_DELAY:-3}" 'BEGIN { printf "%d", e * 1000000 }')
now_us() { local t=${EPOCHREALTIME/./}; echo "$t"; }
why=""; started=0; taken=0; next_us=0; last_say=-1
while (( SECONDS < HARD )); do
  if grep -qE "$FATAL" "$LOG"; then why="fatal error in log"; break; fi
  kill -0 "$PGID" 2>/dev/null || { why="client process exited"; break; }
  if (( ! started )) && grep -q 'KS_DEMO started' "$LOG"; then started=1; next_us=$(( $(now_us) + START_US )); say "demo flight started"; fi
  if (( started )) && (( $(now_us) >= next_us )) && (( taken < SHOTS )); then
    taken=$((taken + 1)); import -display $DISP -window root "$OUT/shot_$(printf '%03d' $taken).png" || true
    (( taken % 10 == 0 || taken == 1 )) && say "shot $taken"
    next_us=$(( $(now_us) + EVERY_US ))
  fi
  (( taken >= SHOTS )) && break
  (( SECONDS % 20 == 0 )) && [[ "$last_say" != "$SECONDS" ]] && { say "waiting... started=$started shots=$taken"; last_say=$SECONDS; }
  sleep ${KS_FRAME_DELAY:-0.05}
done
if (( taken >= SHOTS )); then echo "DEMO SCREENSHOTS: PASS ($taken shots in $OUT, log $LOG)"; exit 0; fi
import -display $DISP -window root "$OUT/fail.png" 2>/dev/null && say "saved what is on screen: $OUT/fail.png"
echo "DEMO SCREENSHOTS: FAIL (${why:-hard cap ${HARD}s reached}; shots=$taken, log $LOG)"
grep -v "DEBUG\|Masking" "$LOG" | grep -E "ERROR|Exception|Failed|Couldn" | head -8 | cut -c1-240
exit 1
