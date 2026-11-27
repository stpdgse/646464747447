# 04 — Full Chat History, Decisions and Bug Log

This is the whole cloud conversation in order. "User" = Tom. Times are omitted; the order is exact.
(The cloud chat also contained many automatic tool outputs; only meaningful content is summarised.)

---------------------------------------------------------------------------------------------------

## Phase 0 — Context / user preferences given up front

System-level user preferences (verbatim intent): never skimp or leave placeholders/half-finished work; if a task needs a
real SDK/tool/package/infrastructure, **install it**; only stub when >90 % confident, else research first (web search,
docs, source); no artificial limits on installing/downloading/compiling; **speak English not German**. User name:
**Tom Pearl**. Email on account: valexapi@gmail.com. The working repo (`/home/user/646464747447`, GitHub
`stpdgse/646464747447`) already contained an unrelated Game Boy Color PCB project (`gbc-handheld/`), PRs #1–#3 unrelated.
Branch to use: `claude/gallant-edison-yxpv26`. Instruction: after pushing always open a **draft PR**.

## Phase 1 — Explaining OpenComputers (Q&A, no code)

1. **User:** "Explain ATM3 OpenComputers."
   **Answer:** ATM3 = All the Mods 3 (1.12.2). Explained OC: building a computer from Case + CPU + RAM + EEPROM +
   HDD(OpenOS) + GPU + screen + keyboard; power; adapters/cables to talk to other mods (AE2, Mekanism); robots; drones;
   upgrades; tips; offered help.
2. **User:** friend built a "Drohne", tablet and EEPROM — "Can you make FPV stuff?"
   **Answer:** Partly. OC drones have no camera/FPV. Remote control via tablet + wireless network cards works. Gave a
   sample **drone EEPROM receiver** (`modem.open(100)`, `drone.move`, `setLightColor`, `setStatusText`) and a **tablet
   controller** (WASD→`modem.broadcast`). Substitutes: sensor/radar view, Computronics camera (unconfirmed), follow-cam.
3. **User:** "Is there any mod that adds cameras for OC drones?"
   **Answer (after web searches):** none found for 1.12.2; CCTV-style camera mods and standalone FPV-drone mods exist
   (Drone FPV for Fabric 1.20.1, FPV Drones, FPV Freecam, Aperture cinematic camera) but none integrate with OC.

## Phase 2 — "Build one" and the question rounds

4. **User:** "Can you build one… get the OC API, a Java/Gradle environment and build a drone camera FPV mod for 1.12.2
   OC? **Ask me a lot of questions in multiple rounds before starting.**" (This is a standing style preference.)

### Question rounds and the user's answers (verbatim choices)
* **Round 1** — View type: **True first-person camera**. Versions: **1.12.2 Forge + OC 1.8.x**. Environment:
  **Singleplayer in ATM3**. Delivery: **Jar file plus source in this repo**.
* **Round 2** — Camera item: **Drone upgrade item**. Activation: **Lua call plus a keybind**. Flight: **Both**
  (keyboard + Lua). HUD: **Basic HUD, Crosshair and compass, Static/noise on weak signal**.
* **Round 3** — Player state: **Player frozen, not protected**. Failure: **Signal loss then auto-exit**. Balance:
  **Drains drone energy**. Recipe: **ATM3-friendly** (OC parts).
