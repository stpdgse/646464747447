# 01 — Project Overview: OpenFPV

## 1. What this is

**OpenFPV** is a Forge mod for **Minecraft 1.12.2** (Forge `1.12.2-14.23.5.2860`) that depends on
**OpenComputers** (`required-after:opencomputers@[1.7.5,)`). It was written for the user's **All the Mods 3**
(ATM3) modpack but only needs OC. It gives OC drones a first-person-view (FPV) camera and a full "real quadcopter"
experience.

* Mod id: `openfpv`, name `OpenFPV`, version `1.0.0`, package `com.openfpv`, licence **MIT** (Tom Pearl).
* Built jar: `build/libs/openfpv-1.12.2-1.0.0.jar` (~172 KB, also shipped in the zip next to this document).
* Source location in git: `openfpv/` inside repo `stpdgse/646464747447` (the repo also contains unrelated work:
  `gbc-handheld/` EDA/PCB project, `setup-eda.sh`, `docs/`; ignore those).
* Branch: `claude/gallant-edison-yxpv26`. Draft PR **#4** "OpenFPV: FPV camera upgrade + modules for OpenComputers
  drones (MC 1.12.2)", base branch `claude/session-notes-migmyj` (because the branch sits on top of PR #1's commits;
  retarget to `main` once PR #1 is merged).
* Commits of this project (oldest → newest):
  `9103289` initial mod, `51530c9` end-to-end self test + fixes, `1212360` quad flight + antennas + long range,
  `805886a` return-to-home failsafe, `2a33a3b` acro/flyTo/goggles/config GUI, `a44ed74` visible drone/sounds/routes/DVR.

## 2. What a player can do (feature list)

| Area | Feature |
|---|---|
| Item | **Camera Upgrade** — fits *one* drone upgrade slot; provides Lua component `camera`. Right-click opens a 6-slot **module bay** (5 module slots + 1 antenna slot) stored in the item's NBT. |
| Modules (items, live in the bay) | Night Vision, Zoom, Gimbal, Thermal Vision, Depth Sensor. Each unlocks Lua methods; without it the call returns `nil, "module not installed: …"`. |
| Antennas (items, live in bay slot 6) | Mk1–Mk4 set the video range: **64 (none) / 160 / 400 / 800 / 1500 blocks**. Crafted in a chain (Mk2 needs Mk1, …). |
| FPV | **V** enters/leaves FPV (nearest camera drone), **B** cycles drones, **Esc** exits. The pilot's body stands still (not protected). |
| Flight model | Real-quad **angle mode** (sticks → tilt → thrust vector → acceleration, glide, self-level) and **acro mode** (rate control, flips, inverted flight), toggled with **N**. Camera is bolted to the body with 15° uptilt, **rolls** with the bank, flip-safe. Piloted speed cap raised from OC's 8 m/s to ~20 m/s. |
| Controls | W/S pitch, A/D bank, Space/Shift throttle, mouse-X or ←/→ yaw, N acro/angle, R record, V/B/Esc. |
| HUD / OSD | Crosshair, artificial horizon, compass tape, altitude, speed, range, battery bar (flashes <15 %), signal bars, control lag, zoom, module tags (`[NV] [THERMAL] [DEPTH] [RTH] [ANGLE]/[ACRO]`), REC timer. |
| Vision | Night vision (green phosphor post-shader), thermal (heat-palette post-shader; living things drawn white-hot; sky/water cold), zoom (FOV 10–110), gimbal tilt/pan. |
| Link | Static/scanlines/tearing from 50 % of range; **control lag** (up to 500 ms) and **packet loss** (up to 30 %) at the edge; signal-loss exit. |
| Long range | While flying, chunks around the *drone* are streamed to the pilot and entities near it are force-tracked (works hundreds of blocks out). Needs an OC **chunkloader upgrade** if nobody is near the drone. |
| Failsafe | First `camera.start()` records a **home point**. Signal loss or battery < 15 % ⇒ autopilot flies home. |
| Autopilot | `flyTo(x,y,z)`, **routes** of up to 64 waypoints (one-shot or loop), saved with the drone. |
| Goggles | **FPV Goggles** helmet-slot item; optional config `flight.requireGoggles` (enforced client **and** server). |
| Visible drone | OC's drone renderer is replaced by an **animated quadcopter** loaded from a **Blockbench `.bbmodel`**: spinning props (only when powered), lean into motion, gimbal-following camera pod, blinking nav lights + status LED (colour from `drone.setLightColor`), turns to heading. |
| Sound | Motor hum following each powered drone (pitch/volume with speed), link-up/link-lost/low-battery beeps, record click. Synthesised, encoded to Ogg Vorbis. |
| DVR | **R** records the FPV feed (OSD included) at 20 fps → H.264 mp4 via `ffmpeg` (PNG-sequence fallback) + **blackbox CSV** telemetry. |
| Config | `config/openfpv.cfg` and an in-game screen (Mods → OpenFPV → Config), live reload. |
| Lua API | 44 methods on component `camera` (see doc 05). |

