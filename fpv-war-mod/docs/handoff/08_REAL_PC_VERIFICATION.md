# First things to verify on the real PC (i5-9400F / GTX 1660)

Do these in order; write results into `docs/PERFORMANCE.md` (create it) and fix problems before adding features.

## A. Does it run?
1. `.\gradlew.bat build` then `.\gradlew.bat runGameTestServer` (expect all 55 pass; if some drone tests flake, note which, see `07_PITFALLS.md`).
2. `.\gradlew.bat runClient`; in-game: new Creative Flat world, `/kamikaze training`, fly a lap. Check the OSD, goggles view and the analog-video effect look right on a real GPU (cloud used software Vulkan; OpenGL is the real-GPU path).

## B. Frame rate (the 60 FPS target; this CPU is the likely bottleneck)
Settings: 1080p, render distance 12, vanilla (no shaders) first, then Iris + Complementary Reimagined (low profile). F3 FPS, note 1 % lows if possible (MC 26 has the F3 frame-time graph, `Shift+F3`).
| Scenario | How | Record |
|---|---|---|
| Idle in training range | `/kamikaze training` | FPS |
| Flying FPV at speed | fly the range | FPS, stutter |
| Big collapse | place `/kamikaze training`'s house, fly the warhead drone into it (use destructionQuality HIGH, then MEDIUM, LOW) | min FPS during collapse, time to settle |
| Enemy squad | `/kamikaze soldier` x10 + `/kamikaze tank`, then fly at them | FPS, AI tick cost |
| Mission | `/kamikaze mission start outpost_alpha` | FPS, chunk gen hitches |
If under target: profile (Spark mod or the built-in profiler `F3+L`), try `flightSubsteps=2`, `destructionQuality=MEDIUM`, fewer debris entities (`QualityPreset` in `destruction/core`), cheaper post-effect (`scripts/gen_fpv_post.py` + `FpvPostEffects`), entity render distance for soldiers (`clientTrackingRange`).

## C. Gamepad (never tested with hardware)
Plug an Xbox/PlayStation-style pad. Check: sticks map to Mode 2 (left = throttle/yaw, right = pitch/roll), A arms, Y cycles mode, B goggles off, deadzone and expo feel, hot-plug, keyboard/gamepad hand-over. Tune `GamepadSticks`/`StickMath` and the rate defaults in `sim/RateCurve`; consider a calibration screen. Also a radio transmitter (EdgeTX/OpenTX USB HID) would be a great test: any joystick SDL sees should work - verify axis order.

## D. GUI and interaction by hand
Drone Workbench: place it, right-click, assemble a drone from parts (recipe in `docs/CRAFTING.md`); shift-click, tooltips, flight log item reading. Sneak+right-click picks drones/jammers up. Keybinds menu "Kamikaze Skies - Flight".

## E. Visual checks cloud could not judge well
Real lighting on the soldier/tank/drone models (do textures look too dark/flat?), shaders (Iris compatibility with the post-effect and with GeckoLib entities), collapse dust/particles, terrain of the warzone dimension (sea inside the dimension? tune), the experimental-settings prompt, day/night.

## F. Sound (does not exist yet)
First feature after this verification: see Phase 8 audio in `03_ROADMAP.md`.

## G. Report format
Short table: scenario, FPS avg/low, settings, what hitched. Plus a list of bugs found. Then continue with the roadmap.
