# 05 — Reference: Lua API, Config, Controls, Recipes, Items

## 1. Lua component `camera` (44 methods)

Get it: `local cam = component.proxy(component.list("camera")())`.
Methods marked **[module]** return `nil, "module not installed: <name>"` unless that module is in the camera's bay.
Most are `direct` (instant, no tick wait); `depth`/`color` are synchronised (run on the server thread).

### Core
| Method | Returns | Notes |
|---|---|---|
| `start()` | `true` / `nil,"no energy"` | Camera on (draws `energyPerTick` per tick). **The first call records the home point.** |
| `stop()` | `true` | Camera off; any pilot is returned to their body; the drone hovers. |
| `isActive()` | boolean | |
| `getRange()` | number | Metres/blocks, from the antenna: 64 / 160 / 400 / 800 / 1500 (configurable). |
| `getAntennaTier()` | number | 0 none, 1..4. |
| `getEnergyUsage()` | number | energy per second (= `energyPerTick*20`). |
| `getModules()` | table | e.g. `{"nightvision","zoom"}`; names: `nightvision, zoom, gimbal, thermal, depth`. |
| `getHeading()` | number | compass degrees 0=N, 90=E, incl. gimbal pan. |
| `setLights(on)` / `getLights()` | boolean | Nav lights + strobe on the visible model. |

### Modules
| Method | Module | Notes |
|---|---|---|
| `setNightVision(bool)` / `isNightVision()` | night vision | green amplified post-shader for the pilot |
| `setThermal(bool)` / `isThermal()` | thermal | heat palette; living things white-hot, sky/water cold |
| `setFov(deg)` / `getFov()` | zoom | 10–110, 70 normal; returns applied value |
| `setZoom(x)` / `getZoom()` | zoom | 1…8× (maps to FOV = 70/x, clamped 10) |
| `setTilt(deg)` / `getTilt()` | gimbal | −90…90, positive looks **up** |
| `setPan(deg)` / `getPan()` | gimbal | wraps (−180,180], positive looks **right** |
| `centerGimbal()` | gimbal | pan=tilt=0 |
| `depth(w,h[,maxDist])` | depth | rows (top→bottom) of distances in blocks, `-1` = no hit; grid follows FOV/pan/tilt and the drone facing; cost `w*h*energyPerRay`; limits `maxWidth×maxHeight` (32×16) and `maxDistance` (64) → returns `nil,"grid must be 1..32 wide and 1..16 high"` / `"not enough energy"` |
| `color(w,h[,maxDist])` | depth | same grid, block **map colour** (`MapColor.colorValue`, e.g. stone 7368816) |

### Navigation
| Method | Returns | Notes |
|---|---|---|
| `setHome()` | true | home = current position |
| `getHome()` | x,y,z or nil | |
| `returnHome()` | true / `nil,"no home point set"` | autopilot straight to home; slows on approach; stops within 1.5 blocks |
| `flyTo(x,y,z)` | true | autopilot to a world position |
| `isAutopilot()` | boolean | flyTo / returnHome / route in progress |
| `isReturning()` / `cancelReturn()` | boolean / true | cancel stops any autopilot, hovers |
| `getPosition()` | x,y,z | drone position |
| `addWaypoint(x,y,z)` | route length / `nil,"route is full (64 waypoints)"` | |
| `addWaypointHere()` | route length | |
| `getRoute()` | table of `{x,y,z}` | |
| `startRoute([loop])` | true / `nil,"the route is empty"` | one-shot (hover at last) or looping |
| `isRouteActive()` | boolean | |
| `getRouteProgress()` | index(1-based), count | stays at last waypoint after finishing |
| `clearRoute()` | true | also stops a running route |

Autopilot is cancelled when a pilot takes over. Triggers that start `returnHome` automatically: **signal loss**
(`failsafe.returnOnSignalLoss`) and **battery < `failsafe.lowBatteryReturn`** (default 15 %).

### Example
```lua
local cam = component.proxy(component.list("camera")())
local cl = component.list("chunkloader")()           -- keep the drone running when you are far away
if cl then component.invoke(cl, "setActive", true) end
cam.start()                                          -- also sets home
cam.setNightVision(true)
local x, y, z = cam.getPosition()
cam.addWaypoint(x+20, y+5, z); cam.addWaypoint(x+20, y+5, z+20); cam.addWaypoint(x, y, z+20)
cam.startRoute(true)                                 -- loop
while true do computer.pullSignal(1) end
```

## 2. Controls (defaults, rebindable in *Controls → OpenFPV*)

| Key | Action |
|---|---|
| **V** | Enter/leave FPV (nearest camera drone with signal) |
| **B** | Next camera drone |
| **W / S** | Pitch forward / back (angle: tilt ±35°; acro: rotate) |
| **A / D** | Bank left / right |
| **Space / Shift** | Throttle up / down |
| **Mouse X** or **← / →** | Yaw |
| **N** | Toggle ACRO / ANGLE |
| **R** | Start / stop DVR |
| **Esc** | Leave FPV (instead of the pause menu) |
Movement keys, mouse buttons and scroll are suppressed while flying; the player's body stays put.

