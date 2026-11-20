# Testing

Everything below runs in the cloud container and in CI. Every script that launches Minecraft has a **hard time cap** and
**fails fast** on fatal log lines, so nothing can hang.

## Toolchain
- Java 25 (Temurin) — Minecraft 26.x requires it. Gradle wrapper 9.2.1, NeoForge `26.3.0.114`, GeckoLib `5.5.7`.
- Locally: `export JAVA_HOME=/opt/jdk-25 PATH=/opt/jdk-25/bin:$PATH`.
- Behind the container proxy, pass `--max-workers=1 -Dorg.gradle.parallel=false` to avoid Maven 429s.

## Levels
| Level | Command | What it proves |
|---|---|---|
| Unit tests (pure Java `sim/`) | `./gradlew test` | Flight/battery/link/morale math, no Minecraft |
| GameTests (headless server) | `./gradlew runGameTestServer` | Mod loads, registries, entities, world logic. Exits non-zero on failure |
| Dedicated server boot | `scripts/server-smoke.sh [cap_s]` | Real server starts with mod + GeckoLib and reaches `Done` |
| Real client screenshot | `scripts/client-screenshot.sh [out.png] [cap_s]` | Real client reaches the UI; screenshot saved |
| Demo world (once) | `scripts/make-demo-world.sh` | Flat peaceful world at `run/saves/DemoWorld` (made with the dedicated server) |
| Scripted FPV flight | `scripts/demo-screenshots.sh [dir] [cap_s] [shots] [every_s]` | Real client joins the world, rides a scripted flight; screenshots of the FPV view + OSD |
| Drone model in world | `KS_DEMO_MODE=model scripts/demo-screenshots.sh dir 150 3 3` | The GeckoLib model renders at the right scale with animated props |

New GameTests: add a function to `gametest/KsGameTests.java` and a JSON in `data/kamikaze_skies/test_instance/`.
Check a new test can fail by temporarily breaking its assertion.

## Client rendering in the cloud (no GPU)
- Minecraft 26.x tries OpenGL, then Vulkan. Xvfb has no usable GLX visual for the game's OpenGL window
  (`Couldn't find matching GLX visual` — expected), so the **Vulkan backend runs on lavapipe** (software):
  `VK_ICD_FILENAMES=/opt/lavapipe/lvp_icd.json`.
- No audio device: `ALSOFT_DRIVERS=null` gives OpenAL a null device.
- Xvfb needs `+extension GLX +render -noreset` and `XDG_RUNTIME_DIR` set.
- The FML early loading window is disabled in the dev run config (`run/config/fml.toml`: `earlyWindowControl=false`).
- Frame rates are very low; screenshots verify look and behavior, **not performance**. Real FPS is checked on Tom's PC.

## Gotchas learned
- `ModConfig.Type` in 26.x is `LOCAL | CLIENT | SYNCED | STARTUP` (no `COMMON`). `SYNCED` values are per-world and cannot be read in `FMLCommonSetupEvent`.
- GameTests are registered in the `TEST_FUNCTION` registry and referenced by JSON test instances; the structure `minecraft:empty` is built in.
- `gradlew runServer` does not forward stdin, so you cannot type `stop`; the smoke script stops it by signal.

## Phase 1 lessons
- **GameTests must not depend on wall-clock ticks.** The 1x1 test structure only keeps its own chunk ticking, so an entity in
  a neighbouring chunk can start ticking late. Tests force-load a 5x5 chunk area and wait for `drone.simTicks()` instead of
  `runAfterDelay` (`afterTicks(...)` in `KsGameTests`). Before that fix the same suite passed or failed at random.
- **Always check a "went nowhere" failure's assumptions with logging first.** A wall test failed because the drone flew *over*
  the wall (excess throttle), not through it; the debug log showed the collision at exactly the expected distance.
- **Headless client prerequisites** (set automatically by the scripts in `run/options.txt`): `onboardAccessibility:false`
  (the first-launch Narrator screen blocks auto-join), `tutorialStep:none`, and `pauseOnLostFocus:false` (an unfocused Xvfb
  window would otherwise pause the integrated server).
