#!/usr/bin/env bash
# Runs the differential tests (crates/ferrite-protocol/tests/oracle.rs) against Mojang's real
# server: our packet codec talks to vanilla, and every packet it sends must decode exactly.
#
#   FERRITE_ACCEPT_MOJANG_EULA=true scripts/oracle.sh <server.jar> [path/to/java]
#
# Running the vanilla server requires accepting the Minecraft EULA (https://aka.ms/MinecraftEULA).
# This script never does that for you: it writes `eula=true` only if you set the variable above.
# The server runs in a temporary directory, bound to 127.0.0.1, in offline mode, and is stopped
# when the script exits. Nothing from the jar is stored in the repository.
set -euo pipefail

jar="${1:?usage: oracle.sh <server.jar> [java]}"
java="${2:-java}"
port="${FERRITE_ORACLE_PORT:-25599}"

if [[ "${FERRITE_ACCEPT_MOJANG_EULA:-}" != "true" ]]; then
  echo "Refusing to run: set FERRITE_ACCEPT_MOJANG_EULA=true to confirm that you accept the" >&2
  echo "Minecraft EULA (https://aka.ms/MinecraftEULA) for running the vanilla server." >&2
  exit 2
fi

work="$(mktemp -d)"
server_pid=""
cleanup() {
  [[ -n "$server_pid" ]] && kill "$server_pid" 2>/dev/null || true
  rm -rf "$work"
}
trap cleanup EXIT

cp "$jar" "$work/server.jar"
printf 'eula=true\n' > "$work/eula.txt"
cat > "$work/server.properties" <<PROPS
online-mode=false
server-ip=127.0.0.1
server-port=$port
network-compression-threshold=256
level-type=minecraft\:flat
generate-structures=false
view-distance=2
simulation-distance=2
spawn-protection=0
max-players=4
motd=ferrite oracle
white-list=false
enforce-whitelist=false
enable-rcon=false
enable-query=false
sync-chunk-writes=false
PROPS

(cd "$work" && exec "$java" -Xmx1G -jar server.jar --nogui) > "$work/server.log" 2>&1 &
server_pid=$!

for _ in $(seq 1 90); do
  grep -q 'Done (' "$work/server.log" 2>/dev/null && break
  kill -0 "$server_pid" 2>/dev/null || { cat "$work/server.log" >&2; echo "server exited early" >&2; exit 1; }
  sleep 1
done
grep -q 'Done (' "$work/server.log" || { echo "server did not start in time" >&2; exit 1; }

cd "$(dirname "$0")/.."
FERRITE_ORACLE="127.0.0.1:$port" \
  cargo test -p ferrite-protocol --test oracle -- --ignored --test-threads=1 "${@:3}"
