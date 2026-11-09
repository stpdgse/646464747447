# 03 — Build, Test and Environment (how it was actually done)

## 1. Build

Prerequisites: **JDK 8** (Temurin 8u512 was used), internet access to `maven.minecraftforge.net`,
`services.gradle.org`, `maven.cil.li`, Mojang hosts. The Gradle wrapper (4.9) downloads itself.

```bash
cd openfpv
./scripts/fetch-deps.sh                       # libs/OpenComputers-MC1.12.2-1.7.5.221-api.jar + libs/OpenComputers-MC1.12.2-1.8.10.jar
export JAVA_HOME=/path/to/jdk8
./gradlew build                               # compiles, runs 38 unit tests, makes + reobfuscates build/libs/openfpv-1.12.2-1.0.0.jar
```
* First run decompiles/deobfuscates Minecraft (~1–2 min, needs ~3 GB heap: `org.gradle.jvmargs=-Xmx3G`).
* `compileOnly` OC **API** jar only — the mod never links against OC internals at compile time (OC internals are used
  by reflection: `DroneAccess`, `DroneRenderer.running/lightColor`, harness).
* `jar { exclude 'com/openfpv/dev/**'; finalizedBy reobfJar }`; encoding forced to UTF-8 (`tasks.withType(JavaCompile)`).
* `build.gradle` special lines: `sourceSets.main.output.resourcesDir = sourceSets.main.java.outputDir` (dev runs need
  assets next to classes), `processResources` expands `${version}`/`${mcversion}` only in `mcmod.info`, and the run
  config forwards `-Pautotest=<x>` as system property `openfpv.autotest`.
* Transient failures happen: Maven/asset downloads occasionally fail (rate limiting / throttling). **Just retry**
  (`./gradlew downloadAssets` resumes). Seen: `Failed to get asset`, and once `Build file line 11` (ForgeGradle
  classpath resolution) which passed on retry.

## 2. Unit tests (`./gradlew test`)

38 tests, run in a few seconds, no Minecraft needed:
* `CameraMathTest` (13): frame orthonormality, MC yaw conventions, grid rays, flight vectors, signal quality,
  FOV/zoom inverse, gimbal limits, compass, energy fraction.
* `QuadPhysicsTest` (14): zero accel when level, lean-forward along heading, banking right, tilt-back brakes,
  throttle clamp, attitude lag convergence, angle wrapping/lerp, terminal speed vs simulation, **acro**: matches angle
  mode at small tilt, inverted pushes down, constant thrust magnitude, `normalizeView` keeps pitch renderable.
* `ControlLagTest` (8): lag scaling, loss probability, immediate delivery at perfect signal, delay by N ticks,
  ordering, loss rate statistics, no loss at perfect signal, clear.
* `ModuleTypeTest` (3): unique bits, mask queries, unique names.

## 3. The end-to-end self test (the most important tool)

`./scripts/run-selftest.sh` (needs `JAVA_HOME`, Xvfb, Mesa llvmpipe, OpenAL libs, curl, python3, unzip, zip):
1. `fetch-deps.sh`; `./gradlew classes` (to create the Forge dev jars); **`fix-dev-side-enum.sh`** (see §5).
2. Copies the full OC jar to `run/mods/`, writes `run/options.txt` if missing, clears old test world.
3. Starts its own **Xvfb with `+extension RANDR +extension GLX`** on a random display.
4. `./gradlew runClient --no-daemon -Pautotest=full`.
5. Prints `run/autotest/summary.txt`; exit status 0 only if `FAIL 0`.
Takes ~6–9 minutes. Screenshots: `run/autotest/screenshots/*.png`. Log lines `AUTOTEST …` in `run/logs/latest.log`.

### 3.1 How the harness works (`dev/DevHarness.java`)
* Installed from `ClientBootstrap.init()` only when `System.getProperty("openfpv.autotest") != null`.
* A client-tick **state machine**: `steps` is a list of `BooleanSupplier`; each tick the current step is called until it
  returns true. Helpers: `once(Runnable)`, `waitTicks(n)`, `until(cond, timeoutTicks, label)` (records a FAIL on timeout
  and moves on), `check(name, ok, detail)` (logs `AUTOTEST PASS/FAIL`), `shot(name)` (`ScreenShotHelper.saveScreenshot`
  into `run/autotest/screenshots`), `onServer(Callable)` (blocking call onto the integrated server thread).
