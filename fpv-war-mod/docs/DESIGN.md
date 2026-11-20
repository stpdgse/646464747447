# Kamikaze Skies — Design Document

Working title: **Kamikaze Skies** (mod id `kamikaze_skies`)
Status: design v1, written before any code. Owner: Tom Pearl. License: All Rights Reserved (private).

This document is the source of truth for scope and decisions. When a decision changes, edit it here and note the date in the changelog at the bottom.

---

## 1. Vision

A single-player-first (AI first, multiplayer later) Minecraft mod about flying FPV drones through a gritty, realistic battlefield. You pilot a strike drone through goggles, dive into vehicles, bunkers and buildings, and watch them blow up and collapse. Controls are simpler than a real FPV sim, but the flight still feels authentic: momentum, drag, battery sag, analog video that dies as you fly out of range.

**Pillars**
1. **FPV feel.** Goggles view, analog noise, OSD, signal loss, throttle-linked motor sound. The tension of the last mile and the lost feed.
2. **Satisfying destruction.** Blast craters plus structural collapse, debris, fire, dust.
3. **Living, scared enemies.** Soldiers panic, scatter, hide and sometimes fire back. Morale spreads.
4. **Realism of presentation.** Every model is individually crafted, detailed and animated. Realistic gritty look, within Minecraft's limits.
5. **Playable.** 60 FPS target on RTX 3060 12 GB / Ryzen 7 5700X / 32 GB with a shader pack.

**Setting rule (fixed decision):** realistic in tone and detail, but **not tied to any real-world conflict**. Factions, names, flags, insignia and vehicle designations are invented. Vehicle designs are generic "modern-style" archetypes, not recreations of a specific real nation's equipment or markings. No real-world flags, unit patches or political references anywhere (assets, text, mission briefings, sound, UI).

**Out of scope for v1:** fixed-wing drones, bomb-dropper multirotors, fully custom shader pack, tactical squad AI, voice acting.

---

## 2. Decisions summary

| Area | Decision |
|---|---|
| Platform | Minecraft Java **26.3**, **NeoForge 26.3.0.114**, **Java 25** (pinned in `gradle.properties`; Minecraft now uses year-based versions) |
| Hard dependency | **GeckoLib** only. Others allowed only if they clearly help and are explained here first |
| Shaders | Iris + a pack such as Complementary/Photon **recommended, not required**. Our own FPV video effect works with or without a pack |
| Project type | Personal passion project, All Rights Reserved |
| Repo | New folder `fpv-war-mod/` in this repo, its own Gradle project. Independent of OpenFPV and OpenComputers |
| Input | Keyboard, gamepad and touch/on-screen are **equal** citizens through one input layer |
| Modes | Mission campaign **and** free-roam sandbox |
| Economy | Craft/resource economy, crafting tiers, mission rewards and unlocks |
| World | Custom dimension (procedural) **and** hand-built mission maps. Eastern-European farmland and villages first |
| Operator | The player's body is in the world and can be targeted. Diegetic UI (goggles, radio, tablet) |
| Multiplayer | Later: co-op missions, then PvP dogfights/races. Racing mode later phase |
| Performance | 60 FPS on the reference rig above with a shader pack |
| Testing | Headless server + automated tests, plus a real client with screenshots and walking tests in the cloud |

---

## 3. Gameplay

### 3.1 Core loop
1. **Prepare** at a command post / workbench: craft parts, assemble a drone, pick a warhead.
2. **Deploy** to a launch point. The operator stands in the world, goggles on.
3. **Recon** with a scout quad: spot and mark targets with the tablet.
4. **Strike** with an FPV drone: fly to the target, hit it, watch the effect, or get jammed/shot down.
5. **Survive.** Enemies hunt the operator when they find the launch point. Relocate or defend.
6. **Debrief** and reward: resources, blueprints, unlocks.

