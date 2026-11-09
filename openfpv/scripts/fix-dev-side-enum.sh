#!/usr/bin/env bash
# Workaround for a ForgeGradle 3 + Minecraft 1.12.2 dev-environment bug.
#
# The MCP merge tool bundles its own stub of net.minecraftforge.fml.relauncher.Side with an extra
# BUKKIT constant, and that stub can end up in the dev jar instead of Forge's real enum. Forge's
# NetworkRegistry then loops over Side.values(), finds no channel map for BUKKIT and the client
# dies at start-up with:
#     NullPointerException at NetworkRegistry.newChannel(NetworkRegistry.java:207)
# (a completely stock Forge MDK does the same, so it is not caused by any mod).
#
# This script copies the real Side.class out of Forge's official universal jar into the cached dev
# jars. Run it once after the first `./gradlew build`/`runClient` has produced them. It only touches
# the Gradle cache, never the project or the mod jar.
set -euo pipefail
FORGE="${1:-1.12.2-14.23.5.2860}"
CACHE="${GRADLE_USER_HOME:-$HOME/.gradle}/caches/forge_gradle"
UNIVERSAL="$CACHE/maven_downloader/net/minecraftforge/forge/$FORGE/forge-$FORGE-universal.jar"
REPO_ROOT="$CACHE/minecraft_user_repo/net/minecraftforge/forge"
ENTRY=net/minecraftforge/fml/relauncher/Side.class

[ -f "$UNIVERSAL" ] || { echo "universal jar not found: $UNIVERSAL (run ./gradlew build first)"; exit 1; }
ls -d "$REPO_ROOT/$FORGE"* >/dev/null 2>&1 || { echo "dev jars not found under $REPO_ROOT (run ./gradlew build first)"; exit 1; }

tmp=$(mktemp -d); trap 'rm -rf "$tmp"' EXIT
(cd "$tmp" && unzip -q -o "$UNIVERSAL" "$ENTRY")

# The dev jars live in "$FORGE" and in "${FORGE}_mapped_snapshot_..." directories.
for jar in "$REPO_ROOT"/"$FORGE"*/forge-"$FORGE"*.jar; do
  case "$jar" in *-obf.jar|*-binpatched.jar|*-inject_src.jar) continue;; esac
  # (read into a variable: `grep -q` on a pipe would SIGPIPE unzip under pipefail)
  if unzip -l "$jar" "$ENTRY" >/dev/null 2>&1 && [ -n "$(unzip -p "$jar" "$ENTRY" | tr -d "\\0" | grep -a BUKKIT || true)" ]; then
    (cd "$tmp" && zip -q "$jar" "$ENTRY")
    echo "fixed $(basename "$jar")"
  else
    echo "ok    $(basename "$jar")"
  fi
done
