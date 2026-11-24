# Architecture and status

The README (`ferrite/README.md`, 634 lines) is the authoritative per-subsystem description;
this file is the map plus an honest done / not-done ledger.

## Crates (all under `ferrite/crates/`)
| crate | role |
|---|---|
| ferrite-data | Generated tables (registries, 1,286 blocks / 35,723 states, packet ids, items, entities, loot, shapes, commands, timelines, **lang (en_us, new)**). Never hand-edit `generated/`. |
| ferrite-datagen | Runs the vanilla data generator + Java dumpers (`java/*.java`) and writes `ferrite-data/src/generated`. Reproducible; CI diffs it. |
| ferrite-nbt | NBT read/write (network + named), quota/depth limits, **SNBT writer and (new) SNBT parser** (`parse.rs`), `List::Mixed` variant. |
| ferrite-protocol | Packet codecs, VarInt, framing, zlib, AES-CFB8, sans-IO `ServerSession`. |
| ferrite-net | `Lobby` policy + tokio backend + io_uring backend. |
| ferrite-swarm | Bot load tester and `ferrite-interact` (scripted bot used by the recording scripts). |
| ferrite-remote | Query + RCON servers/clients. |
| ferrite-ecs | Archetype ECS, `forbid(unsafe_code)`, deterministic. |
| ferrite-sim | Region scheduler (rayon), deterministic exchange. |
| ferrite-world | Paletted chunks, zstd codec, crash-safe log-structured store. |
| ferrite-blocks | Block physics: updates, fluids, redstone, pistons, placement rules, shapes. Verified vs recordings. |
| ferrite-crafting | Recipes / crafting grid logic. |
| ferrite-loot | Loot table evaluation. |
| ferrite-text | Text components (bare string vs compound encoding, mixed-list wrapping). |
| ferrite-commands | Brigadier port, argument parsers, command tree, dispatcher parse. **New: blockstate.rs, nbt.rs, item.rs, components.rs.** |
| ferrite-game | The 20 TPS single-writer game thread: players, chunks, entities, mobs, containers, furnaces, commands (server/commands.rs), tick manager, persistence. |
| ferrite-server | The executable. |

## Done (and how each was verified)
- Handshake/status/login/configuration, online + offline auth, Query, RCON (vs live vanilla).
- Play state: join sequence (captured from vanilla), chunk streaming, entity tracking, movement,
  chat, block break/place, survival mining + drops + pickup, inventory clicks (replayed vs
  recordings), crafting table, chests/barrels, health/hunger/fall damage/eating/death/respawn,
  passive mobs (movement maths from source knowledge, NOT yet compared with vanilla), furnaces
  (600+400 recorded scenarios, window clicks recorded), saving.
- Block physics incl. redstone/pistons/water/lava: recordings of vanilla.
- Commands: tree per permission level (CommandDump), Brigadier port, ALL argument types except
  those in the "not done" list; 139 recorded command scenarios replayed over a real connection
  (`crates/ferrite-game/tests/commands.rs`); implemented commands: gamemode, difficulty, list,
  seed, say, me, msg/tell/w, teammsg, teleport/tp, time (26.x clocks/timelines), tick.
- Parsing differential test: ~40k generated lines per run, identical tree path / error+cursor.
- **SNBT parser (new): 300,000 generated lines (600,000 answers) identical to vanilla**; 4000-case
  recorded corpus replayed in CI (`crates/ferrite-nbt/tests/snbt_replay.rs`).
- Block state / predicate args (new): exact on ~30k lines. Block-entity `{..}` and selector
  `nbt=`, `nbt_tag`/`nbt_compound_tag` args (new): exact.

## Not done / partial
- Item stack arg (new, uncommitted work is committed but PARTIAL): syntax, unknown/repeated/
  malformed plumbing exact; component *value validation* only for 8 components
  (`components::VALIDATED`). The other ~110 accept anything. 3/39,722 fuzz lines differ
  (custom_data string errors, see 06).
- item_predicate (`[..]` conditions, `~`, `|`, `(`, `count`, `*`, `#tag`), text components
  (JSON + SNBT), nbt_path, particle, resource_or_tag variants, loot_predicate, etc.: argument
  parsers still return `ferrite.unsupported` (the differential test skips them and the game
  tells players honestly).
- Commands not implemented (92 vanilla commands show in the tree): setblock, fill, give, clear,
  summon, kill, weather, xp, defaultgamemode, setworldspawn/spawnpoint, op/deop, kick, stop,
  save-all, gamerule, effect, tellraw, title, data, execute, scoreboard, ... (see 06 for order).
- Mobs: no hostile mobs, breeding, mob persistence; passive-mob behaviour unverified vs vanilla.
- No lighting engine, no worldgen (custom flat/simple), no dimensions/portals, no villagers,
  redstone is verified but not all block entities exist, no advancements/loot via datapacks,
  no signed chat (play state), io_uring play-state path not built, benchmarks vs Paper etc. not run.
- Player XP goes direct (no orbs), no recipe book.
