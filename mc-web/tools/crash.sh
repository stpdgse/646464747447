#!/bin/bash
# Run the built game in Node and print only the crash report head (description + exception line).
cd "$(dirname "$0")/../driver"
node --max-old-space-size=8000 --stack-size=8000 ../tools/runnode.js out game-split.wasm ${1:-60} 2>&1 \
  | grep -v -i 'trace-warnings\|FakeClass' | grep -E -A3 '^Description:|^ERR |CompileError' | cut -c1-300 | head -${2:-20}
