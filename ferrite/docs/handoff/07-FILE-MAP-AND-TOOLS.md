# File map and tools

## Scripts (`ferrite/scripts/`)
fetch-server-jar.sh (download+verify jar) | oracle.sh (protocol codec vs live server) |
vanilla-commands.sh (parse differential) | vanilla-snbt.sh (SNBT differential, `FERRITE_SNBT_RECORD`) |
vanilla-codec.sh | record-commands.py (+ command-scenarios.txt) | record-furnace.py |
record-furnace-windows.py | record-inventory.py | record-windows.py | record-crafting.py |
record-placement.py + gen-placement.py | record-physics.py | bench-net.sh.

## Recorded data (`crates/*/tests/data/`)
commands.txt (139 scenarios), furnace.txt, furnace2.txt, furnace_windows.txt, snbt.txt (4000 cases),
placement/inventory/crafting/physics recordings. Re-record only with the exact scripts above.

## Java harnesses
ferrite-commands/java/ParseOracle.java; ferrite-nbt/java/SnbtOracle.java;
ferrite-datagen/java/{CommandDump,ItemDump,EntityDump,ShapeDump}.java; ferrite-game/java/DecodePackets.java;
docs/handoff/tools/CompDump.java.

## Handoff extras (`docs/handoff/`)
- `tools/`: probe.py, pprobe.py, rc.py, fur.py (ad-hoc RCON/world probe scripts from the cloud session;
  they import `mc.py` (RCON client + anvil chunk reader) and hard-code `/tmp` paths: adapt).
- `probes/in*.txt`: raw probe inputs (SNBT: in.txt..in9.txt from the SNBT session are in the same dir
  as command probes; names collide by design: files from `sn/` and `po/` were copied together, so
  `in.txt` etc. may be a mix. Treat as seeds, not documentation).

## Key source files to read first
- ferrite-game/src/server.rs, server/commands.rs (command handlers, `select_players`, tick manager),
  server/{furnaces,containers,mobs}.rs, persist.rs
- ferrite-commands/src/{reader,args,dispatch,tree,selector,blockstate,nbt,item,components}.rs
- ferrite-nbt/src/parse.rs (the SNBT parser; read the module doc + `Parser::value`, `call`, `int_syntax`)
- ferrite-text/src/lib.rs, ferrite-data/src/lang.rs
- tests: ferrite-commands/tests/vanilla_parse.rs (generator for fuzz), ferrite-game/tests/commands.rs