## 3. Config (`config/openfpv.cfg`; also Mods → OpenFPV → Config)

| Key | Default | Meaning |
|---|---|---|
| `range.tiers` | 64,160,400,800,1500 | Range with no antenna, Mk1..Mk4 (list of 5, 16–20000) |
| `camera.energyPerTick` | 0.1 | energy/tick while active |
| `depth.energyPerRay` / `maxWidth` / `maxHeight` / `maxDistance` | 0.02 / 32 / 16 / 64 | |
| `client.noise` | true | static on weak signal |
| `client.customDroneModel` | true | animated quadcopter renderer (restart to apply) |
| `client.droneSounds` | true | |
| `client.ffmpegPath` | `ffmpeg` | |
| `flight.maxTilt` | 35 | angle-mode max bank/pitch (°) |
| `flight.attitudeResponse` | 0.2 | fraction of error closed per tick |
| `flight.accelScale` | 0.12 | blocks/tick² at 90° tilt (×sin) |
| `flight.throttleAccel` | 0.05 | vertical accel |
| `flight.yawRate` | 4.5 | °/tick at full stick |
| `flight.acroRate` | 12 | °/tick in acro |
| `flight.drag` | 0.9 | per-tick velocity retention while piloted (OC stock 0.8) |
| `flight.speedMultiplier` | 2.5 | × OC's 8 m/s cap while piloted |
| `flight.cameraUptilt` | 15 | camera mounting angle (°) |
| `flight.requireGoggles` | false | must wear FPV Goggles |
| `link.controlLag` / `maxLagTicks` / `maxPacketLoss` | true / 10 / 0.3 | |
| `failsafe.returnOnSignalLoss` / `lowBatteryReturn` | true / 0.15 | |
| `longrange.streamRadius` / `boostEntities` / `boostRadius` | 5 / true / 96 | chunk radius (also capped by server view distance), entity streaming |
Note: a stale `run/config/openfpv.cfg` may still contain obsolete keys (`camera.maxRange`, `flight.lead`); harmless.

## 4. Items and recipes (all shaped; OC parts resolved at runtime)

Legend: `I` iron ingot, `G` glass pane (camera/zoom/…) or gold ingot (antenna Mk2) — see each row, `R` redstone dust,
`P` printed circuit board, `V` graphics card T1, `C` chip.
| Item | Pattern | Keys |
|---|---|---|
| Camera Upgrade | `IGI / RVR / IPI` | G=paneGlass, V=`graphicscard1` |
| Night Vision | `DGD / QCQ / IPI` | D=glowstone dust, G=paneGlass, Q=nether quartz, C=`chip2` |
| Zoom | `GAG / ICI / IPI` | G=paneGlass, A=gold ingot, C=`chip1` |
| Gimbal | `IXI / RCR / IPI` | X=piston, C=`chip1` |
| Thermal | `BQB / GCG / IPI` | B=blaze powder, Q=quartz, G=paneGlass, C=`chip2` |
| Depth Sensor | `ERE / LCL / IPI` | E=ender pearl, L=lapis, C=`chip2` |
| Antenna Mk1 | `IRI / RWR / IPI` | W=`wlancard1` |
| Antenna Mk2 | `GRG / RAR / GWG` | G=gold ingot, A=Mk1, W=`wlancard2` |
| Antenna Mk3 | `DCD / RAR / DWD` | D=diamond, C=`chip3`, A=Mk2, W=`wlancard2` |
| Antenna Mk4 | `EQE / CAC / EWE` | E=ender pearl, Q=`chipdiamond`, C=`chip3`, A=Mk3, W=`wlancard2` |
| FPV Goggles | `ISI / GCG / LPL` | S=string, G=paneGlass, C=`chip2`, L=leather |
(`P` = `printedcircuitboard` for all. Missing OC parts fall back to vanilla stand-ins with a log warning.)
Registry ids: `openfpv:camera_upgrade`, `night_vision_module`, `zoom_module`, `gimbal_module`, `thermal_module`,
`depth_module`, `antenna_mk1..4`, `fpv_goggles`. Creative tab: "OpenFPV".

## 5. Files the mod writes at runtime
* `config/openfpv.cfg`
* `<game dir>/openfpv/recordings/flight_YYYYMMDD_HHMMSS.mp4` (or `_frames/` PNGs) and `.csv`
  (`tick,x,y,z,yaw,pitch,roll,speed_ms,battery_pct,signal_pct,distance_m,lag_ms,mode`).

## 6. Protocol constants
`PacketViewEnd.reason`: 0 REFUSED, 1 SIGNAL_LOST, 2 DRONE_GONE, 3 CAMERA_OFF.
`PacketCameraState` flags bits: 1 active, 2 night, 4 thermal, 8 returning, 16 lights.
GUI id 1 = module bay. Channel name `openfpv`.