## 3. Why it exists / history in two paragraphs

Tom saw a friend build an OC drone with a tablet and EEPROM and asked whether FPV is possible. OC drones have no
camera and no FPV; no addon existed (searched). Tom asked for a real mod to be built, with many clarifying
questions first. It was built from scratch: toolchain (JDK 8, ForgeGradle 3, Gradle 4.9), OC API, a scripted
end-to-end test harness running a real Minecraft client headless, then successive rounds of features on request
(realism, range, "10000x better", visible drone / routes / sounds / recording).

The whole story, including every bug and decision, is in `04_CHAT_HISTORY_AND_DECISIONS.md`.

## 4. Repository layout (project part)

```
openfpv/
  build.gradle, gradle.properties, gradlew*, gradle/        ForgeGradle 3 build, mod version, deps versions
  LICENSE, README.md                                         MIT, user-facing manual
  scripts/
    fetch-deps.sh            downloads OC API jar + OC 1.8.x jar into libs/
    fix-dev-side-enum.sh     workaround for the ForgeGradle-3 "Side.BUKKIT" crash (dev runs only)
    run-selftest.sh          one-command end-to-end test (Xvfb + Mesa + real client)
    gen_textures.py          item icons + GUI texture (pure python)
    gen_drone_assets.py      drone .bbmodel + texture + all sounds (+ sounds.json); needs ffmpeg/libvorbis
  src/main/java/com/openfpv/   (see doc 02)
  src/main/resources/          assets (lang, models, textures, shaders, sounds, bbmodel), mcmod.info, pack.mcmeta
  src/test/java/               JUnit tests (maths, physics, control lag, module masks)
  docs/                        screenshots + last self-test summary
  handoff/                     THIS folder
```

Total: ~5,000 lines of Java (incl. a 1,300-line test harness), ~450 lines of Python, 5 shader files, a 3,365-line
(generated, pretty-printed) `.bbmodel`.

## 4b. Numbers

* Java classes: 39 (main) + 4 test classes. Largest: `DevHarness` 1304, `CameraEnvironment` 722, `FpvClient` 509.
* Unit tests: 38 (CameraMath 13, QuadPhysics 14, ControlLag 8, ModuleType 3).
* End-to-end checks: **138** (see `docs/selftest-summary.txt`).
* Items: camera upgrade, 5 modules, 4 antennas, goggles = 11 items, 11 recipes (shaped, built in code).
* Packets: 5 (`PacketCameraState` S→C, `PacketAttitude` S→C, `PacketViewEnd` S→C, `PacketView` C→S, `PacketControl` C→S).
* Keybinds: V toggle, B next, N mode, R record, ← yaw left, → yaw right.

## 5. Dependencies and versions

| Thing | Version | Notes |
|---|---|---|
| Minecraft | 1.12.2 | |
| Forge | 14.23.5.2860 | MDK uses **ForgeGradle 3.+** on **Gradle 4.9**; needs **JDK 8** |
| MCP mappings | `snapshot_20171003-1.12` | |
| OpenComputers (compile) | `MC1.12.2-1.7.5.221` **API jar only** (`compileOnly`) | from `https://maven.cil.li` |
| OpenComputers (runtime/test) | `1.8.10` (Modrinth) | any OC 1.7.5+/1.8.x should work; API surface used is small |
| JUnit | 4.12 | |
| ffmpeg (optional, runtime) | any with libx264 | DVR; also used by `gen_drone_assets.py` (libvorbis) |

## 6. People / accounts / places

* User: **Tom Pearl**. GitHub repo owner shown in tools: `stpdgse`, repo `646464747447` (private-ish; push via the
  session's git proxy). Claude session URL (cloud): `https://claude.ai/code/session_012D6NdNStvvagn37wvZw1GY`.
* The cloud sandbox is **ephemeral**; everything worth keeping was committed/pushed or is in the zip.
