#!/usr/bin/env bash
# Runs the differential SNBT test: Ferrite's parser against the official server's own.
#
#   scripts/vanilla-snbt.sh /path/to/unpacked-bundler-dir [cases]
set -euo pipefail
dir="${1:?usage: vanilla-snbt.sh <unpacked bundler dir> [cases]}"
cp="$(find "$dir/versions" "$dir/libraries" -name '*.jar' | sort | tr '\n' ':')"
export FERRITE_VANILLA_CLASSPATH="$cp"
[[ -n "${2:-}" ]] && export FERRITE_SNBT_CASES="$2"
cd "$(dirname "$0")/.."
cargo test -p ferrite-nbt --test vanilla_snbt -- --ignored --nocapture
