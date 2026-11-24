#!/usr/bin/env bash
# Runs the differential command-parsing test: Ferrite's parser against the official server's own
# command dispatcher. Needs a JDK and the server jar; the jar's libraries come from the directory
# the data generator unpacked.
#
#   scripts/vanilla-commands.sh /path/to/unpacked-bundler-dir [cases]
set -euo pipefail
dir="${1:?usage: vanilla-commands.sh <unpacked bundler dir> [cases]}"
cp="$(find "$dir/versions" "$dir/libraries" -name '*.jar' | sort | tr '\n' ':')"
export FERRITE_VANILLA_CLASSPATH="$cp"
[[ -n "${2:-}" ]] && export FERRITE_PARSE_CASES="$2"
cd "$(dirname "$0")/.."
cargo test -p ferrite-commands --test vanilla_parse -- --ignored --nocapture
