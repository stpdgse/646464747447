# Where the work stopped, and what to do next

## Exact stopping point
Branch `claude/epic-cori-fsfcww`, last commit "Item stack arguments (partial component validation) and
the generated English language table". Working tree was clean and builds + clippy clean at pack time.
Task in flight: **#14 Commands** - making argument types parse exactly like vanilla so the commands that
use them (`give`, `clear`, `setblock`, `fill`, `summon`, `tellraw`, `data`, `execute`...) can be implemented.

Open mismatch (3 of 39,722 fuzz lines): `give @s x[custom_data=<non-compound string>]`. Vanilla message
second half is the English rendering of the SNBT exception (see 04, "Item stacks"). To fix:
1. `ferrite_data::lang::render(key, args)` already exists (generated `lang.rs`, `EN_US` sorted slice).
2. In `components.rs` for `custom_data` strings: run `ferrite_nbt::parse::parse(text)`; on `Err(e)` build
   `render(e.key, e.args) + " at position {cursor}: " + context(text, cursor)` (context = optional `...`
   if cursor>10, then up to 10 UTF-16 units before cursor, then `<--[HERE]`); trailing data uses key
   `argument.nbt.trailing` (renders "Unexpected trailing data"); when it parses to a non-compound:
   `Expected compound tag, got <java Tag.toString>` (e.g. `1b` for bool(1), strings printed quoted).
   Java `Tag.toString` quoting picks the quote char minimising escapes: check `ferrite_nbt::snbt::to_snbt`
   against `SnbtOracle` outputs before trusting it (it was written for another purpose).
3. Rerun `scripts/vanilla-commands.sh <gen-work> 40000`; `argument.literal.incorrect` renders
   "Expected literal %s" etc. (arguments exactly as stored).
Probe files for this are `docs/handoff/probes/in5.txt` (+ `ParseOracle` usage in 03).

## Suggested order (highest value first)
1. Finish the above, commit, push, check CI (datagen-is-reproducible must stay green; the new
   `lang.rs` is generated, so run `cargo datagen` and confirm `git diff` is empty).
2. **Implement the commands whose arguments are now parseable**, each recorded from vanilla first
   (`scripts/record-commands.py` + `scripts/command-scenarios.txt`, replay in `tests/commands.rs`):
   `defaultgamemode`, `kill`, `weather`, `xp`, `setworldspawn`/`spawnpoint`, `op`/`deop` (ops.json),
   `kick`, `stop`, `save-all`/`save-on`/`save-off`, `gamerule`, `give`, `clear`, `setblock`, `fill`,
   `summon`, `effect`. Output texts are exact: use the recorder, do not guess.
3. `item_predicate` argument (probe: `[a=b|c]`, `~`, `!`, `(...)`, `count`, `#tag`, `*`; errors are
   `argument.id.invalid`, `arguments.item.predicate.malformed [id, reason]`, etc., see probes/in.txt),
   text components (JSON via Gson-like lenient parser + SNBT; `component` and `style` args), `nbt_path`,
   `particle`, `resource_or_tag*`, `loot_predicate`, `item_slot(s)`, `scoreboard_slot`, etc.
4. Component codec port: do it data-driven. Suggestion: a small Rust codec framework mirroring DFU
   decode-from-NBT with its exact message composition (`Not a number`, `Not a map: X`, `Failed to parse
   either. First: ...; Second: ...`, `No key K in MapLike[{...}]` joined by `; `, `... missed input: {...}`),
   then define each component declaratively. Verify each with a probe batch (names list: run
   `tools/CompDump.java`). Update `VALIDATED` + the README honesty note as you go.
5. `execute` (large), entity selection for mobs/items, `tellraw`/`title`/`data`.
6. Signed chat, hostile mobs and the rest of the older backlog: lighting, worldgen (seed-exact),
   dimensions/portals, villagers, advancements/datapacks/functions, io_uring play state, benchmark
   suite against Paper/Folia/Pumpkin/Minestom, PGO/LTO release pipeline.

## Standing process
Commit + push after each logical step (hook enforces), keep PR #5 description current, respond to CI
events (fix failures; never skip/quarantine tests), commit trailers:
`Co-Authored-By: Claude ... <noreply@anthropic.com>` + `Claude-Session: ...` were used in the cloud;
use your own attribution locally. Update README sections (Commands list, "not parsed yet" list) with
each change; claims in the README must stay honest.
