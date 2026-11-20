#!/usr/bin/env bash
# Launch Blockbench (Electron) on a virtual display with software WebGL and a CDP debug port.
# Usage: ./run-blockbench.sh [--port 9222] [--size 1600x900]   (runs in foreground; kill to stop)
set -euo pipefail
BB_DIR="${BB_DIR:-/home/user/jannisx11/blockbench}"
PORT=9222; SIZE=1600x900
while [[ $# -gt 0 ]]; do case $1 in --port) PORT=$2; shift 2;; --size) SIZE=$2; shift 2;; *) shift;; esac; done
exec xvfb-run -a -s "-screen 0 ${SIZE}x24" \
  "$BB_DIR/node_modules/electron/dist/electron" "$BB_DIR" \
  --no-sandbox --use-gl=angle --use-angle=swiftshader --enable-unsafe-swiftshader --ignore-gpu-blocklist \
  --remote-debugging-port="$PORT" --userData "${BB_DATA:-/tmp/bbdata}"