* **Round 4** — Name: **OpenFPV**. Textures: **Generated simple texture**. Lua API: **Extended**. Licence: **MIT**.
* **Round 5** — Extras: **Zoom/FOV, Night vision, Gimbal tilt/pan, Depth/color read** (+ "You also need Extra Module
  for Night Vision and stuff"). Keys: **V toggle, Esc exit**. Linking: **Cycle through drones**. Testing: "See if u can
  Full Game test and Automaten Checks with minecraft dev" (= dev client + automated checks).
* **Round 6** — Modules: **Separate upgrade items**. More modules: **Zoom, Gimbal, Thermal vision** (no depth module
  chosen here). Test setup: **Dev client with Xvfb**. Downloads: **Yes, download whatever is needed**.
* **Round 7** — Depth read location: user chose **"Add a Depth Sensor Module"** (separate module). Plan confirm: **Yes,
  start building**. Fallback: **Software rendering, then report**.

### Design problem discovered mid-way → a question
While reading OC's `DroneTemplate` bytecode: a **Tier-1 drone has 2 upgrade slots, Tier-2 has 3** (creative 9), so camera +
4–5 modules can't be separate drone upgrades. **Question asked; user chose "Module bay on the camera (Recommended)"**
(right-click the camera to open a small GUI; camera uses one drone slot).

## Phase 3 — Building the toolchain and v1 of the mod

* Repo was unrelated (gbc-handheld) → project placed in `openfpv/`.
* Sandbox had JDK 21 only → downloaded **Temurin JDK 8** via Adoptium API; Gradle 4.9 (wrapper); Forge 1.12.2-14.23.5.2860
  **MDK** (it is a ForgeGradle **3** MDK). Mojang/Maven/CIL hosts reachable through the proxy.
* Found OC dependency: `maven.cil.li` only has up to `MC1.12.2-1.7.5.221` (API jar); OC **1.8.10** (latest 1.12.2 build)
  downloaded from **Modrinth** for runtime tests. Decision: compile against the 1.7.5 API jar, run against 1.8.10.
* Decompiled OC 1.8.10 with **CFR** to learn the internals (drone entity, `DroneData`, `DroneTemplate`, `Delegator`,
  `EEPROM`, `Constants$ItemName` for part names). Key facts learned: `Drone.getTarget/setTarget(Vec3d)` use **absolute**
  coordinates; `MachineHost.internalComponents()` lists installed stacks; `drone.drag=0.8`, `maxVelocity=0.4` (8 m/s),
  `maxAcceleration=0.1`, `gravity=0.05`; targets are **rounded to ¼ block** by `targetX_$eq`.
* First `./gradlew build` succeeded (1 min). Added tracking tasks.
* Wrote v1: items (camera + 5 modules), module bay (NBT inventory, container, GUI), `DriverCamera`,
  `CameraEnvironment` (Lua), `ViewerManager`, packets, `FpvClient`, `FpvEvents`, `Hud`, shaders, textures (generated),
  lang, recipes, unit tests (16).
* First commit `9103289`.

## Phase 4 — Getting a real client running headless (the long debugging)

Problems in order, each solved:
1. **Asset download failures** (`Failed to get asset: icons/minecraft.icns`) — host fine (200), throttled; re-run `downloadAssets`.
2. **LWJGL `ArrayIndexOutOfBounds` in `LinuxDisplay`** — `xvfb-run` lacks RANDR → run own `Xvfb +extension RANDR +extension GLX`;
   installed `x11-utils x11-xserver-utils mesa-utils xdotool` (needed `apt-get update`). llvmpipe gives GL 4.5.
3. **`NetworkRegistry.newChannel` NPE** — even a pristine Forge MDK ("control experiment") crashed identically.
   Root cause traced: dev jar `Side` enum had a third constant **BUKKIT** (from `mergetool`'s embedded stub);
   fixed by replacing `Side.class` with the official one → `scripts/fix-dev-side-enum.sh` (a first version missed the
   `…_mapped_snapshot_…` directory — the one actually used at runtime — and was corrected).
4. **Dev client couldn't find the mod's assets** — resources dir fix in `build.gradle`.
5. Harness bugs, in order: drones not spawning (OC reads *held* item; chunks not loaded) → drones spawned but didn't
   run (not powered: used `Drone.start()` via reflection) → Scala map `ClassCastException` (verify tables from Lua) →
   flight didn't move (acceleration only set in `preparePowerUp`) → "HUD speed 0" was the drone pressed against the
   wall (test now samples *peak* speed).
6. Result: **64 → 66 → 67 checks pass**. Screenshots inspected; fixes from looking at them:
   * vignette drawn as **flat bands on two edges** → GL left in `GL_FLAT`; set `GL_SMOOTH` (restore `GL_FLAT`).
   * **night vision sky blown out** → soft-knee curve `1-exp(-2.4·l)`.
   * **thermal sky glowing hot** → blue-dominant pixels treated as cold.
* **Design bug found by test:** OC registers drone entities with **tracking range 80**; my config allowed 96 → the drone would
  vanish before signal loss. Default range set to **64**, config cap **80**, documented.
* More harness additions: OC **Assembler** acceptance (driver registry: accepted for drone, refused for robot/host,
  loose modules refused) → 73 checks.
* Static reobf check + production dedicated server + production client (menu) with release jar. Docs, MIT licence,
  screenshots. Commit `51530c9`; **PR #4 opened as draft** (base `claude/session-notes-migmyj`, explained in the PR).
* Final reports to user included honest limits ("Not played by a human in a full ATM3 install").
* Jar and screenshots sent to the user as files.

## Phase 5 — "Is it realistic?"

**User:** "Is it realistic". **Answer:** two readings — (a) will it work in the real game (probably, untested in ATM3);
(b) is it realistic as a drone camera (no: stylised; 64-block range; frozen body; thermal is a look; drone slow; creative
flight). Offered tilt, lag, banking.

## Phase 6 — Quadcopter feel + long range

**User:** "Make real quadcopter feel and Range upgrades to way higher."
Questions asked and answered:
* Flight model → **Angle mode (Recommended)** (not acro, not both).
* Max range → **1000+ blocks**.
* Range upgrades → **Antenna module tiers (Recommended)**.
* Signal effects → **Add control lag**.

Implementation notes:
* Antenna Mk1–Mk4 = bay slot 6, ranges 64/160/400/800/1500; `PacketCameraState` carries `range` + drone position.
* **Flight model:** sticks→tilt (lag 0.2/tick, max 35°)→accel `0.12·sin(tilt)`; vertical from throttle; per tick
  `setTarget(pos+accel)`; OC adds exactly that to velocity; drag lowered to 0.9, speed cap ×2.5. Needed **`DroneAccess`**
  (¼-block rounding bypass via DataParameters; limits via reflection).
* Client: camera bolted to body with 15° uptilt; **camera roll** via `EntityViewRenderEvent.CameraSetup`; horizon HUD;
  mouse-X → yaw stick (player yaw delta), arrow keys also yaw; `ControlLag` model (delay/loss).
* **Long range** needed a hard look at vanilla: chunks sent only around the player; entities tracked only within range of
  the player. Built `LongRangeLink` (watch chunks around the drone via `PlayerChunkMap`; raise tracker ranges via
  reflection with MCP+SRG names).
* Bugs found by the new test:
  * `Shift descends` assertion was wrong (measure from peak, not start).
  * far drone **vanished**: unwatched chunk unloaded → in real OC play a far drone needs a **chunkloader upgrade** → test
    drones B, C, E get `chunkloaderupgrade` + Lua `chunkloader.setActive(true)`; documented for players.
  * **Long-range link never came up**: vanilla `EntityTrackerEntry` only re-evaluates visibility when the entity moves
    > 4 blocks, so a hovering drone/standing pig was never sent → call `entry.updatePlayerEntity(player)` after boosting
    and after restoring.
  * Cycling test assumed only B and C exist (E is in range now); yaw-follow check raced the interpolation (tolerance
    2.5 ticks of rotation).
* Result 101/101; screenshots (bank, 300 blocks, 700 blocks with LAG 400 ms) inspected; production server + client smoke
  tested; commit `1212360`; files sent.
* Static check of obfuscated reflection names (see doc 03 §4).

## Phase 7 — "Make everything 10000x better"

**User** (long message): take lots of time, make every little thing 1000x better, more features, rework everything,
rewrite what you want.
I created/cleaned tasks and asked two questions; one malformed question call failed (JSON), the second was answered:
working style → **"Just keep going until done"**.
* Round 1 delivered only a **return-to-home failsafe** (home point, signal loss / <15 % battery → autopilot; Lua
  API; HUD `[RTH]`; 105 checks) — commit `805886a`. I said honestly what I had *not* done.
* **User:** "Well then keep going, I said keep going with everything I said until 1000x better is complete."
* Round 2 delivered (commit `2a33a3b`, 122 checks): **acro mode** (N; `QuadPhysics.acroAcceleration`, `normalizeView`),
  drone model **turns to heading** (server sets `rotationYaw`), **`flyTo`/`isAutopilot`/`getPosition`**, **FPV goggles**
  (+`requireGoggles`, client+server enforcement), **in-game config GUI** (Forge `GuiConfig`, live reload).
  Screenshot of the config GUI showed categories without descriptions → added category comments/lang keys.

## Phase 8 — Visible drone, routes, recording, lights, sound

**User:** "OK do visible drone, routes, realistic recording, lights and sound and animated and stuff. Make good
Blockbench models and stuff."
Delivered (commit `a44ed74`, 138 checks):
* **`.bbmodel`** generated by `gen_drone_assets.py` (34 cubes, groups for props/pod/lights) + 64×64 texture; **BbModel**
  loader/renderer; **DroneRenderer** replaces OC's renderer in postInit; animation (props, lean, pod, lights).
* **Sounds**: synthesised, `libvorbis` mono Ogg; `ModSounds`, `sounds.json`, `DroneSounds` (MovingSound hum, beeps).
* **Routes** (Lua), saved in NBT; route index clamped (bug found by test: index overran to size → progress "4/3").
* **Lights** (`setLights`, status LED colour).
* **DVR**: `Recorder` (ffmpeg pipe, PNG fallback), blackbox CSV, key R, REC timer. The test verifies mp4 exists, ≈61
  frames/60 ticks, CSV rows, and `ffprobe` reports `h264`.
* Screenshots of the drone in world and close-up inspected (simple but clearly a quadcopter).
* A first production-client smoke test failed only because the **Xvfb :99 had died** after a worker restart; restarted and it
  passed (`Drone renderer replaced…`).
* Files re-sent after the session worker restart (earlier sends had no confirmed result).
* Honest caveats given: sound never heard (no audio device), model quality modest, multiplayer/ATM3 untested, replay and
  remote controller not done.

## Phase 9 — Handoff

**User:** wants to move from this cloud chat to a local chat; asked for very detailed handoff documents and a zip with all
source files, everything else and the documents → this folder (`handoff/`) and the zip `openfpv-handoff.zip`.

---------------------------------------------------------------------------------------------------

## Decision log (why things are the way they are)

| # | Decision | Reason / alternative rejected |
|---|---|---|
| 1 | Modules in the camera bay | Drone slots too few (2/3). Alternatives: modules as drone upgrades (rejected: loadout too limited); craft modules into the camera (rejected: not removable). |
| 2 | Compile against OC API 1.7.5 jar, test on 1.8.10 | CIL maven stops at 1.7.5; API is stable. |
| 3 | Recipes in code | OC part ids only known at runtime. |
| 4 | Test harness in the main source set, excluded from jar | FG3 extra source sets are awkward; harness is guarded by a system property. |
| 5 | Dev workaround script instead of switching to another build system | Stock MDK is broken the same way; minimal, verified, reproducible. (Alternative not tried: RetroFuturaGradle.) |
| 6 | Server-authoritative flight | Consistency; also lets Lua autopilot and piloting share one mechanism (the target). |
| 7 | Range cap 80 / default 64 originally → tiers up to 1500 with streaming | OC tracking range 80; later solved by LongRangeLink. |
| 8 | Control lag modelled on the client | The client owns the sticks; delaying/dropping before sending is equivalent and cheap. |
| 9 | Replace OC's renderer rather than overlay | Overlay would double geometry; replacement is configurable (`customDroneModel`). |
| 10 | Blockbench file is the source of truth for the model | User asked for Blockbench models; loader reads the same file. |
| 11 | DVR streams to ffmpeg, not an in-process encoder | Real H.264 quality, no Java video codec dependency; PNG fallback keeps it working without ffmpeg. |
| 12 | Sounds synthesised in Python | No external assets/licences; reproducible. Caveat: never auditioned by a human. |

## Things the assistant told the user that you should keep true

* The mod has **not** been played by a human in ATM3.
* Not tested in multiplayer or alongside ATM3's other mods.
* Sounds were not auditioned (no audio device in the sandbox).
* The long-range reflection names were verified statically for production, not executed there.
