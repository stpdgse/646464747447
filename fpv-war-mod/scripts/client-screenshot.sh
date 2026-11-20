#!/usr/bin/env bash
# Launch the real Minecraft client (dev mode, offline account) on Xvfb with Mesa software GL,
# wait for the main menu, take a screenshot, then shut everything down.
#
# HARD CAP: the whole script is killed after HARD seconds no matter what (default 150).
# FAIL FAST: aborts immediately on a known fatal log line instead of waiting for the cap.
# NOTE: MC 26.x tries OpenGL then falls back to Vulkan; this container has no GPU, so Vulkan runs on
#       lavapipe (software) via VK_ICD_FILENAMES. 'Failed to create backend OpenGL' is expected and not fatal.
# Usage: scripts/client-screenshot.sh [out.png] [hard_cap_seconds]
set -uo pipefail
cd "$(dirname "$0")/.."
OUT="$(realpath -m "${1:-build/screenshots/title.png}")"; HARD="${2:-150}"
export JAVA_HOME="${JAVA_HOME:-/opt/jdk-25}"; export PATH="$JAVA_HOME/bin:$PATH"
mkdir -p "$(dirname "$OUT")" run
LOG="$(mktemp /tmp/ks-client.XXXXXX.log)"; DISP=:97
export DISPLAY=$DISP XDG_RUNTIME_DIR=/tmp/xdg-rt LIBGL_ALWAYS_SOFTWARE=1 GALLIUM_DRIVER=llvmpipe VK_ICD_FILENAMES=/opt/lavapipe/lvp_icd.json ALSOFT_DRIVERS=null
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
setsid ./gradlew --no-daemon --max-workers=1 -Dorg.gradle.parallel=false runClient > "$LOG" 2>&1 &
PGID=$!
cleanup() { kill -TERM -- "-$PGID" 2>/dev/null; pkill -TERM -P "$PGID" 2>/dev/null; sleep 3; kill -KILL -- "-$PGID" 2>/dev/null; kill "$XPID" 2>/dev/null; }
trap cleanup EXIT

FATAL='Exception in thread|Crash report|Reported exception|FATAL|Could not (resolve|GET)|BUILD FAILED|Failed to initialize the mod loading'
ok=0; why=""
while (( SECONDS < HARD )); do
  if grep -qE "$FATAL" "$LOG"; then why="fatal error in log"; break; fi
  if ! kill -0 "$PGID" 2>/dev/null; then why="client process exited"; break; fi
  if grep -qE 'Sound engine started|Error starting SoundSystem' "$LOG"; then ok=1; break; fi
  (( SECONDS % 15 == 0 )) && say "waiting for main menu... ($(wc -l < "$LOG") log lines)" && sleep 1
  sleep 1
done
(( ok == 0 && SECONDS >= HARD )) && why="hard cap ${HARD}s reached"

if (( ok )); then
  say "main menu reached, letting it render 15s..."; sleep 15
  import -display $DISP -window root "$OUT" && say "screenshot: $OUT"
  echo "CLIENT SCREENSHOT: PASS (log: $LOG)"; exit 0
fi
echo "CLIENT SCREENSHOT: FAIL ($why) after ${SECONDS}s (log: $LOG)"
grep -v "DEBUG\|Masking" "$LOG" | grep -E "ERROR|Failed|Exception|Couldn" | head -8 | cut -c1-220
exit 1
