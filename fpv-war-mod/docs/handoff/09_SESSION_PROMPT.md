# Prompt to paste into a new Claude Code session on the PC

```
You are continuing the Minecraft mod "Kamikaze Skies" for Tom Pearl (NeoForge 26.3.0.114, Minecraft 26.3, Java 25, GeckoLib 5.5.7).
Project root: the folder containing fpv-war-mod/ (and tools/blockbench/). First read, in order:
fpv-war-mod/CLAUDE.md, fpv-war-mod/docs/handoff/00_START_HERE.md, then 01..09 in that folder, then docs/DESIGN.md and the docs it links.
Phases 0-7 are done (flight sim, FPV view, destruction, battery/link/jammers, crafting, art slice, enemy AI, warzone dimension + missions).
This machine is a real PC (Windows, i5-9400F, 32 GB, GTX 1660) with a GPU, display and audio, unlike the cloud container that built everything.
Step 1: follow 04_WINDOWS_SETUP.md and 08_REAL_PC_VERIFICATION.md: build, run all tests, run the client, measure FPS, test gamepad, report results to Tom, fix what you find.
Step 2: continue with Phase 8 in 03_ROADMAP.md (audio first, then briefing UI, touch controls, more missions, AA/EW defences, terrain polish, gore).
Rules: never leave placeholders or half-finished work; install real tools; research when under 90% sure; speak English; call him Tom; keep war content fictional (no real insignia/conflict);
logic goes in pure-Java cores with unit tests, glue gets GameTests (one solo environment each), visual features get a demo and a real look; every script that launches the game has a hard cap;
big chunks of work then a short milestone summary; commit and push regularly.
```
