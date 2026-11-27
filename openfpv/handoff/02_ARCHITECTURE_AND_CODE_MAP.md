# 02 — Architecture and Code Map

Package root: `com.openfpv`. "S" = runs on the (integrated or dedicated) server thread, "C" = client thread.

## 1. Big picture

```
 LUA PROGRAM (drone EEPROM)            SERVER                                   CLIENT (pilot)
 ────────────────────────   ──────────────────────────────────────   ─────────────────────────────────
 component "camera"  ─────►  CameraEnvironment (OC ManagedEnvironment)
   start/stop/setFov…         · state: active, night, thermal, fov, tilt, pan, lights, home, route…
                              · update() each tick: energy drain, low-battery RTH, autopilot, state sync
                              · registers itself in CameraRegistry (by drone entity id)
                                   │  PacketCameraState (every 20 ticks / on change, to whole dimension)
                                   ▼                                                    ▼
                              ViewerManager  ◄── PacketView(start/stop) ───────  FpvClient (state machine)
                              · sessions by player UUID                          · knows all active cameras
                              · quad flight sim (angle/acro)  ◄─ PacketControl ─ · sticks → ControlLag → send
                              · sets drone target each tick  ── PacketAttitude ► · camera = drone attitude
                              · range/energy/signal checks   ── PacketViewEnd ──► · HUD, shaders, DVR, sounds
                              · LongRangeLink (chunks+entities)
                                   │
                              DroneAccess (reflection into OC's Drone entity: exact targets, speed cap, drag)
```

**Key idea:** a piloted drone is never moved directly. Every tick `ViewerManager.fly()` computes a thrust vector from
the commanded attitude and sets OC's *movement target* to `position + acceleration`. OC's own drone physics then adds
exactly that to the velocity (it adds `min(accel, distance)` toward the target) and applies drag. That is why OC's
quarter-block target rounding had to be bypassed (`DroneAccess`).

## 2. File-by-file

### 2.1 Common / registry
| File | Lines | Purpose |
|---|---|---|
| `OpenFPV.java` | 71 | `@Mod`. preInit: config, network, GUI handler, registers `ViewerManager` + itself on the event bus, client bootstrap. init: `Driver.add(new DriverCamera())`. postInit: client renderer swap. `onConfigChanged` reloads config. `guiFactory` = `client.OpenFPVGuiFactory`. |
| `Config.java` | 148 | All settings (static fields) + `Configuration cfg`, `load(File)`, `read()` (copy file→fields, set category comments/lang keys). `rangeForTier(int)`. |
| `ModItems.java` | 77 | Creative tab, item instances (`CAMERA`, `GOGGLES`, `MODULES[5]`, `ANTENNAS[4]`), registration + model registration (client). |
| `ModRecipes.java` | 93 | Recipes built **in code** with `ShapedOreRecipe`; OC parts fetched via `li.cil.oc.api.Items.get(name)` (`chip1/2/3`, `printedcircuitboard`, `graphicscard1`, `wlancard1/2`, `chipdiamond`); falls back to vanilla stand-ins + warning if an OC part is missing. |
| `ModSounds.java` | 29 | 5 `SoundEvent`s registered via `RegistryEvent.Register<SoundEvent>`. |

### 2.2 Items and module bay (`item/`, `bay/`)
| File | Purpose |
|---|---|
| `item/ItemCamera.java` | Right-click opens the bay GUI (`player.openGui`), tooltip lists installed modules. |
| `item/ItemModule.java`, `ModuleType.java` | A module item with a `ModuleType` (NIGHT_VISION, ZOOM, GIMBAL, THERMAL, DEPTH); `bit()`/`has(mask,type)`; `luaName`. |
| `item/ItemAntenna.java` | Tier 1..4; tooltip shows range from config. |
| `item/ItemGoggles.java` | `getEquipmentSlot` = HEAD (wearable in helmet slot without an armor model). |
| `bay/ModuleBay.java` | NBT (de)serialisation under item tag `openfpv.Modules` (list of stacks with `Slot` byte); `SIZE=6`, `MODULE_SLOTS=5`, `ANTENNA_SLOT=5`; `readMask`, `readAntennaTier`; `Inv` = `InventoryBasic` that writes back to the stack on every change. |
| `bay/ContainerModuleBay.java` | 6 bay slots at x=34+18i, y=20 + player inventory; slot validity; **blocks moving the camera stack being edited**; shift-click routing. |
| `bay/GuiModuleBay.java`, `GuiHandler.java` | Client GUI with ghost icons in empty slots; GUI id 1, hand ordinal passed in `x`. |

