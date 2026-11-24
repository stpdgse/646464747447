# Local setup

Layout: git repo root is `646464747447/`; the Rust workspace is in `ferrite/` (there are also
unrelated `gbc-handheld/`, `docs/` folders; ignore them). The zip contains `ferrite/` only plus
`.github/` copy under `repo-root-extras/`; if you clone the repo instead, you get everything.

## Toolchain
- `rustup` will install the pinned nightly from `ferrite/rust-toolchain.toml` (it was
  `nightly-2026-11-09` in the cloud). Components: rustfmt, clippy, miri (miri only for one CI job).
- JDK 25 (Temurin) for the Java harnesses and the vanilla server (26.x requires Java 25).
  Set `FERRITE_JAVA`/`FERRITE_JAVAC` if not on PATH.
- Python 3 for the recording scripts (stdlib only; `docs/handoff/tools/mc.py` is the RCON/anvil helper).
- Linux is the main dev platform. Windows works for the server; the scripts are bash.

## The Mojang jar (NOT included, do not redistribute)
`ferrite/scripts/fetch-server-jar.sh 26.3 /tmp/jar` downloads and SHA-1-verifies the official
bundler `server.jar` from Mojang's launcher manifest. Then unpack its libraries once:
```
cd /tmp && mkdir gen-work && cd gen-work && java -DbundlerMainClass=net.minecraft.data.Main \
   -jar /tmp/jar/server-26.3.jar --all --output out    # (or just run cargo datagen, below)
```
The differential test scripts expect a directory containing `versions/*/server-26.3.jar` and
`libraries/**.jar` (the layout the bundler unpacks). In the cloud this was `/tmp/claude-0/gen-work`;
create it by running the bundler once with `-Dbundlerrepo`-style unpacking, or simply run the
datagen once with `--work-dir /tmp/gen-work`, which unpacks exactly that layout.
Generated game data is regenerated with:
```
cargo datagen --jar /tmp/jar/server-26.3.jar --java /path/to/jdk25/bin/java --work-dir /tmp/gen-work
git diff --exit-code crates/ferrite-data/src/generated    # must be clean (CI checks this)
```

## Build and test
```
cd ferrite
cargo build --workspace
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings      # CI uses -D warnings
cargo test --workspace                                       # no jar needed; replays recordings
```
Jar-needing (ignored) tests, each wrapped by a script that builds the classpath:
```
scripts/vanilla-commands.sh /tmp/gen-work 40000   # command parsing vs official dispatcher
scripts/vanilla-snbt.sh     /tmp/gen-work 300000  # SNBT parser vs official parser
FERRITE_SNBT_RECORD=$PWD/crates/ferrite-nbt/tests/data/snbt.txt scripts/vanilla-snbt.sh ...  # re-record corpus
FERRITE_ACCEPT_MOJANG_EULA=true scripts/oracle.sh /tmp/jar/server-26.3.jar   # protocol codec vs live server
scripts/vanilla-codec.sh ...                        # codec differential
```
Recording scripts (`scripts/record-*.py`) start a real vanilla server on loopback (offline mode,
`pause-when-empty-seconds=-1`, RCON enabled) and a Ferrite bot (`ferrite-swarm`'s
`ferrite-interact`) and store the answers in `crates/*/tests/data/`. They need the EULA variable too.

## Disk
`target/` grows to tens of GB. Cleaning: `rm -rf target/miri target/release target/doc target/criterion
target/debug/incremental`; stale `target/debug/build` dirs older than a few hours are safe to delete.

## Cloud paths that appear in notes
`/tmp/claude-0/gen-work` = unpacked bundler dir; `/tmp/claude-0/oracle-f/server.jar` and
`/opt/mcdata/oracle2/server.jar` = the bundler jar; `/tmp/claude-0/po`, `/tmp/claude-0/sn` = probe
work dirs (inputs preserved in `docs/handoff/probes/`). Re-create equivalents locally.
