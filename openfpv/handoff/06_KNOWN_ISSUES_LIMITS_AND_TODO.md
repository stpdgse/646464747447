# 06 — Known Issues, Limits, What Is NOT Verified, and the Backlog

## A. What has and has not been verified (read this first)

| Claim | Evidence | Gap |
|---|---|---|
| Mod loads with OC 1.8.10 (dev + production client + production server) | logs, 138-check harness, production smoke tests | **Never loaded in a real ATM3 install** (hundreds of other mods) |
| Lua API behaves as documented | harness drives real Lua on real OC drones; depth() distance matches ground truth to ~0.1 block | table results checked from Lua; some methods only exercised from Java |
| Flight model | harness: pitch ramp, 15 m/s peak, glide, level-out, hover, bank, yaw, acro, throttle | **No human has flown it.** Feel/tuning (`attitudeResponse`, `accelScale`, `drag`) is by numbers + screenshots only |
| Long range (300 and 700 blocks, chunks+entities) | harness in dev env with MCP names | The **obfuscated reflection names** were verified only statically against the SRG jar; this code path was **not run in production** |
| Rendering (HUD, shaders, drone model) | screenshots from llvmpipe (software GL) | Never seen on a real GPU / with shader packs / OptiFine |
| Sounds | registered + MovingSound active in harness; Ogg files decode | **Never heard.** Synthesised; may sound harsh/clippy; no audio device in the sandbox |
| DVR | mp4 + CSV produced, `ffprobe` says h264, ~1 frame/tick | Performance on high resolutions/real GPUs untested (glReadPixels per tick) |
| Multiplayer / dedicated server gameplay | server **boots** with the jar | No client↔server session was ever tested; packet/range logic written for MP but unproven; config values like lag are read client-side |
| Compatibility with other mods | none | e.g. mods that replace the drone renderer, change entity tracking, or alter OC drones |

## B. Known limitations / rough edges

1. **Drone needs a loaded chunk.** If nobody is near and no OC chunkloader upgrade is active, the drone stops/unloads —
   inherent to OC. Documented for players; test drones use `chunkloaderupgrade`.
2. **Signal range ≠ render distance.** Streaming radius is 5 chunks around the drone (config, capped by server view
   distance); far terrain beyond that is fog/void.
3. **LongRangeLink raises tracker ranges of every entity within 96 blocks of the drone** for the pilot's benefit;
   with huge mob counts this costs bandwidth. It restores ranges on release; a server crash mid-session cannot leave
   state behind (ranges are in-memory only).
4. **Speed cap override** (`maxVelocity`, `drag`) changes OC drone behaviour while piloted for *everyone*
   (restored on exit). If two pilots could fly one drone it would conflict — second pilot is refused.
5. **Targets are written directly** into OC's synced DataParameters via reflection; if OC renames `DataTargetX/Y/Z`,
   `maxVelocity`, `drag` the code falls back (coarse flight / no speed raise) and logs a warning — it won't crash.
6. **OC drone `facing()`** is derived from the entity yaw; depth scans and `getHeading()` use it, so while piloted they
   reflect the pilot's yaw (the server sets `rotationYaw` every tick).
