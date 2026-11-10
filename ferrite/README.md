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
| Parallelism | World split into regions that tick in parallel, each owning its own ECS world; cross-region effects are messages delivered at barriers in an order that never depends on thread timing (see [Simulation core](#simulation-core)). Parallel *systems inside* a region are deliberately not built yet |
| Networking | `tokio` and `io_uring` backends over one shared policy layer. **`tokio` is the default**: on loopback the benchmarks do not justify `io_uring` (see [Networking](#networking)) |
| Storage | Custom format: palette + bit-packed chunks, zstd, in a CRC-checked log-structured store with an in-memory index (see [World storage](#world-storage)); seed-exact vanilla world generation comes later |
| Game data | Generated from the official server's data reports, never hand-typed |
| Auth | Online mode (RSA + AES-CFB8 + Mojang session server, **on by default**) and offline mode; Query and RCON. Signed chat arrives with the play state (see [Authentication](#authentication)) |
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
| `ferrite-remote` | Query (UDP) and RCON (TCP) servers plus clients, verified against vanilla |
| `ferrite-ecs` | Archetype ECS: `#![forbid(unsafe_code)]`, generational entities, typed queries, bundles, deferred commands, resources. Deterministic by construction |
| `ferrite-world` | Chunk model (paletted, bit-packed sections), versioned codec with zstd, and the crash-safe log-structured chunk store |
| `ferrite-sim` | Region scheduler (rayon) with deterministic exchange, plus the particle workload and single-threaded reference used to prove determinism |
| `ferrite-server` | The executable (mimalloc, clap flags, both backends, online mode, Query, RCON) |

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

## Authentication

Online mode is **on by default** (`--online-mode false` disables it, with a warning). The flow is
vanilla's: the server sends a 1024-bit RSA public key (X.509 DER) and a 4-byte verify token; the
client answers with a random 16-byte secret and the token, both PKCS#1 v1.5 encrypted; both sides
switch to AES-128-CFB8 keyed (and IV'd) by the secret and compute Minecraft's non-standard signed
SHA-1 digest of `server id + secret + public key`; the server asks Mojang's session server whether
that hash was registered by the player.

What is worth knowing:

* **RSA is constant-time.** The server decrypts attacker-supplied ciphertext, which is exactly
  the "Marvin" timing attack (RUSTSEC-2023-0071). `rsa` is pinned to 0.10, which fixes it with
  *implicit rejection*: invalid padding yields a pseudo-random plaintext rather than an error.
  Our session therefore fails closed on length and a constant-time token comparison. CI runs
  `cargo audit` so a vulnerable version cannot come back.
* **Slots are taken after authentication**, not before, so connections stuck mid-login cannot
  keep a server full.
* **Mojang being down is not the player's fault.** An unreachable or erroring session server
  answers `authservers_down`; only an explicit "not verified" answers `unverified_username`. The
  response is parsed as untrusted JSON with protocol-sized limits (fuzzed).
* **Backpressure.** Verification runs on a worker pool behind a bounded queue; a login storm
  during an outage is refused immediately instead of consuming memory and sockets.
* **A pipelining bug in PR 1 was found and fixed here:** a client may send its first encrypted
  packets in the same TCP segment as the plaintext key packet, and the decoder used to treat
  those already-buffered bytes as plaintext.
* **`monoio` needs its `sync` feature** because authenticator threads complete logins by waking
  tasks that belong to another thread; without it monoio panics. The tests caught this.

Verified against vanilla in online mode: vanilla's encryption request parses, vanilla accepts our
RSA-encrypted secret and token, and its reply arrives encrypted with the secret *we* chose, which
proves the DER, RSA and CFB8 handling agree with the vanilla implementation. OpenSSL is used as a
second independent oracle for the key format and PKCS#1 v1.5 in both directions. The Mojang HTTP
calls themselves are tested against an in-process mock of the session server, because no real
Mojang account is available in CI.

**Signed chat is not implemented yet, on purpose.** It lives in the play state (chat session
packets, message chains) and cannot be verified without play-state chat packets and a real client
that signs messages. It will land with those, rather than as code nothing can test.

## Query and RCON

`ferrite-remote` implements both, enabled with `--query-port` and `--rcon-port`
(`FERRITE_RCON_PASSWORD`, an empty password is refused, and RCON binds to `127.0.0.1` unless told
otherwise because it is unencrypted).

Testing them against the real vanilla server found that the documentation is wrong or incomplete
in several places, so the code follows vanilla's measured behaviour:

| Observed in vanilla 26.3 | Documentation said |
|---|---|
| Query echoes the session id **verbatim** (and corrupts ids with high bits set) | only the low nibbles are used |
| A stat request of exactly 8 payload bytes is a full stat; **any other length of 4 or more is a basic stat** | 4 bytes is basic, 8 is full |
| RCON **drops the connection if two packets arrive in one read**, so a client must send its end-of-output marker only after the first reply | the marker can be pipelined |
| A failed RCON login leaves the connection open for a retry | not stated |
| An unknown command answers `Unknown or incomplete command. See below for error<cmd><--[HERE]` with no newline | not stated |

One deliberate difference: a repeated Query handshake returns the same token for the rest of the
30 second period, where vanilla issues a new one and invalidates the old. That stops anyone who
can spoof a source address from knocking out its legitimate token; clients cannot tell. Query
only ever answers requests carrying a valid token, plus a rate limit, so it cannot be used as an
amplifier (the handshake is at most 16 bytes in reply to 7). RCON additionally compares the
password in constant time, slows down and limits wrong guesses, and rejects oversized or
malformed packets from the header alone.

## Simulation core

The aim is the one guarantee the whole project depends on: **a tick produces the same state for any
number of threads and any scheduling.**

**How it works.** The world is a grid of regions. Each region owns an ECS `World` outright, so
there is no sharing and no locking. A tick is a list of phases. In a phase every region runs the
same function in parallel, touching only its own world and pushing messages to its outbox. Between
phases an *exchange* gathers outboxes in region order and delivers them, so an inbox is ordered by
(source region, emission order): a function of the messages, not of who finished first. Border
state travels as read-only "ghost" messages, and entities that cross a border as migration
messages.

**How it is proven.** `ferrite_sim::particles` is a neighbour-interaction workload whose
floating-point sums depend on addition order. A plain single-threaded `Reference` runs the same
physics with no regions, no ECS and no threads. The tests require every tick, bit for bit, to
match it for 7 region layouts x 5 thread counts, for a heavily unbalanced world, for rectangular
worlds, for 12 repeated 4-thread runs, and for random worlds (the tests are run with thousands of
cases in CI). A negative control keeps the check honest: with halo exchange disabled the result
*must* differ, and more than 200 particles cross region borders in the scenario.

**What it does not give.** Vanilla's exact *global* update order for rules that depend on it
(redstone, entity push collisions). The particle rules are Jacobi-style (everything reads the
start-of-tick state), which is order-independent. Order-dependent systems need an ordered mode,
and are designed system by system when they are built. Within one region, systems run
sequentially.

**ECS speed** (one run, 1M `pos += vel` entities; the VM is noisy, treat as +/-15%):

| | ferrite | hecs 0.11 | bevy_ecs 0.20 | plain `Vec` |
|---|---|---|---|---|
| Iterate 1M entities | 1.52 ms | 1.38 ms | 3.17 ms | 1.36 ms |
| Spawn 100k entities | 4.9 ms | 2.4 ms | 7.2 ms | n/a |
| Insert then remove a component, 100k | 6.5 ms | 8.6 ms | 14.1 ms | n/a |

Iteration is at memory-bandwidth speed. Spawn is about 2x slower than hecs. The first version was
slower still (spawn 9.0 ms, insert/remove 17.6 ms) because every spawn sorted and hashed a `Vec`
of component ids and every insert boxed its value; a per-bundle-type cache, cached archetype
transitions and typed moves fixed that.

**Scaling** (200,000 interacting particles, 16x16 regions, 4-core VM):

| Threads | ms per tick | Speedup |
|---|---|---|
| 1 | 103.7 | 1.00x |
| 2 | 57.8 | 1.79x |
| 3 | 37.3 | 2.78x |
| 4 | 28.7 | 3.62x (90% efficient) |

The region count barely matters (2x2 to 32x32 within about 10%). The honest caveat: the workload
is a toy, so this shows the *scheduler* scales, not that vanilla's real mob workload will.

## World storage

**Format.** A chunk is a column of 16x16x16 sections, each holding a block container (4096 states)
and a biome container (64 cells). A container is a single value, a palette plus bit-packed indices,
or packed raw ids, whichever its contents need, and it grows on demand. Packing is vanilla's rule
(an entry never straddles a 64-bit word), so the same buffers can later go straight to the network;
for example a 4096-entry section is 256, 342, 410, 456 and 512 longs at 4 to 8 bits. On disk a chunk
is written in *canonical form* (sorted minimal palette, narrowest width), so equal chunks always
encode to equal bytes, then zstd-compressed. The codec is versioned and treats its input as
hostile: the decompressed size is read from the frame header and refused if oversized before
anything is allocated, and every length, index and id is validated.

**Store.** An append-only log of CRC-checked records in segment files, plus an in-memory index from
chunk position to record. Overwritten and deleted chunks become dead space that `compact()`
reclaims while readers and writers keep running.

* **Torn writes.** Opening scans the newest segment in full; an incomplete or corrupt tail is
  truncated, and the discarded bytes are saved next to it as `*.discarded` instead of destroyed.
  Tested by cutting the log at **every byte offset** of a multi-record log.
* **Silent corruption.** Every `get` re-verifies the record's checksum, so a damaged chunk fails
  alone with an error and is repaired by simply writing it again. Structural damage in an older
  segment is reported, not guessed at.
* **Crash-safe compaction.** The output takes an id *between* the sealed sources and the new
  active segment (so replay order is always right), is fsynced before the rename, and the sources
  are then removed **oldest first**. A put always precedes the tombstone that cancels it, so no
  prefix of removals can bring a deleted chunk back. This is tested for every crash state, and a
  negative control proves it matters: removing segments newest-first *does* resurrect a deleted
  chunk. (I originally relied on `Drop` to delete old files, and a concurrent reader would have
  broken that ordering; working through the crash cases is what made me look.) Compaction also
  refuses to drop a corrupt live record rather than lose it silently.
* One process per directory (an exclusive lock file), and concurrent readers never block on writers
  beyond an index lookup.

**Measured** (one run on a 4-core cloud VM; a realistic generated terrain chunk with real block
state ids, caves, ores, sea level and varied biomes):

| | |
|---|---|
| Terrain chunk, raw to zstd 3 | 21,028 B to 1,612 B (13x); flat world 63 B; empty chunk 29 B |
| Encode / decode a terrain chunk | 158 us / 55 us (about 18,000 loads per second per core) |
| zstd level | 3 is the default: level 1 is 4% *larger* for the same time, level 6 only 5% smaller for 18% more time, level 9 7% smaller for twice the time |
| Store `put` / `get` (checksummed) | 1.2 us / 0.5 us, plus 180 us per write if fsynced every time |
| Rebuild the index on open | 6 ms for 20,000 chunks |
| Compact 20,000 records, half dead | 50 ms |

The first version spent 600 us serialising and 390 us decoding a terrain chunk, almost all of it in
my own canonicalisation and per-entry validation, not in zstd. Working on the palette instead of
every entry, and unpacking whole words, made them 5.4x and 7x faster with byte-identical output.

**Not here yet.** Block entities, entities, heightmaps and lighting (the format is versioned so
they can be added), world metadata, and a smarter index for worlds with many millions of chunks
(the index is a `HashMap`, and opening reads every record header). `fsync` cost depends on the
disk, so the 180 us figure is only indicative.

## Quality gates

* 254 tests (plus 6 oracle tests that need a server) including property tests, with the cipher cross-checked against the RustCrypto
  `cfb8` crate and checked directly against the NIST SP 800-38A vector, and compression cross-checked against `flate2`.
* Ten `cargo-fuzz` targets: frame decoder, codec readers, frame round-trip, whole-session driving
  (structure-aware, reaches login and configuration), the NBT reader, the Query handler, the Query
  reply parsers, the Mojang profile parser, the chunk decoder, and the store's recovery scan.
* `cargo audit` in CI: no known vulnerabilities (two transitive "unmaintained" notices remain).
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
4. **Online-mode authentication, Query and RCON** (signed chat moves to the play-state work, see
   [Authentication](#authentication)).
5. **Custom ECS and region scheduler** (deterministic parallel ticking), see
   [Simulation core](#simulation-core).
6. **World storage format and chunk pipeline**, see [World storage](#world-storage).
7. Block updates, physics, redstone (bit-exact).
8. Entities, mobs, villagers, trading, raids, breeding, farming.
9. Exact vanilla world generation.
10. Commands (Brigadier, selectors, NBT), datapacks, functions, advancements, loot tables.
11. Dimensions and portals.
12. Public benchmark suite against Paper, Folia, Pumpkin and Minestom.

## License

MIT OR Apache-2.0, at your option.
