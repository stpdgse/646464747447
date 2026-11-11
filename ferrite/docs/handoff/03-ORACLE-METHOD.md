# The oracle method (how nearly everything was verified)

Rule: never guess vanilla behaviour. Ask the real server (or its libraries) and record the answer.
Never read decompiled source (clean-room); inspect only observable behaviour, registries, data
reports and `javap -public` signatures when needed (signatures, not bodies).

Four flavours, all used:

1. **Library harnesses** (fast, no server needed). Java programs compiled against the jar's
   libraries and run headless after `Bootstrap.bootStrap()`:
   - `crates/ferrite-commands/java/ParseOracle.java`: feeds `"<level> <line>"` to the real
     `CommandDispatcher`, prints `OK nodes` or `ERR cursor message`.
   - `crates/ferrite-nbt/java/SnbtOracle.java`: real `TagParser`; prints canonical type-tagged
     values and errors (`key\u0001arg...`) for BOTH `parseFully` and `parseAsArgument`.
   - `crates/ferrite-datagen/java/{CommandDump,ItemDump,EntityDump,ShapeDump}.java`: dumps data.
   - `docs/handoff/tools/CompDump.java`: lists data component types + codec toString (toString is
     only shallowly structural, many codecs are opaque `EncoderCache`; don't count on it).
   Run pattern: `javac -d out -cp "$CP" X.java && java -cp "out:$CP" X in.txt out.txt` where
   `CP=$(find gen-work/versions gen-work/libraries -name '*.jar' | sort | tr '\n' ':')`.
2. **Generated differential fuzz** (`tests/vanilla_parse.rs`, `tests/vanilla_snbt.rs`): a Rust
   generator makes tens of thousands of valid/invalid lines, runs both implementations, prints the
   first 40 mismatches. Iterate: fix a class of mismatch, rerun with a bigger count/another seed.
3. **Record-and-replay against a live vanilla server** (`scripts/record-*.py`): start vanilla in a
   temp dir (frozen ticks via `/tick freeze`, RCON, a scripted bot `ferrite-interact`), execute a
   scenario, store inputs and observed packets/messages in `crates/*/tests/data/*.txt`. Normal
   `cargo test` replays them against Ferrite with no jar. Record AFTER writing the model when you
   can (the second furnace batch caught model errors).
4. **Probe batches**: write a text file of lines, run the harness, `paste -d'|' in out`. All probe
   inputs used this session are in `docs/handoff/probes/` (not curated, but useful seeds).

Useful facts about the servers:
- Properties: `online-mode=false`, `server-ip=127.0.0.1`, `pause-when-empty-seconds=-1` (else the
  tick loop stalls with no players), `allow-flight=true`, whitelist off, RCON on.
- `/tick freeze` + `/tick step N` makes timing deterministic. `time set 54` at scenario start
  removes cross-scenario state leakage.
- Masked values in recordings: game time, look_at entity ids, list-order events.
- Brigadier errors cursor semantics: UTF-16 units; `-1` means "no position" (repeated-component
  and similar exceptions created without context).

Mismatch triage habit that worked: print vanilla vs ours side by side, form a hypothesis about the
grammar/rule, write 20-50 targeted probes, confirm, then implement. When a hypothesis about the
packrat error model failed, fall back to "emulate observed behaviour" + fuzz to find residue.
