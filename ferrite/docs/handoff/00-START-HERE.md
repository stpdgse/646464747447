# Ferrite handoff: start here

Prepared at the end of a long cloud session. Everything needed to continue locally is in this
zip: the full source tree (including git history), these documents, the Java oracle harnesses,
the probe inputs and the helper scripts. **No Mojang files are included** (see 01).

## What Ferrite is
A from-scratch Minecraft: Java Edition **26.3** (protocol 777, world data version 5023) server
in Rust, goal: as fast and multicore as possible, bit-exact with vanilla wherever observable.
Owner: Tom Pearl. Standing instructions from Tom (keep following them):
- Speak English. "Dont simplify anything, I don't care how long it takes", no placeholders or
  half-finished work; install real tools; research (web/docs/source) when under ~90% sure.
- Full vanilla parity, region-parallel ECS, tokio (default) + io_uring, Linux + Windows,
  online + offline auth, custom world format, exact worldgen later, game data generated from the
  official jar, pinned nightly Rust, workspace of small crates, MIT OR Apache-2.0, **clean room**
  (never decompile or copy Mojang code; learn behaviour by black-box probing the real server).
- Give short progress updates, do not stop for small things, when the disk fills delete caches
  (`target/miri`, `target/release`, old `target/debug/build`) and keep going.
- Mojang EULA was accepted by Tom **for local oracle testing only** (scratch dir, loopback,
  offline mode, never committed). Never accept it on someone else's behalf.

## Read in this order
1. `01-SETUP-LOCAL.md`: install toolchain/JDK, get the jar, build, run every test.
2. `02-ARCHITECTURE-AND-STATUS.md`: crates, what is done, what is verified how.
3. `03-ORACLE-METHOD.md`: THE core technique (differential testing against the real server).
4. `04-COMMANDS-AND-SNBT-DEEP-DIVE.md`: everything learned about Brigadier, SNBT, items, blocks.
5. `05-BUGS-FIXES-GOTCHAS.md`: every bug hit and how it was fixed; traps to avoid.
6. `06-CURRENT-STATE-AND-NEXT-STEPS.md`: the exact point the work stopped and a prioritized plan.
7. `07-FILE-MAP-AND-TOOLS.md`: where things live, what each script/harness does.

Branch: `claude/epic-cori-fsfcww`, draft PR #5 in `stpdgse/646464747447`. Last commit when this
was packed: see `git log -1`. CI (workflow `.github/workflows/ferrite.yml` at the repo root, one
level above `ferrite/`) was green on the commit before the last two (CI for those was pending).
