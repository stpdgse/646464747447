#!/usr/bin/env bash
# Runs the differential codec tests: Ferrite's packets are decoded by the official server's own
# codecs. Needs a JDK and the server jar; the jar's libraries come from the directory the data
# generator (or the bundler) unpacked.
#
#   scripts/vanilla-codec.sh /path/to/unpacked-bundler-dir
#
# The directory must contain versions/<v>/server-<v>.jar and libraries/ (cargo datagen --work-dir
# leaves exactly that behind).
set -euo pipefail
dir="${1:?usage: vanilla-codec.sh <unpacked bundler dir>}"
cp="$(find "$dir/versions" "$dir/libraries" -name '*.jar' | sort | tr '\n' ':')"
export FERRITE_VANILLA_CLASSPATH="$cp"
cd "$(dirname "$0")/.."
cargo test -p ferrite-game --test vanilla_codec -- --ignored --nocapture