- **GeckoLib renderer generics:** GeckoLib creates a plain `EntityRenderState` and a mixin makes it a `GeoRenderState`. A
  custom renderer-state subclass causes a `ClassCastException` through javac's bridge methods; use
  `GeoEntityRenderer<E, EntityRenderState>` and a *raw* `RenderPassInfo` override (see `DroneRenderer`).
- **Model pipeline:** `tools/blockbench` generates a `.bbmodel`, the headless Blockbench exports Bedrock geometry, and it is
  copied to `assets/kamikaze_skies/geckolib/models/entity/*.geo.json` (animations under `geckolib/animations/entity/`,
  textures under `textures/entity/`). GeckoLib logs `Loaded N models and N animations` on success.
- Gamepads come from SDL3 (LWJGL is on the classpath; Minecraft 26 has no GLFW).

## Phase 2 lessons (destruction)
- **GameTest ticks run faster than real time**, while force-loaded chunks load in real time. A test that waits a fixed number
  of ticks for a chunk to start ticking can time out. Use generous `max_ticks` (6000) for tests that spawn entities; passing
  tests still finish immediately.
- **Neighbouring GameTests contaminate each other** when the code under test looks tens of blocks around (stability
  analysis). Give those tests their own `test_environment` (so they run one at a time) and a large `padding` in the test
  instance JSON (100 blocks).
- **Assert "at least", not "exactly", when other tests may leave blocks nearby**, and prefer checks tied to your own blocks.
- **A failing assertion should say what it saw.** Putting the offending cells and block types in the failure message turned a
  mysterious flake into an obvious bug (vanilla's `#dirt` tag no longer contains grass blocks, so grass was classified as a
  structure).
- Vanilla tag contents change between versions; check them in the jar's `data/minecraft/tags/` before relying on them.
- `scripts/demo-screenshots.sh` takes `KS_DEMO_MODE=strike`, `KS_START_DELAY=0` and a fractional interval for dense frames,
  e.g. `KS_DEMO_MODE=strike KS_START_DELAY=0 scripts/demo-screenshots.sh build/screenshots/strike 150 60 0.25`.

## Phase 3 lessons (battery, link, video)
- **A model change can silently break old tests.** Adding the radio link made the Phase 1 `pilot_flow` test fail, because its mock
  pilot stood far from the drone and the link correctly failed. Tests that bind a pilot must place the pilot next to the drone.
- **Check a value at the moment you mean to**, not after the world has moved on: the jammer failsafe test measured the signal
  after the disarmed drone had fallen out of the jammer's field. Assert soon after the event (`afterTicks`), or tie the check
  to a state flag.
- **Visual features need a real-client look.** The shader compiled and loaded without any log error but did nothing at first:
  `GameRenderer` rebuilds its post-effect list every frame, wiping a per-tick addition. Only the screenshot revealed it; the
  fix was `LocalPlayer.setActivePostEffects`.
- **Demos mutate the world.** The strike demo's cottage was still standing in the saved demo world and the next demo flew into
  it. The demo script now restores `DemoWorld.base` before every run.
- **Slow-motion makes fast events capturable** in software rendering (~1 frame per real second): demos set 5-10 TPS.
- Colour blocks are a `ColorCollection` in this Minecraft version: `Blocks.CONCRETE.pick(DyeColor.RED)`.

## Phase 4 lessons (crafting and builds)
- **Use the object's own parameters in tests.** The blackbox-detonation test computed a throttle from one drone's thrust curve and fed
  it to a heavier build: the drone over-throttled and flew over the wall. A build changes the throttle curve, so take it from
  `drone.sim().params()`.
- **Test the data, not just the code.** `part_recipes_craft_from_raw_materials` asks the loaded `RecipeManager` for every generated
  recipe key and crafts a few from vanilla items; a typo in the generator's JSON would otherwise only show up in a player's hands.
- **Sweep combinatorial content.** `DroneBuildTest` visits all 13,122 builds and asserts flyability and sane flight time; it caught
  nothing, which is the point of keeping it as tables are tuned.
- Show real item/block rendering with the `parts` demo: `KS_DEMO_MODE=parts KS_START_DELAY=1 scripts/demo-screenshots.sh build/screenshots/parts 200 16 1.2`.
- Colour/loot/blockstate/model JSON formats for 26.x: copy a vanilla example out of the jar (`data/minecraft/...`) before writing your own.
