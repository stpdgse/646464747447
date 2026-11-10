#!/usr/bin/env bash
# Benchmarks the Ferrite backends (and optionally a vanilla server) with the bot swarm.
#
#   scripts/bench-net.sh                                  # tokio vs io_uring
#   scripts/bench-net.sh <server.jar> <java>              # ...and vanilla status pings
#
# Two scenarios:
#   churn   : a new connection per operation (status ping, or a full join). Server on cores 0-1.
#   steady  : persistent connections doing ping/pong. Server on ONE core and the swarm on three,
#             so the server is the bottleneck and the result is per-core throughput.
#
# Everything runs over loopback, where ~80% of the server's CPU is the kernel's TCP stack, which
# flatters all servers equally. Compare the numbers with each other, not with a real network.
# (`taskset` and 4+ cores are needed for pinning; without them the runs are unpinned.)
set -euo pipefail
cd "$(dirname "$0")/.."

jar="${1:-}"; java="${2:-java}"
port=25710
secs="${BENCH_SECONDS:-10}"
srv_cpus="0,1"; swarm_cpus="2,3"
pin_srv=(); pin_swarm=(); pin_srv1=(); pin_swarm3=()
if command -v taskset >/dev/null && [[ "$(nproc)" -ge 4 ]]; then
  pin_srv=(taskset -c "$srv_cpus"); pin_swarm=(taskset -c "$swarm_cpus")
  pin_srv1=(taskset -c 0); pin_swarm3=(taskset -c 1-3)
fi

cargo build --release -q -p ferrite-server -p ferrite-swarm
server=target/release/ferrite-server
swarm=target/release/ferrite-swarm

pid=""
stop() {
  if [[ -n "$pid" ]]; then kill "$pid" 2>/dev/null || true; wait "$pid" 2>/dev/null || true; fi
  pid=""
  sleep 0.5
}
trap stop EXIT

load() { # mode connections seconds [threads]
  "${swarm_pin[@]}" "$swarm" --addr "127.0.0.1:$port" --mode "$1" --connections "$2" --duration "$3" --threads "${4:-2}"
}
swarm_pin=("${pin_swarm[@]}")
summarize() { awk '/^ok:/{ok=$2} /^failed:/{f=$2} /^rate:/{r=$2} /^latency:/{p50=$3; p99=$7} /^error:/{if(!e)e=$0} END{printf "%10s ops/s  p50 %8s  p99 %8s  (ok %s, failed %s)\n", r, p50, p99, ok, f; if(e)print "      " e}'; }

# Waits until the server at $port accepts connections; dies loudly if the process exited.
wait_ready() {
  for _ in $(seq 1 100); do
    kill -0 "$pid" 2>/dev/null || { echo "server exited before becoming ready" >&2; exit 1; }
    (exec 3<>"/dev/tcp/127.0.0.1/$port") 2>/dev/null && return 0
    sleep 0.1
  done
  echo "server did not become ready on port $port" >&2; exit 1
}

run_suite() { # label modes...
  local label="$1"; shift
  echo "== $label"
  for spec in "$@"; do
    mode="${spec%%:*}"; conns="${spec##*:}"
    load "$mode" "$conns" 3 >/dev/null 2>&1 || true            # warm-up
    printf '  %-6s c=%-4s ' "$mode" "$conns"
    load "$mode" "$conns" "$secs" 2>&1 | summarize
  done
  stop
}

for backend in tokio uring; do
  swarm_pin=("${pin_swarm[@]}")
  "${pin_srv[@]}" "$server" --addr "127.0.0.1:$port" --backend "$backend" --threads 2 --max-players 1000000 >/dev/null 2>&1 &
  pid=$!; wait_ready
  run_suite "ferrite/$backend, churn (2 server threads on 2 cores)" status:64 status:256 join:64
done
for backend in tokio uring; do
  swarm_pin=("${pin_swarm3[@]}")
  "${pin_srv1[@]}" "$server" --addr "127.0.0.1:$port" --backend "$backend" --threads 1 --max-players 1000000 >/dev/null 2>&1 &
  pid=$!; wait_ready
  run_suite "ferrite/$backend, steady (1 server thread on 1 core)" ping:64 ping:256 ping:1024
done

if [[ -n "$jar" ]]; then
  work="$(mktemp -d)"; cp "$jar" "$work/server.jar"
  [[ "${FERRITE_ACCEPT_MOJANG_EULA:-}" == "true" ]] || { echo "set FERRITE_ACCEPT_MOJANG_EULA=true to include vanilla" >&2; exit 2; }
  printf 'eula=true\n' > "$work/eula.txt"
  printf 'online-mode=false\nserver-ip=127.0.0.1\nserver-port=%s\nwhite-list=false\nmax-players=100000\nview-distance=2\nlevel-type=minecraft\\:flat\nspawn-protection=0\n' "$port" > "$work/server.properties"
  (cd "$work" && exec "${pin_srv[@]}" "$java" -Xmx1G -jar server.jar --nogui) >"$work/log" 2>&1 &
  pid=$!
  for _ in $(seq 1 90); do grep -q 'Done (' "$work/log" 2>/dev/null && break; sleep 1; done
  wait_ready
  swarm_pin=("${pin_swarm[@]}")
  run_suite "vanilla 26.3 (JVM, churn, cores $srv_cpus)" status:64 status:256
  rm -rf "$work"
fi
