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
