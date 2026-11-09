# 00 — START HERE (paste this into your new local chat)

You are continuing a project that was built in a cloud Claude Code session. Everything you need is in the
`handoff/` folder of this repository. **Read the documents in this order before touching code:**

1. `handoff/01_PROJECT_OVERVIEW.md` — what OpenFPV is, who the user is, what exists, what state it is in.
2. `handoff/02_ARCHITECTURE_AND_CODE_MAP.md` — every source file, data flow, threading model, key design decisions.
3. `handoff/03_BUILD_TEST_ENVIRONMENT.md` — exactly how it is built and tested, including the non-obvious
   workarounds (Forge `Side.BUKKIT` bug, XRandR, resources dir, production smoke tests).
4. `handoff/04_CHAT_HISTORY_AND_DECISIONS.md` — the full conversation: every request, every question asked and
   the user's answers, every bug found and how it was fixed, in order.
5. `handoff/05_REFERENCE_LUA_API_CONFIG_CONTROLS.md` — the complete Lua API, config keys, controls, recipes.
6. `handoff/06_KNOWN_ISSUES_LIMITS_AND_TODO.md` — honest list of what is NOT verified, limits, and a prioritised backlog.
7. `handoff/07_LOCAL_SETUP_GUIDE.md` — how to set the project up on the user's own machine and continue.

## The user

* Name: **Tom Pearl** (email valexapi@gmail.com). Plays Minecraft with the **All the Mods 3 (ATM3)** pack (1.12.2).
* **Language: English.** (He sometimes types German words like "Drohne", "Mod Jahr"; always answer in English.)
* Standing preferences he gave: *never skimp or leave placeholders / half-finished work; install the real SDK/tool
  rather than a thinner workaround; only stub something if >90 % sure it is the right call, otherwise research
  first (web search, read docs/source); no artificial limits on installing/downloading/compiling what the task needs.*
* He wants the mod to be extremely good ("1000x better", "make everything 10000x better"), and said "just keep going
  until done" when asked how to work through large scope. He likes being asked many clarifying questions *before*
  a big build. He asked for Blockbench-style models, sounds, recording, routes, lights, animation — all done (see docs).

## The project in one paragraph

**OpenFPV** is a Minecraft **1.12.2 Forge** mod (Forge 14.23.5.2860) that adds first-person-view cameras to
**OpenComputers (OC)** drones. A *Camera Upgrade* drone upgrade provides a Lua `camera` component; the player
presses **V** to see through the drone and fly it like a real quadcopter (angle mode and acro mode). It has
5 modules (night vision, zoom, gimbal, thermal, depth sensor), 4 antenna tiers (64 → 1500 blocks) with
chunk/entity streaming so you can see far away, control lag/packet loss, return-to-home failsafe, waypoint
routes, FPV goggles, DVR recording + blackbox CSV, an animated Blockbench drone model with props/lights/sounds,
and an in-game config screen. Source lives in `openfpv/` of the git repo `stpdgse/646464747447`, branch
`claude/gallant-edison-yxpv26`, draft **PR #4** (base `claude/session-notes-migmyj`).

## Current state (at handoff)

* 138/138 automated in-game checks pass; 38 JUnit tests pass; release jar builds; production dedicated server and
  production client boot with the release jar. See doc 03 and 06 for exactly what that does and does **not** prove.
* **Never played by a human** in a real ATM3 install. First job after handoff: have Tom drop
  `openfpv-1.12.2-1.0.0.jar` into ATM3's `mods/` and report what happens.

## Rules for continuing (how the previous session worked — keep doing it)

1. **Test everything in the real game.** There is a scripted end-to-end harness (`com.openfpv.dev.DevHarness`,
   run with `scripts/run-selftest.sh`). Add a check there for every new feature; look at the screenshots it writes.
2. **Distrust green checks that you wrote yourself**: several "failures" during development were bugs in the test
   (see doc 04), and several real bugs were only found by looking at screenshots. Verify, don't assume.
3. Keep unit-testable maths in `com.openfpv.util` (no Minecraft classes) with JUnit tests.
4. Commit in small steps with clear messages; push to the same branch; keep the PR description current.
5. Be honest in reports: say what was tested, what was not, and what you could not verify.
6. Do not mention the model name in commits/PRs (an instruction from the original environment).

## First prompt suggestion for the new chat

> "I'm Tom. Read `handoff/00_START_HERE_PROMPT.md` and the other handoff documents, then confirm you understand the
> OpenFPV project. Then run `./scripts/run-selftest.sh` (or tell me what is missing on this machine) and report the
> result. After that I want to continue with: <what you want next>."
