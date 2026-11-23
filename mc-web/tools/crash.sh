#!/bin/bash
# Run the built game in headless Chromium and print the crash description and exception lines.
cd "$(dirname "$0")/../driver"
NODE_PATH=$(npm root -g) node ../tools/runpage.js out ${1:-60} 2>&1 \
  | grep -v -i 'FakeClass\|trace-warnings' | grep -E -A${3:-3} '^\[console\] Description:|^\[console\] ERR |CompileError|pageerror|PAGE CRASHED' \
  | cut -c1-300 | head -${2:-24}