* At the main menu it creates a **flat creative world** `openfpv_autotest` (seed 4242, generator string
  `3;minecraft:bedrock,2*minecraft:dirt,minecraft:grass;1;`), deletes the previous save first.
* **Real OC drones** are built from NBT and placed through OC's own item-use path: the drone item must be in the
  player's **main hand** (OC's `Delegator.onItemUse` reads `player.getHeldItem`, not the stack argument).
  Drones must be **powered on like a player does**: reflection call `Drone.start()` (runs `preparePowerUp()` which
  sets the drone's acceleration; `machine().start()` alone leaves acceleration 0 and the drone cannot move).
  Chunks must be force-loaded before spawning (`spawnEntity` silently fails in unloaded chunks).
* Four drones: **A** (all modules + Antenna Mk2, runs a long Lua API test and reports through
  `machine().lastError()` = `"RESULT|k=v;…"`), **B** (all modules, chunkloader, the main FPV test drone),
  **C** (no modules, chunkloader), **E** (all modules + Antenna Mk3 + chunkloader at x=300).
  Lua results are read via the machine crash message because tables returned from Java callbacks arrive as Scala maps
  when invoked from Java (`ClassCastException: scala.collection.mutable.HashMap`); table-valued results are therefore
  verified from real Lua, not from Java.
* Camera methods are also called from Java through OC's real plumbing: `((Component) env.node()).invoke(name, null, args…)`.
* Scenario order: world load → scenery + drones → Lua results → module gating → Assembler/driver acceptance → recipes →
  module-bay GUI (+screenshots) → FPV entry → vision modes (night/thermal/zoom/gimbal, screenshots) → **DVR** →
  quad flight (pitch ramp, speed > 8 m/s, glide, level-out, hover, bank, camera roll, yaw, **acro**, throttle) →
  signal quality/lag/loss exit → **failsafe** → **long range at 300 and 700 blocks** (entity+chunks streamed, lag,
  packet loss, release) → return-home arrival → `flyTo` → drone model/renderer/sounds (+screenshots) → lights →
  routes (one-shot + loop) → Esc/cycle/camera-off → goggles (client + server) → config GUI + live reload → finish
  (`mc.shutdown()`).

### 3.2 Things the harness taught us (bugs it found — details in doc 04)
Drone not powered; `Delegator` reading the held item; unloaded chunks; Scala-map conversion; flight blocked by the wall
(test placed it 14 blocks away); tracker not re-evaluating stationary entities; unwatched drones unloading; etc.
**Lesson: a failing check is as likely to be a test bug as a mod bug — find out which before "fixing" anything.**

## 4. Production-fidelity smoke tests (done manually in the cloud sandbox)

The dev environment uses readable (MCP) names; players get the **reobfuscated** jar. So after each feature round:
1. **Static reobf check:** `javap -c -p` over the jar's classes; every reference into `net/minecraft/` should be
   `func_*/field_*` except enum constants, `java.lang.Enum/Object` methods and Forge-added methods
   (`setRegistryName`, `openGui`, `WorldProvider.getDimension`). 382 mapped refs at the time of the first check.
2. **Reflection names** (obfuscated) were verified against the `-srg.jar` in the Gradle cache:
   `EntityTrackerEntry.range = field_73130_b`, `maxRange = field_187262_f`, `EntityTracker.trackedEntityHashTable =
   field_72794_c`, `PlayerChunkMap.getOrCreateEntry = func_187302_c` (found via `javap -c` of `isVisibleTo`).
   **These code paths run only during a long-range FPV session, so they have NOT been executed in a production
   environment** — only statically verified. (They are executed in the dev harness with MCP names.)
3. **Dedicated server:** Forge installer `--installServer`, copy release jar + OC 1.8.10 to `mods/`, `eula=true`,
   `online-mode=false`, start; expect `OpenFPV 1.0.0 ready: camera driver registered with OpenComputers` and
   `Done (…s)!`, then `stop`. Passed each round (also proves client-only classes don't load on the server).
4. **Production client:** Forge installer `--installClient` into a scratch dir, then launch
   `net.minecraft.launchwrapper.Launch` with `--tweakClass net.minecraftforge.fml.common.launcher.FMLTweaker`
   and a classpath built from the version JSONs (`tools/mklaunch_production_client.py` in this folder builds
   `classpath.txt` + extracts LWJGL natives; vanilla `1.12.2.json` was copied from the Gradle cache
   `minecraft_repo/versions/1.12.2/version.json`). Expect the main menu "7 mods loaded, 7 mods active" and log
   `Drone renderer replaced with the OpenFPV quadcopter model`. Passed (menu only — **no FPV flight was run in
   production**).
   These scratch dirs (`prodsrv/`, `prodcli/`) lived in the cloud sandbox scratchpad and are **not** in the zip.

## 5. Environment quirks (all solved; keep the workarounds)

| Symptom | Cause | Fix |
|---|---|---|
| `ArrayIndexOutOfBoundsException` in `LinuxDisplay.getAvailableDisplayModes` at startup | LWJGL 2 needs the **XRandR** X extension; `xvfb-run` default lacks it | Start Xvfb with `+extension RANDR +extension GLX` (done in `run-selftest.sh`). |
| `NullPointerException at NetworkRegistry.newChannel(NetworkRegistry.java:207)` during mod construction — **even for a stock Forge MDK** | The ForgeGradle-3 1.12.2 dev jar contains a stub `net.minecraftforge.fml.relauncher.Side` with an **extra `BUKKIT` constant** (comes from the MCP `mergetool` fat jar and overrides the real class). Forge's `NetworkRegistry` loops `Side.values()` and has no map for BUKKIT. | `scripts/fix-dev-side-enum.sh` copies the real `Side.class` from Forge's universal jar (SHA-1 verified vs Maven) into the cached dev jars (`minecraft_user_repo/.../forge-…-{injected,mci,srg,mapped_snapshot}.jar`). Dev runs only; the shipped jar is unaffected. Re-run after Gradle regenerates the cache. |
| Dev client can't find models/lang/textures (`FileNotFoundException openfpv:models/item/….json`) | FG3 puts resources in `build/resources/main`; FML looks next to classes | `sourceSets.main.output.resourcesDir = sourceSets.main.java.outputDir` in `build.gradle`. |
| Many `Unable to read a class file correctly … module-info.class` ERROR lines in dev logs | ASM 5.2 vs Java-9 classes in some libs | Harmless noise. |
| `ALSA lib … No such file` / `Could not locate OpenAL library` | No sound card in the sandbox | Harmless; sounds are verified for registration/playing state, not audibility. |
| Asset/Maven download failures | throttling | Retry. |
| Gradle `-q` build prints "Picked up JAVA_TOOL_OPTIONS" | Sandbox proxy options | Harmless. |

Sandbox specifics you will **not** have locally: an HTTPS proxy (`JAVA_TOOL_OPTIONS` with `-Dhttps.proxyHost=…`, CA bundle
`/root/.ccr/ca-bundle.crt`), Xvfb on `:99` started by hand, apt packages installed during the session
(`x11-utils x11-xserver-utils mesa-utils xdotool`), JDK 8 under `/opt/jdk8`.

## 6. Asset generation (reproducible)
* `python3 scripts/gen_textures.py` — item icons (camera, 5 modules, 4 antennas, goggles) + `module_bay.png` GUI. Pure stdlib PNG writer.
* `python3 scripts/gen_drone_assets.py` — `.bbmodel` + 64×64 texture (shelf-packed box UVs) + 5 sounds
  (pure-python synth → WAV → `ffmpeg -c:a libvorbis -q:a 4 -ac 1`) + `sounds.json`. The motor loop is a 2.0 s seamless
  loop (integer cycles; periodic noise from random-phase sines at integer multiples of 1/2 s).
  Re-running regenerates uuids in the `.bbmodel` (harmless) — **do not re-run it if you hand-edited the model in
  Blockbench**, it overwrites the file.