### 2.3 OpenComputers integration (`oc/`)
| File | Purpose |
|---|---|
| `DriverCamera.java` | `prefab.DriverItem` + `HostAware`: accepts the camera only for hosts assignable to `li.cil.oc.api.internal.Drone`; slot `Upgrade`, tier 0; creates `CameraEnvironment`. Modules/antennas have **no** driver on purpose (they only live in the bay). |
| `CameraEnvironment.java` | 722 lines. The `camera` component. Node: `Network.newNode(this, Visibility.Network).withComponent("camera").withConnector().create()`. State is `volatile` (Lua "direct" callbacks run on the machine thread); `dirty` flag + `update()` (server thread) does all world access/network sends. Callbacks: base, night/thermal/zoom/gimbal/depth, home/RTH/flyTo, routes, lights (see doc 05). Autopilot, route progression and low-battery trigger live here. NBT: night, thermal, fov, tilt, pan, lights, route, home. `active` is deliberately **not** persisted. |
| `CameraRegistry.java` | `ConcurrentHashMap<entityId, CameraEnvironment>`; registered in `onConnect`, removed in `onDisconnect`. |
| `DroneAccess.java` | Reflection into `li.cil.oc.common.entity.Drone`: writes `DataTargetX/Y/Z` DataParameters directly (exact floats; OC's `setTarget` rounds to ¼ block), reads/sets private final floats `maxVelocity` and `drag`. Falls back to the public API (coarse) if names change. |

### 2.4 Server (`server/`)
| File | Purpose |
|---|---|
| `ViewerManager.java` | Authoritative FPV sessions (`HashMap<UUID,Session>`, server thread only). `begin()` validates (goggles if required, entity is Drone, camera active, distance ≤ `cam.range()`, not piloted by another), raises speed cap + lowers drag (`DroneAccess`), starts `LongRangeLink`. `onServerTick` checks end conditions (player dead, drone gone, camera off, signal lost → RTH failsafe) then `fly()`: angle/acro attitude, `QuadPhysics` accel, `clampLength(0.095)`, `setTarget(pos+accel)`, sets `entity.rotationYaw` (visible heading), sends `PacketAttitude`. Control timeout 20 ticks → sticks zero (hover). `finish()` restores limits, holds position, releases the link. Test hooks: `attitude(UUID)`, `linkStats(UUID)`, `wearsGoggles`. |
| `LongRangeLink.java` | **Chunks:** every 5 ticks makes the pilot a watcher (`PlayerChunkMapEntry.addPlayer`, entry obtained by reflection on private `getOrCreateEntry`/`func_187302_c`) of chunks within `streamRadius` of the drone that are outside the pilot's own view square; unwatch on leave (never removes chunks inside own view + 1 margin). **Entities:** raises `range`/`maxRange` of the tracker entries (reflection: `range`/`field_73130_b`, `maxRange`/`field_187262_f`, `EntityTracker.trackedEntityHashTable`/`field_72794_c`) for the drone and entities within `boostRadius`, and **explicitly calls `entry.updatePlayerEntity(player)`** because vanilla only re-evaluates visibility when an entity moves > 4 blocks (a hovering drone / standing pig would never be sent). Restore + re-evaluate on release. |

### 2.5 Network (`net/`)
| Packet | Dir | Contents / handler |
|---|---|---|
| `PacketCameraState` | S→C | entityId, flags (active, night, thermal, returning, lights), fov, tilt, pan, energy (0..1), modules mask, **range**, drone x/y/z. → `FpvClient.onState`. Sent to the whole dimension. |
| `PacketAttitude` | S→C | entityId, yaw, pitch(nose-down), roll(right). Every tick to the pilot. → `FpvClient.onAttitude`. |
| `PacketViewEnd` | S→C | reason: REFUSED/SIGNAL_LOST/DRONE_GONE/CAMERA_OFF. |
| `PacketView` | C→S | entityId + start flag. |
| `PacketControl` | C→S | entityId, 4 sticks (pitch, roll, yaw, throttle ∈ [-1,1]), acro flag. Sent **every tick** while flying (after the control-link model). |
`Net.java` holds the `SimpleNetworkWrapper` (channel "openfpv") and helper senders.

### 2.6 Client (`client/`)
| File | Purpose |
|---|---|
| `FpvClient.java` | 509 lines. State machine + key bindings. `CAMS` map of known cameras (from state packets). `enter()` sends `PacketView` and sets **pending**; the view switches to the drone only when the entity is loaded on the client, its chunk is loaded and the first attitude has arrived (needed for far drones). `clientTick()` handles keys (V,B,N,R), telemetry (self-tracked speed, distance, signal quality), shaders/night effect, and `sendControl()` = sticks → `ControlLag` (delay/loss) → `PacketControl`. Mouse yaw: the vanilla camera code turns the player; the per-tick yaw delta is interpreted as a yaw stick, player pitch is zeroed. Camera angles: `viewAngles()` = interpolated body attitude + yaw pan − uptilt − gimbal tilt, normalised flip-safe. `aimCamera()` writes them to the drone entity (render view entity) each frame. |
| `FpvEvents.java` | Forge event subscribers: ticks, Esc→exit (`GuiOpenEvent`), freeze player movement (`InputUpdateEvent`), cancel clicks/scroll, FOV, **camera roll** (`EntityViewRenderEvent.CameraSetup`), hide hand/vanilla HUD, thermal entity tinting (`RenderLivingEvent` Pre/Post → flat white/grey), world unload/disconnect reset; calls `DroneSounds.clientTick()` and `Recorder.frame()`. |
| `Hud.java` | Vignette (must set `GL_SMOOTH`), static/scanline `Noise`, crosshair, horizon, compass, telemetry, tags, REC timer. |
| `BbModel.java` | **Blockbench loader/renderer.** Parses `.bbmodel` JSON (Free Model, box UV): elements (cubes with from/to/origin/rotation/uv_offset), outliner groups. Draws cubes with `Tessellator` (`POSITION_TEX_NORMAL`), per-group rotation about the group origin via an `Animator`, fullbright tinted `light_*` cubes. |
| `DroneRenderer.java` | `Render<Entity>` that replaces OC's drone renderer (put into `RenderManager.entityRenderMap` in postInit). Animation: props spin only when `isRunning()` (reflection on OC entity), lean from entity motion, `camera_pod` pitch = uptilt+gimbal tilt / yaw = −pan, lights blink, status LED from `lightColor()`. |
| `DroneSounds.java` | `MovingSound` motor hum per powered drone within 48 blocks (re-scan every 10 ticks), pitch/volume from speed; low-battery beep; `play()` for UI sounds. |
| `Recorder.java` | DVR: `glReadPixels` once per tick at `RenderTickEvent.END`, frames → bounded queue (12) → writer thread → `ffmpeg` stdin (rawvideo rgb24, `vflip`, libx264) or PNG fallback; telemetry CSV; never blocks the game (drops frames when queue full). |
| `OpenFPVGuiFactory.java` | `IModGuiFactory` → `GuiConfig` over `Config.cfg` categories. |
| `ClientBootstrap.java` | Registers events + key bindings; `postInit()` replaces the drone renderer; installs the dev harness only if system property `openfpv.autotest` is set. |

### 2.7 Pure logic (`util/`) — all unit-tested, no Minecraft imports
* `CameraMath` — clamps, FOV/zoom, gimbal limits, **signal quality** (1 up to 50 % of range, linear to 0 at 100 %), orthonormal camera frame (`forward/right/up`), pinhole `gridRay` for the depth sensor, flight vector, compass heading.
  Conventions: MC yaw 0 = +Z (south), 90 = −X (west); pitch positive = looking down; Lua gimbal tilt positive = up.
* `QuadPhysics` — `acceleration` (angle mode: `accelScale·sin(tilt)` along body axes, vertical from throttle), `acroAcceleration` (thrust along body-up for any attitude, level hover cancels exactly), `approach` (attitude lag), `wrapDegrees`, `lerpAngle`, `normalizeView` (keep pitch in ±90 by flipping yaw/roll), `clampLength`, `terminalSpeed`.
* `ControlLag` — queue that delays/drops control packets by signal quality; order-preserving; seeded RNG.

### 2.8 Dev-only
* `dev/DevHarness.java` (1304 lines) — scripted end-to-end test; **excluded from the release jar** (`jar { exclude 'com/openfpv/dev/**' }`). See doc 03.

## 3. Threading model (important)

* OC "direct" Lua callbacks run on the OC machine thread → `CameraEnvironment` fields are `volatile`; callbacks only set fields and a `dirty` flag. World access and packets happen in `update()` (server thread). **Non-direct** callbacks (`depth`, `color`) run on the server thread and ray-trace the world.
* `ViewerManager` / `LongRangeLink` are server-thread only (static `HashMap`, no locking).
* Client state (`FpvClient` etc.) is client-thread only; packet handlers hop onto it with `addScheduledTask`.
* `Recorder` has one writer thread; handoff via `ArrayBlockingQueue`; `recording`/`finished` are `volatile`.

## 4. Design decisions worth knowing (and why)

1. **Modules live in the camera's bay, not drone slots.** A Tier-1 drone has 2 upgrade slots, Tier-2 has 3 (creative 9); camera + 4 modules wouldn't fit. User chose the bay (GUI) approach.
2. **One `camera` component** with module-gated methods (instead of one component per module) so Lua has a single handle; modules are marker items.
3. **Server-authoritative flight, client-authoritative sticks.** Attitude is simulated on the server and sent back every tick; the camera displays server attitude (consistent, slightly laggy on dedicated servers = realistic).
4. **Quarter-block rounding bypass** (`DroneAccess`) — essential; with OC's API the fine per-tick targets are unusable.
5. **Tracker re-evaluation** — essential for long range (see LongRangeLink).
6. **Entity target ranges**: OC registers drones with tracking range **80**; the default no-antenna range is 64 so the entity never disappears before the signal does.
7. **Recipes in code** because OC part registry names are only known at runtime and ATM3 may remove/alter parts.
8. **Resources for ForgeGradle 3 dev runs**: `sourceSets.main.output.resourcesDir = sourceSets.main.java.outputDir` (otherwise FML finds no assets in dev).
9. **Shaders live in the `minecraft` namespace** (`assets/minecraft/shaders/...`, names prefixed `openfpv_`) because vanilla 1.12 `ShaderManager` cannot resolve namespaced program names.
10. **Thermal vision** = post-shader (luma→heat palette, blue-dominant pixels cold) + `RenderLivingEvent` flat-white silhouettes; night vision also borrows the vanilla night-vision potion on the client player (vanilla ignores the render view entity for lightmap brightness).
11. **Drone renderer replacement** at postInit by overwriting `RenderManager.entityRenderMap` for OC's drone class (config `client.customDroneModel`).

## 5. Data formats

* Camera item NBT: `openfpv:{ Modules:[{Slot:b, id, Count, …}] }`. Per-environment persisted state is written by OC under the item's `oc:data` automatically via `save/load`.
* Drone item NBT (OC, used by the test harness): `oc:tier` (byte; 0=T1, 1=T2, 3=creative), `oc:components` (list of item stack NBT, **compact, no slot gaps**), `oc:storedEnergy` (int).
* EEPROM stacks for tests: `li.cil.oc.api.Items.registerEEPROM(name, codeBytes, dataBytes, readonly)`.
* `.bbmodel`: Blockbench 4.x "free" model; names used by code: groups `prop_fl/fr/bl/br`, `camera_pod`, `body`, `arms`, `skids`, `lights`; cubes `light_red/green/white/status`. Regenerate with `scripts/gen_drone_assets.py`.