### 3.2 Modes
- **Missions:** destroy specific targets; recon-then-strike; protect/defend the operator. Each mission has a briefing (radio/tablet), objectives, optional bonus goals, and rewards.
- **Free roam:** the generated war-zone dimension. No fixed goals. Spawns, patrols and convoys continue. Battery is unlimited only if the player enables the "practice" game rule; otherwise realistic.
- **Training range** (small, part of Phase 3): learn controls and the OSD.
- **Racing/freestyle:** gates, lap timer, leaderboards. **Later phase.**

### 3.3 Operator and link
- The operator model stands at the launch point while the player looks through the drone. The body is a real entity: it can be spotted, shot, and killed. Death of the operator ends the flight and the mission attempt (not necessarily the drone's last frames).
- A **radio link** connects drone and operator: RSSI depends on distance, terrain line-of-sight, antenna tier and **jammer fields**. See §4.3.

### 3.4 Progression
- **Crafting tiers (T1–T3)** for frame, motors, props, battery, flight controller, camera/VTX, antenna and warhead. Tiers are gated by resources and by blueprints unlocked from missions.
- **Mission rewards:** resource bundles, blueprints (new frame/warhead variants), cosmetic camera filters.
- No RPG stat tree in v1.

### 3.5 Failure and loss
- A drone that crashes, is shot down, or runs out of battery is **destroyed and its items are lost**.
- If the drone carries a **Blackbox** module, the last ~30 s of telemetry and camera frames are saved and can be reviewed on the tablet.
- Losing the operator ends the mission attempt.

---

## 4. Systems design

### 4.1 Drone flight model
Goals: arcade-leaning realism. Forgiving but with momentum, drag and prop wash.

- **Modes:** *Angle/Horizon* (default, self-leveling, beginner) and *Acro* (rate-based). An **assist slider** blends between them. Altitude/GPS hold only on the recon drone.
- **State:** position, velocity, orientation quaternion, angular velocity.
- **Forces:** thrust (4 motors, per-motor RPM from throttle/mixer), gravity, quadratic drag, simple prop-wash/ground-effect terms, collision impulses.
- **Control:** stick input → target rates (rates + expo, Betaflight-style tunable in config) → PID per axis → motor mixer. Presets (Beginner/Pilot/Expert) just change rates and assist.
- **Integration:** Minecraft ticks at 20 Hz, too coarse. The server integrates flight at a fixed **substep (target 4× per tick, 80 Hz)**. The client interpolates for rendering and drives the camera at frame rate.
- **Authority:** the server is authoritative. Single-player uses the integrated server. For multiplayer, add client-side prediction and reconciliation (Phase 9), designed in from the start by keeping the flight sim a pure function of `(state, input, dt)` with no Minecraft dependencies.
- **Input layer:** an `InputSource` abstraction (keyboard, gamepad, touch) produces normalized `(throttle, yaw, pitch, roll, arm, ...)`. Gamepad is read through the game's input system; touch uses on-screen virtual sticks. All three go through the same deadzone/expo/rate pipeline and the same network packet.

### 4.2 Battery
- LiPo-style model: cell voltage as a function of state of charge, internal resistance, **voltage sag under load** (current from total thrust). Low-voltage warning and cutoff.
- Battery tier sets capacity (flight time), sag and weight.
- Voltage feeds the OSD and slightly reduces motor output at low charge (realistic).

### 4.3 Radio link, video and jammers
- **RSSI** = f(distance, antenna tier, terrain occlusion by raycast sampling, jammer field).
- **Video feed** degrades with RSSI: scanlines, color bleed, noise, rolling bars, then freeze/glitch, then cut-out. Implemented as a **custom post-process effect** that works with or without Iris packs.
- **Control link** degrades as well: added latency and packet drop below a threshold, then failsafe (the drone cuts power or does a configurable failsafe; **no RTH in v1**).
- **Jammers** are structures/vehicle systems with a radius and strength. They reduce RSSI inside their field. Players can attack them or route around.
- **OSD:** battery voltage, RSSI, speed, altitude, flight timer, mode, warnings. Style: Betaflight-like text overlay, toggleable elements.

### 4.4 Warheads
| Warhead | Effect | Strong vs | Weak vs |
|---|---|---|---|
| Impact blast (standard) | Medium radius blast, moderate block damage | Structures, soft vehicles | Heavy armor |
| Shaped charge | Small radius, high penetration, deep crater | Armored vehicles, bunkers | Infantry, open ground |
| Fragmentation | Wide radius, many damage rays | Infantry, spotters | Structures, armor |

Fuzing is impact-based in v1. Warheads add weight (affects flight and battery).

### 4.5 Destruction
- **Blast:** custom explosion solver. Rays from the center lose energy through blocks by hardness; blocks are removed or damaged based on energy. Different profiles per warhead.
- **Structural collapse:** after a blast, run an integrity check on the affected region. Blocks are nodes; support comes from ground and from block-to-block connections (with a max unsupported span per material). Unsupported blocks become falling debris (pooled falling-block entities or lightweight debris entities, capped per tick/per area).
- **Budgets (performance-critical):** collapse analysis runs **off the main thread** on a snapshot of the region; results are applied over several ticks with a global per-tick block budget. Debris, particles and fire are capped and degrade by quality setting.
- **Effects:** dust clouds, fireballs, smoke columns, scorch/crater decals via block states, camera shake, screen distortion on the drone feed, shockwave knock-back for entities.
- **Targets with health:** vehicles and some structures have hit points in addition to block damage (a tank is not just a pile of blocks).

### 4.6 Enemy AI ("reactive and scared")
No tactical squad AI in v1. Behavior is a morale-driven state machine.

- **Perception:** sight cone with range, hearing of drone prop noise (louder near, low battery sag affects pitch), reaction to explosions.
- **States:** Idle → Alert → Panic → Flee/Hide → FightBack (rare) → Dead. Morale value 0–100.
- **Morale:** falls with nearby explosions, casualties, a drone approaching fast, an officer dying. Rises with the officer present and with cover/vehicles.
- **Contagion:** one panicking soldier lowers morale of those within a radius. **Officer/spotter** can rally and alert.
- **Hide:** soldiers run into bunkers, trenches or vehicles.
- **Fight back:** a brave minority or cornered soldiers fire at the drone; accuracy falls with panic. **Drone-hunters** use a jammer gun or shotgun. **Machine gunners** lay suppressive fire on a sighted drone.
- **Operator hunt:** if the launch point is revealed, a small team goes there. Alerts spread through officer/radio.
- **Difficulty** scales perception, fire accuracy and morale thresholds.

### 4.7 Vehicles (AI)
Tank, APC/IFV, trucks, artillery/mobile air defense. Simple path-following on roads and open ground, crews (optional later), turret tracking for armed vehicles, hit points per component. They are **targets first**, not a full vehicle sim.

---

## 5. Content list (each asset modeled, textured and animated one by one)

Models: Blockbench `.bbmodel` → GeckoLib. Textures 64–128 px per model (mixed by importance). The headless Blockbench pipeline in `tools/blockbench/` is the starting point.

### 5.1 Drones and items
- FPV strike quad (frame tiers T1–T3, motors, props with spin animation)
- Recon quad
- Goggles (first-person overlay + item model)
- Radio controller / tablet (diegetic UI)
- Warhead variants (impact, shaped, frag)
- Battery, flight controller, camera/VTX, antenna, Blackbox module
- Workbench / command post blocks

### 5.2 Soldiers (full animation sets)
Roles: **Rifleman, Machine gunner, Drone-hunter, Officer/Spotter.**
Animations: idle, walk, run, crouch, aim, fire, reload, panic, flee, hide, flinch, hit reaction, death (several), plus role-specific (radio call, rally, jammer gun sweep, MG deploy).
Gore: moderate, configurable blood/impact effects, blast throw, no extreme dismemberment.

### 5.3 Vehicles
- Battle tank (turret traverse, gun recoil, tracks, exhaust, wreck model)
- APC/IFV (hatches, turret, wheels or tracks, wreck)
- Trucks / supply and utility vehicles (wheels, cargo variants, wrecks)
- Artillery piece / mobile air defense (elevating barrel, radar dish animation)

### 5.4 Structures and defenses
- Ruined buildings and houses (several styles, built from blocks + decorative models)
- Bunkers, trenches, fortified positions, sandbag/barrier props
- Anti-drone **jammer** (antenna array with active animation)
- **Anti-air gun / net emplacement**, spotter post

### 5.5 Terrain and blocks
Mud, churned field, burned ground, rubble, damaged variants of common building blocks, craters, tank tracks decals.

---

## 6. World

- **Custom dimension:** procedural Eastern-European-style farmland, tree lines, dirt roads, villages (ruined and intact), trenches and fortified lines, scattered wrecks. Controlled terrain generator, structure sets and spawn tables.
- **Mission maps:** hand-built scenario structures/arenas, loaded as templates into mission instances. Every mission ships objective markers and spawn data.
- **Seasonal variants** (mud/snow) are a possible later addition, not v1.

---

## 7. Audio

All sounds are **original or CC0**, tracked in `docs/SOUNDS.md` with source URL, license and checksum.

- **Drone motors:** procedural or layered loops with pitch/volume tied to throttle/RPM. Doppler for other players hearing it.
- **Explosions:** near/far variants with distance delay and echo. Different profiles per warhead.
- **Radio:** static crackle that rises as RSSI falls, warning beeps (low battery, signal), menu clicks.
- **Ambience:** distant gunfire, shouting, engines, wind.
- **Sourcing rules:** only download from reputable libraries (e.g. Freesound CC0, or comparable CC0 sources). Keep only audio files (`.ogg`/`.wav`/`.flac`), never executables or archives of unknown content. Verify the license per file, record it, store a SHA-256, and scan with ClamAV when available. Reject anything not clearly CC0 or compatible.

---

## 8. Technical architecture

```
fpv-war-mod/
  build.gradle, gradle.properties, settings.gradle
  src/main/java/.../kamikazeskies/
    core/          mod bootstrap, registries, config
    sim/           pure Java: flight model, battery, link/RSSI, morale math (NO Minecraft deps)
    drone/         drone entity, server tick, networking, control authority
    input/         InputSource (keyboard/gamepad/touch), rates/expo pipeline
    render/        FPV camera, post-process, OSD, goggles overlay
    destruction/   blast solver, integrity analysis, debris, budgets
    ai/            perception, morale state machine, roles, operator hunt
    world/         dimension, structures, mission instances
    content/       items, blocks, crafting, loot/rewards
    audio/         sound events, procedural motor audio
  src/main/resources/  assets (geo models, animations, textures, sounds), data
  src/test/java/       unit tests for sim/ and destruction math
  src/gametest/        NeoForge GameTests (headless server)
  docs/                this file, SOUNDS.md, ASSETS.md, TESTING.md
```

**Key principles**
- `sim/` is **pure Java and heavily unit-tested.** Flight, battery, RSSI and morale math do not touch Minecraft classes.
- **Server authoritative** game logic; the client renders and sends inputs. This is what keeps multiplayer possible later.
- **Data-driven** content where practical (warheads, drone parts, mission definitions as JSON).
- **Budgeted** heavy work (destruction, AI) with quality presets.
- No OpenComputers, no Lua, no dependency on the old OpenFPV code. Old flight logic is **reference only**, reimplemented cleanly in `sim/`.

---

## 9. Testing strategy

- **Unit tests** (JUnit) for `sim/`: hover equilibrium, rate response, battery curve, RSSI model, morale transitions, blast energy math, integrity solver on hand-made structures.
- **GameTests** (NeoForge) on a headless dedicated server: drone spawns and flies scripted input into a wall; a building collapses within budget; AI panics when a drone approaches; mission objectives complete.
- **Real-client runs** in the cloud under Xvfb with software rendering (SwiftShader/Mesa): launch client in dev mode (offline account), run scripted walk/fly paths, capture **screenshots and short GIFs** of each feature. Limits: slow frame rates, no real GPU. These verify look and behavior, not performance.
- **Performance:** measure server tick time and entity/block counts on the headless server; budget numbers are tracked in `docs/TESTING.md`. Final FPS on the reference rig is for Tom to confirm.
- **CI:** build and unit tests on every PR when practical.

---

## 10. Phases and exit criteria

Delivery is in big chunks with a draft PR, screenshots/GIFs and a short written summary at each milestone.

| # | Phase | Exit criteria |
|---|---|---|
| 0 | **Project skeleton and test harness** ✅ done 2026-11-20 | Gradle project builds a NeoForge mod jar; dedicated server boots; a GameTest runs; client launches under Xvfb and a screenshot is captured; CI builds |
| 1 | **Flight sim core + input** ✅ done 2026-11-20 | `sim/` flight model with unit tests; spawnable drone entity; keyboard and gamepad flying; goggles camera; basic OSD |
| 2 | **Destruction prototype** | Drone impact destroys blocks with a blast profile; structural collapse of a test house; budgets and quality presets; GIF of a house collapsing |
| 3 | **FPV feed, battery, link** | Analog post-process, voltage sag, RSSI with distance and jammers, signal loss/freeze; training range |
| 4 | **Crafting and drone assembly** | Parts, tiers, workbench, warheads, blackbox; drone loss rules |
| 5 | **Art pipeline vertical slice** | One soldier (rifleman, full animation set), one vehicle (tank) and the FPV drone, fully modeled, textured and animated through Blockbench + GeckoLib |
| 6 | **Scared AI + enemy roles** | Perception, morale, contagion, hide/flee/fight; all four soldier roles; jammer and anti-air defenses; operator hunt |
| 7 | **Missions and world** | Mission framework, custom dimension, first hand-built mission map, rewards/unlocks, free-roam mode |
| 8 | **Content completion** | Remaining vehicles, structures, defenses, audio pass, polish, config screen, touch controls |
| 9 | **Multiplayer** | Co-op missions, then PvP dogfights; client prediction; later racing mode |

Phase order may change as we learn. Phase 2 may be pulled earlier if the skeleton is quick, because destruction determines whether the idea feels good.

---

## 11. Risks and open questions

| Risk | Plan |
|---|---|
| Structural collapse is too slow | Off-thread analysis, region limits, per-tick block budget, quality presets; prototype early in Phase 2 |
| Flight feel at 20 TPS | Server substeps + client interpolation; pure-function sim; tune rates in Phase 1 |
| 60 FPS with shaders and many entities | Entity caps, LOD for animations, pooled debris, scalable settings; measure continually |
| Cloud client testing is slow or limited | Rely on headless GameTests for correctness; use the client only for visual checks and screenshots |
| Mojang auth in cloud | Use dev-mode offline accounts for the client |
| Sound licensing | CC0-only rule, per-file records, checksum + scan |
| Asset volume (4 soldier roles, 4 vehicles, many structures) | One asset at a time, with a checklist in `docs/ASSETS.md`; reuse rigs and animation sets across soldier roles |
| NeoForge/Minecraft version churn | Pin exact versions; upgrade only at milestone boundaries |
| Touch controls on Java Edition | On-screen sticks via our own overlay; confirm the target devices later |

**Open questions (decide later):** faction names and mission narrative, flight-feel preset values, final asset art direction palette, whether vehicles need crews in v1.

---

## 12. Changelog
- 2026-11-20: Phase 1 complete. Pure-Java flight sim (rate PID with I-term relax and anti-windup, angle/acro assist blend, X-quad mixer, motor lag, drag), server-authoritative drone entity with block collisions and crash, pilot networking, keyboard + gamepad (SDL3) input layer, quaternion FPV camera with roll, OSD, GeckoLib model from the Blockbench pipeline. 55 unit tests, 10 GameTests. Controls in `CONTROLS.md`.
- 2026-11-20: Phase 0 complete. Pinned Minecraft 26.3 / NeoForge 26.3.0.114 / GeckoLib 5.5.7 / Java 25. Gradle project, unit tests, GameTest harness, server smoke test and software-rendered client screenshot all working (see `TESTING.md`).
- 2026-11-20: Design v1 written from the planning Q&A. Decisions: NeoForge + GeckoLib, new folder in this repo, fictional (non-real-conflict) setting, CC0 sound sourcing approved, all-input-equal control layer.
