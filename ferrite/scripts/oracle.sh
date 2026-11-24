#!/usr/bin/env bash
# Runs the differential tests (crates/ferrite-protocol/tests/oracle.rs) against Mojang's real
# server: our packet codec talks to vanilla, and every packet it sends must decode exactly.
#
#   FERRITE_ACCEPT_MOJANG_EULA=true scripts/oracle.sh <server.jar> [path/to/java]
#
# Running the vanilla server requires accepting the Minecraft EULA (https://aka.ms/MinecraftEULA).
# This script never does that for you: it writes `eula=true` only if you set the variable above.
# Two servers run in a temporary directory, bound to 127.0.0.1 (one offline, one online-mode), and are stopped
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
pids=()
cleanup() {
  for p in "${pids[@]:-}"; do [[ -n "$p" ]] && kill "$p" 2>/dev/null || true; done
  rm -rf "$work"
}
trap cleanup EXIT

# start_server <name> <port> <online-mode> [extra properties...]
start_server() {
  local name="$1" sport="$2" online="$3"; shift 3
  local dir="$work/$name"
  mkdir -p "$dir"
  cp "$jar" "$dir/server.jar"
  printf 'eula=true\n' > "$dir/eula.txt"
  cat > "$dir/server.properties" <<PROPS
online-mode=$online
server-ip=127.0.0.1
server-port=$sport
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
sync-chunk-writes=false
PROPS
  for extra in "$@"; do echo "$extra" >> "$dir/server.properties"; done
  (cd "$dir" && exec "$java" -Xmx1G -jar server.jar --nogui) > "$dir/server.log" 2>&1 &
  pids+=("$!")
}

wait_ready() { # name
  for _ in $(seq 1 120); do
    grep -q 'Done (' "$work/$1/server.log" 2>/dev/null && return 0
    sleep 1
  done
  cat "$work/$1/server.log" >&2; echo "server $1 did not start in time" >&2; exit 1
}

# Offline server for the protocol tests; online server for the key-exchange test.
start_server offline "$port" false enable-rcon=true rcon.port=$((port + 2)) rcon.password=ferrite-oracle \
  enable-query=true query.port=$((port + 3))
start_server online $((port + 1)) true
wait_ready offline
wait_ready online

cd "$(dirname "$0")/.."
export FERRITE_ORACLE="127.0.0.1:$port"
export FERRITE_ORACLE_ONLINE="127.0.0.1:$((port + 1))"
export FERRITE_ORACLE_RCON="127.0.0.1:$((port + 2))"
export FERRITE_ORACLE_QUERY="127.0.0.1:$((port + 3))"
cargo test -p ferrite-protocol --test oracle -- --ignored --test-threads=1 "${@:3}"
cargo test -p ferrite-remote --test vanilla -- --ignored --test-threads=1 "${@:3}"
