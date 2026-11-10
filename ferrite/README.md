# Ferrite

A from-scratch Minecraft: Java Edition server in Rust, built for multicore throughput and
bit-exact vanilla behaviour. Target release: **26.3** (protocol 777, world data version 5023).

> **Status: a server that accepts real connections.** Clients can ping it, log in (offline mode)
> and complete configuration; it then ends the connection politely because gameplay does not
> exist yet. Everything is verified against a real vanilla server. See the [roadmap](#roadmap).

```sh
cargo run --release -p ferrite-server -- --addr 127.0.0.1:25565 --max-players 100
cargo run --release -p ferrite-swarm  -- --addr 127.0.0.1:25565 --mode join --connections 32
```

## Decisions

| Area | Decision |
|---|---|
| Scope | Full vanilla parity, bit-exact wherever it is observable (redstone order, mob AI, RNG) |
| Parallelism | World split into regions ticked in parallel, ECS systems parallel inside a region, deterministic merge of cross-region effects at fixed sync points |
| Networking | `tokio` and `io_uring` backends over one shared policy layer. **`tokio` is the default**: on loopback the benchmarks do not justify `io_uring` (see [Networking](#networking)) |
| Storage | Custom on-disk format (no Anvil), with seed-exact vanilla world generation |
| Game data | Generated from the official server's data reports, never hand-typed |
| Auth | Online mode (RSA + AES-CFB8 + session server) and offline mode; signed chat |
| Platforms | Linux x86_64 and Windows |
| Toolchain | Pinned nightly, `unsafe` allowed where measured, mimalloc, per-tick arenas, LTO + PGO |
| Legal | MIT OR Apache-2.0, clean-room. No Mojang code or jar is committed; only ids, names and state layouts derived from the data reports. |

## Crates

| Crate | Purpose |
|---|---|
| `ferrite-data` | Generated tables: 95 registries (7,053 entries), 1,286 blocks / 35,723 block states, 213 packet ids, version constants |
| `ferrite-datagen` | Runs the vanilla data generator and emits `ferrite-data`'s tables. Validates every layout assumption the runtime relies on |
| `ferrite-nbt` | NBT with both root encodings (network and named), modified UTF-8, vanilla quota and depth accounting enforced before allocation |
| `ferrite-protocol` | VarInt/VarLong, zero-copy codec, framing, zlib (libdeflate), AES-128-CFB8, packet definitions for handshake/status/login/configuration, and the sans-IO `ServerSession` state machine |
| `ferrite-net` | `Lobby` (every connection decision, backend-independent) plus a multi-thread `tokio` backend and a thread-per-core `io_uring` backend (`monoio`, `SO_REUSEPORT`, Linux) |
| `ferrite-swarm` | Bot load tester: status, join and steady-state ping modes with latency percentiles. Its bots are checked against the vanilla server |
| `ferrite-server` | The executable (mimalloc, clap flags, both backends) |

### Regenerating the tables

The generator needs the official `server.jar` and the JDK it requires (Java 25 for 26.x).

```sh
scripts/fetch-server-jar.sh 26.3 /tmp/jar          # downloads and verifies the SHA-1
cargo datagen --jar /tmp/jar/server-26.3.jar --java /path/to/jdk25/bin/java
```

Output is byte-for-byte reproducible, and CI fails if the committed tables differ from a fresh
run. The generator **fails loudly** instead of guessing. For example, it caught that block-state
ids are numbered over properties *sorted by name* (the report lists them in class-declaration
order), and it asserts that for every one of the 35,723 states.

`packets.json` contains packet names and ids only, not field layouts. Packet structures are
written by hand and verified against a real vanilla server (differential testing, planned).

## Protocol layer

* `varint`: vanilla-exact semantics, branch-free encoder with a single wide store.
* `codec`: `Decode` borrows from the input (no copies); every peer-controlled length
  (`read_str`, `read_byte_array`, `read_list`) requires an explicit bound.
* `frame`: enforces the same limits as vanilla (3-byte length prefix, 2 MiB frames, 8 MiB
  decompressed, compressed packets must meet the threshold, declared size must match exactly).
* `cipher`: CFB8 encryption is a serial dependency chain, but decryption only needs ciphertext
  we already have, so the decryptor batches windows through AES-NI in parallel.

### Measured performance

Intel Xeon @ 2.1 GHz (cloud VM), one core, `cargo bench -p ferrite-protocol`. Indicative only;
these will be re-measured on dedicated hardware.

| Operation | Result |
|---|---|
| VarInt encode | 445 M values/s (3x the naive `encode + extend_from_slice`) |
| VarInt decode | 408 M values/s |
| Frame decode, 32 B packet, no compression | 31 ns (about 32 M packets/s) |
| Frame decode, 16 KiB zlib packet | 2.2 GiB/s |
| Frame encode, 16 KiB, zlib level 4 | 440 MiB/s |
| CFB8 decrypt | 424 MiB/s |
| CFB8 encrypt | 63 MiB/s |

CFB8 encryption is latency-bound (about 10 AES rounds per byte, one byte at a time). The
planned fix is to interleave *different connections'* streams so the AES unit stays busy, which
a thread-per-core server is well placed to do.

### Connection state machine

`session::ServerSession` is sans-IO: feed it bytes, poll it for `Event`s, write out
`pending_output()`. It owns what changes how bytes are *interpreted* (packet ids per state,
compression, encryption) and the fixed parts of the protocol (known-pack negotiation, registry and
tag data, finishing configuration). Decisions (status text, who may join) come back as events.
Clients that lack the matching `minecraft:core` data pack are turned away with a clear message,
because shipping full registry data is not implemented yet.

## Verified against vanilla

`tests/oracle.rs` speaks to Mojang's real 26.3 server using only this repository's codec. Every
packet it sends must decode with zero leftover bytes, which is how field-layout mistakes are
found. It already caught one (a missing `session_id` in `login_finished`). It also asserts that the
generated configuration data (32 synchronized registries and 773 tags with their resolved ids)
equals what vanilla sends, and that our offline-mode UUID derivation equals vanilla's.

```sh
# You must accept the Minecraft EULA (https://aka.ms/MinecraftEULA) yourself; the script refuses
# to run otherwise and never writes eula=true on its own.
FERRITE_ACCEPT_MOJANG_EULA=true scripts/oracle.sh /path/to/server-26.3.jar /path/to/jdk25/bin/java
```

Two things in the data are deliberately *not* compared by position, because vanilla's order is
not stable: the order of tag registries and the order of tags inside a registry (hash iteration,
different on every server start).

## Networking

`ferrite_net::Lobby` makes every decision about a connection (server list text, name rules,
capacity, version rejection, kicking) by driving a `ServerSession`. The backends only move bytes,
so behaviour cannot drift between them, and the end-to-end tests run **every scenario against
both**: garbage input, slow-loris timeouts, byte-at-a-time delivery, mid-login disconnects, a full
server, and 600 concurrent joins with zero failures.

```sh
scripts/bench-net.sh                                    # tokio vs io_uring
FERRITE_ACCEPT_MOJANG_EULA=true scripts/bench-net.sh server.jar /path/to/jdk25/bin/java  # + vanilla
```

Measured on a 4-core, 2.1 GHz cloud VM over loopback, server and load generator pinned to separate
cores (one run each; expect roughly +/-10% noise):

| Scenario | ferrite / tokio | ferrite / io_uring | vanilla 26.3 |
|---|---|---|---|
| Status ping, new connection each, 2 server cores, 64 bots | **~47,000/s** | ~31,000/s | ~34,000/s |
| Same, 256 bots | **~45,000/s** | ~32,000/s | ~38,000/s |
| Full login + configuration to play, 64 bots | ~2,200/s | ~1,900/s | not compared (vanilla also spawns the player) |
| Steady ping/pong on persistent connections, **1 server core** | ~212,000/s | ~192,000/s | not applicable |

What this does and does not show:

* **The socket layer is not where the speed will come from.** On loopback about 80% of the
  server's CPU is the kernel's TCP stack and only about 15% is Ferrite's own code. Against vanilla
  the status-ping advantage is a modest 1.2 to 1.4x, not an order of magnitude. The multiplier has
  to come from the tick loop, the ECS and the world, which is what the next subsystems are for.
* **`io_uring` did not win here.** It makes far fewer syscalls (2.3 per status operation against
  10.7 for tokio) and uses a third less user CPU, but spends more in the kernel on this VM, so
  throughput is roughly even and sometimes lower. Its tail latency is much better at 1,024
  persistent connections (p99 9 ms against 140 ms). A real NIC and many long-lived connections may
  change the balance, so the backend stays available and this will be re-measured on your hardware.
* Vanilla ran with a 3 second warm-up, on two cores that also host its JIT and GC threads.

The benchmarks also found two cheap wins, both fixed: constructing a session allocated a
libdeflate compressor for every connection even though status pings never compress (3.6 us down to
50 ns), and the load tester itself ran out of ephemeral ports until its sockets were closed with a
reset instead of lingering in TIME_WAIT.

## Quality gates

* 116 tests (plus 2 oracle tests that need a server) including property tests, with the cipher cross-checked against the RustCrypto
  `cfb8` crate and checked directly against the NIST SP 800-38A vector, and compression cross-checked against `flate2`.
* Five `cargo-fuzz` targets: frame decoder, codec readers, frame round-trip, whole-session driving
  (structure-aware, reaches login and configuration), and the NBT reader.
* Miri (strict provenance, symbolic alignment) on the `unsafe` code.
* `clippy -D warnings`, rustfmt, CI on Linux and Windows.

```sh
cargo test --workspace
cargo bench -p ferrite-protocol --bench protocol
cd crates/ferrite-protocol/fuzz && cargo fuzz run frame_decoder
```

## Roadmap

One pull request per subsystem.

1. **Foundation**: data pipeline, protocol primitives, framing, cipher.
2. **Packets and sessions**: NBT, packet definitions, handshake / status / login / configuration
   state machine, vanilla oracle tests.
3. **Networking**: `tokio` and `io_uring` backends, connection policy, bot-swarm load tester, benchmarks.
4. Online-mode authentication, signed chat, Query and RCON.
5. Custom ECS and region scheduler (deterministic parallel ticking).
6. World storage format and chunk pipeline.
7. Block updates, physics, redstone (bit-exact).
8. Entities, mobs, villagers, trading, raids, breeding, farming.
9. Exact vanilla world generation.
10. Commands (Brigadier, selectors, NBT), datapacks, functions, advancements, loot tables.
11. Dimensions and portals.
12. Public benchmark suite against Paper, Folia, Pumpkin and Minestom.

## License

MIT OR Apache-2.0, at your option.
