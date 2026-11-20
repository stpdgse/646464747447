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