7. **Player body not protected** (user's choice). Monsters can hurt the standing pilot.
8. **Goggles have no armor model** (they equip into the helmet slot but render nothing on the player).
9. **Drone model is simple** (34 cubes, flat shading, generated texture). The `.bbmodel` is hand-parameterised by a
   script, not artist-made. Props are boxes, no motion-blur discs.
10. **Thermal vision** is a stylised post effect, not physical.
11. **`en_us` only** (no other languages).
12. **No JEI/NEI integration, no advancements, no `/openfpv` commands.**
13. **Config reload semantics:** most values apply live; `client.customDroneModel` needs a restart; `range.tiers`
    applies live but tooltips read client config.
14. **Control-lag RNG is client-side and seeded** (`20260101L`) → deterministic loss pattern (fine for gameplay).
15. **DVR** captures the *framebuffer* each tick; if the game runs below 20 fps frames are repeated/dropped
    (one capture per game tick, max). Audio is not recorded.
16. **Mod-list "Missing English translation for FML"** and `module-info.class` ASM errors in dev logs are harmless noise.
17. `run/config/openfpv.cfg` of the dev environment may contain obsolete keys; harmless.
18. The harness file is large (1.3k lines) and order-dependent: steps assume prior state (e.g. drone B hovering at its
    start). When adding steps, run the whole scenario; avoid adding long waits mid-scenario (it times out chunk-unloads).

## C. Bugs that were found and fixed (so you don't reintroduce them)

* HUD vignette in `GL_FLAT` → always `GlStateManager.shadeModel(GL_SMOOTH)` before gradients, restore `GL_FLAT` after.
* `EntityTrackerEntry` re-evaluates visibility only when the entity moves >4 blocks → always call `updatePlayerEntity` after
  changing ranges.
* OC target rounding to ¼ block → `DroneAccess`.
* `Drone.start()` (not `machine().start()`) to power on; OC `onItemUse` uses the held stack.
* Route index overrun at completion → clamp to last waypoint.
* Config GUI categories lacked comments → `setCategoryComment/LanguageKey`.
* `CameraSetup` roll sign verified visually (right bank ⇒ horizon rotates counter-clockwise).
* Esc must be intercepted at `GuiOpenEvent` (cancel `GuiIngameMenu`), not via key handling.
* Players must not be able to move the camera item while its bay GUI is open (`slotClick` guard).

## D. Backlog (prioritised suggestions; none started)

**P0 — verify in the real world**
1. Have Tom drop the jar into ATM3 and fly: report crashes, key conflicts (V/B/N/R/arrows), HUD scale, feel.
2. A production-client **FPV** smoke test (not just main menu): launch the production client, create a world, give a
   drone via commands/NBT, fly. Reuse `tools/mklaunch_production_client.py`. Ideal: make the DevHarness runnable against the
   reobfuscated jar (add a second Gradle jar task with a `reobf` entry).
3. Multiplayer test with a real second player on a dedicated server.
4. Listen to the sounds; retune `gen_drone_assets.py` (motor harmonics/noise, beeps) or replace with real recordings.

**P1 — gameplay / realism**
* Remote controller item and/or tablet app; pairing so only the owner can fly.
* Replay of recorded flights (blackbox CSV → ghost drone) and an in-game DVR file browser.
* Waypoint planner GUI (right-click with a "route planner" item; click blocks to add waypoints; show path particles).
* Battery model: voltage sag with throttle, flight-time estimate on the OSD, charging station block.
* Wind/gusts, prop-wash wobble at high descent speed, ground effect, crash damage + drops, propeller guards.
* Real dynamic light from the lights (needs a light-source mod or a tiny block hack) and a landing light beam.
* Camera lens effects: rolling shutter, chromatic aberration, lens dirt, DVR compression artefacts at weak signal.
* FPV goggles armor model; goggle-only analogue video static.
* Payload/cargo drop module, speaker module, follow-me (target an entity/player), orbit mode, altitude hold toggle.
* Acro tuning: separate rates curve (expo), camera angle presets.

**P2 — polish / engineering**
* Better Blockbench model (hand-made, animated bones; maybe separate models for the camera pod and gimbal), proper
  texture painting, LOD.
* Localisation files; JEI recipe categories; advancements; `/openfpv` debug command; creative-tab ordering.
* Replace reflection in `LongRangeLink` with Access Transformers (`accessTransformer` in `build.gradle`) — safer/clearer
  once verified that FG3 remaps MCP names in AT files for 1.12.2.
* Consider RetroFuturaGradle to avoid the FG3 Side.BUKKIT workaround.
* CI: GitHub Actions running JDK 8 `./gradlew build` + unit tests (the end-to-end test needs Xvfb/Mesa, doable on `ubuntu` runners).
* Split `DevHarness` into scenario classes; add a "quick" scenario (~2 min) for faster iteration.
* Make `DroneRenderer` also draw for *non-OC* drone entities if other mods use the same class? (not needed now).

## E. Possible risks when merging PR #4
* Base branch is `claude/session-notes-migmyj` (PR #1). Retarget to `main` after PR #1 merges, or rebase.
* The repo has unrelated content; the project is self-contained under `openfpv/`.
