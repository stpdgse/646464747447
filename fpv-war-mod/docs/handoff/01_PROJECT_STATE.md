# Project state (end of Phase 7)

Branch `claude/elegant-pascal-hr9x9b` (draft PR). Last commit: `ae508c4 Phase 7: warzone dimension, data-driven missions...`.
Counts: 99 main Java files, 14 test files, 55 GameTests, 7 unit-test classes groups (`sim`, `input`, `destruction`, `build`, `ai`, `art`, `mission`).

## Phase log
| Phase | Delivered | Key files |
|---|---|---|
| 0 | Gradle/NeoForge skeleton, GameTest harness, headless server + client smoke scripts | `build.gradle`, `scripts/*.sh` |
| 1 | Flight sim (pure Java, 80 Hz substeps, rates/expo, angle/mixed/acro), drone entity + item, pilot networking, keyboard + SDL gamepad input, FPV camera, OSD, GeckoLib drone model | `sim/`, `drone/DroneEntity`, `input/`, `net/`, `client/Fpv*` |
| 2 | Destruction: block blast solver (ray-based), structural support solver, async CollapseManager (budgeted per tick), warhead drones (impact, shaped), quality presets | `destruction/`, `destruction/core/` |
| 3 | Battery (sag, drain), radio link (distance, blocks, jammers, failsafe, dropped packets), jammer entity, analog video degradation (post-effect), training range, warnings | `sim/Battery`, `sim/RadioLink`, `sim/LinkMonitor`, `drone/JammerEntity`, `client/FpvPostEffects`, `drone/TrainingRange` |
| 4 | Crafting: 7 tiered parts (frame, motors, props, battery, flight controller, camera, antenna) x3 tiers + warheads + blackbox, Drone Workbench block, assembly recipe, drone build component, flight log blackbox | `build/`, `item/`, `scripts/gen_assets.py`, `docs/CRAFTING.md`, `docs/RECIPES.md` |
| 5 | Art: toolchain (`art/`), rifleman (44 cubes, held rifle, IK-solved poses, 14 animations), tank (34 cubes, 5 animations, wreck texture), drone hero model (37 cubes), entities + renderers | `art/`, `drone/SoldierEntity`, `drone/TankEntity`, `client/*Renderer` |
| 6 | Enemy AI: fear model, roles, cover/flee/engage, marksmanship, shoot-down, blast alarm, ally-death fear, tank traverse and fire | `ai/`, `drone/SoldierEntity`, `drone/TankEntity`, `drone/Threats` |
| 7 | Warzone dimension, mission framework (JSON), MissionManager, Outpost builder, commands | `mission/`, `data/.../dimension`, `data/.../missions` |

## Commands (`/kamikaze ...`, operator level)
`training` (builds a practice range + gives a drone) - `soldier [stance|role]` - `tank` - `warzone` (teleport) - `mission list|start <id>|status|abort`.

## Config (`kamikaze_skies-*.toml`, `KsConfig`)
`flightSubsteps` (4), `crashSpeed` (9 m/s), `destructionQuality` (LOW/MEDIUM/HIGH, default HIGH), `detonationSpeed` (3 m/s), `linkRangeBlocks` (400), `batteryEnabled` (true).
**Lower `destructionQuality` first if the GTX 1660 struggles during collapses.**

## Content inventory
- Entities: `fpv_drone`, `jammer`, `soldier` (3 roles), `tank`.
- Items: drone (with `drone_build` data component), jammer, 25 part items, workbench block.
- Models (GeckoLib, in `assets/kamikaze_skies/geckolib/`): `fpv_drone` (+spin anim), `jammer`, `rifleman` (+14 anims), `tank` (+5 anims, wreck texture).
- Missions: `outpost_alpha`.
- Dimension: `kamikaze_skies:warzone` (overworld type, noise generator, fixed plains biome). Vanilla shows a one-time "Experimental Settings" prompt for worlds with it.

## Verified vs unverified
| Verified (cloud, software rendering / headless) | NOT verified (needs real hardware) |
|---|---|
| All unit tests, 55 GameTests, dedicated server boot | **Frame rate** (target 60 FPS on RTX 3060 / R7 5700X; this PC is weaker: i5-9400F + GTX 1660) |
| Client launches, renders every model, FPV post-effect, OSD, animations, demo flights, mission start | **Gamepad** with real hardware (SDL3 init OK, mapping unit-tested) |
| Collapse and blast visuals via screenshots / GIFs | **Audio**: there is NO real sound yet; vanilla stand-ins only, and the cloud had no audio device |
| Crafting recipes via data/GameTest | Workbench GUI hand-clicking, shader packs (Iris), long-session stability, multi-minute collapses |

## Screenshots / GIFs (in `docs/images/`)
Phase 1 FPV demo, Phase 2 strike GIF, Phase 3 link loss GIF, Phase 4 parts, Phase 5 soldiers GIF + tank GIF + drone, Phase 6 AI GIF, Phase 7 mission GIF.
