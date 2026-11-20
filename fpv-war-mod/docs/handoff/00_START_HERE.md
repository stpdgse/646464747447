# Kamikaze Skies - Handoff (start here)

Owner: **Tom Pearl**. Private passion project, All Rights Reserved. Working title **Kamikaze Skies** (mod id `kamikaze_skies`).
A NeoForge mod for **Minecraft Java 26.3** (NeoForge `26.3.0.114`, **Java 25**, **GeckoLib 5.5.7**): fly realistic-but-fictional FPV drones
(keyboard, gamepad, later touch) into a gritty battlefield, destroy vehicles and structures, and face enemies that panic, hide and shoot back.

This zip was made on 2026-11-20 so work can continue on Tom's **second PC (Intel i5-9400F, 32 GB RAM, GTX 1660)**. The project was built entirely in a
cloud Linux container with **no GPU, no audio, no gamepad and no real display**, so everything that needs real hardware is still unverified.
That is the main reason to move: see `08_REAL_PC_VERIFICATION.md`.

## Read in this order
1. `01_PROJECT_STATE.md` - what exists, phase by phase, with file map and numbers (tests, commands, content).
2. `02_ARCHITECTURE.md` - packages, how the pieces talk, conventions you must keep.
3. `04_WINDOWS_SETUP.md` - get it running on the PC in about 20 minutes.
4. `08_REAL_PC_VERIFICATION.md` - the first things to do on the real machine (frame rate, gamepad, sound, GUI clicks).
5. `03_ROADMAP.md` - the remaining plan (Phase 8 onward) in actionable detail, plus a backlog.
6. `05_ART_PIPELINE.md` - how models, textures and animations are produced (code-generated, previewed in Blockbench).
7. `06_DECISIONS_AND_PREFERENCES.md` - every decision Tom made and how he likes to work. **Follow these.**
8. `07_PITFALLS.md` - bugs and traps already hit; saves hours.
9. `09_SESSION_PROMPT.md` - a ready prompt to paste into a new Claude Code session on the PC.

Existing docs in `docs/` (DESIGN, CONTROLS, DESTRUCTION, LINK_AND_BATTERY, CRAFTING, RECIPES, AI, MISSIONS, ART_PIPELINE, TESTING) stay the technical source of truth;
the handoff files summarise and link to them. `DESIGN.md` is the scope document: change it (with a dated changelog line) when a decision changes.

## One-paragraph status
Phases 0-7 are done: flight sim, FPV camera/OSD/analog video, destruction (blast + structural collapse), battery and radio link with jammers, crafting and
drone assembly, art vertical slice (rifleman with 14 animations, tank with 5, redone drone), enemy AI (fear/roles/cover/flee/engage, tank traverse), a warzone
dimension and a data-driven mission framework with the first mission (Outpost Alpha). 55 GameTests + 60-odd unit tests pass headless. Not done: real audio,
touch controls, briefing UI, more missions/content, multiplayer, performance tuning on real hardware.

## Golden rules (from Tom)
- Never leave placeholders or half-finished work; install the real tool instead of a thin workaround. Below ~90 % confidence, research first.
- Speak English. Call him Tom.
- Big chunks, then a short milestone summary with screenshots/GIFs. Keep scripts hard-capped and fail-fast (he was burned by hangs).
- War-game content stays (fictional, no real insignia/conflict, no real-world politics). He explicitly said keep it.
- Commit and push work to the working branch; open a draft PR; do not leave uncommitted work.
