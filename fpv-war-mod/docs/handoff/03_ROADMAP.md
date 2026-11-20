# Roadmap (what is left)

Original plan lives in `docs/DESIGN.md` section "Phases". Done: 0-7. Remaining below, with concrete work items, acceptance checks and file hints.

## Immediate: on the real PC (do before new features) -> `08_REAL_PC_VERIFICATION.md`
FPS measurement, gamepad, workbench GUI, Iris shader compatibility, long-session check. Fix whatever it finds first.

## Phase 8 - Content completion, touch controls, audio, UI
1. **Audio (CC0 sounds, Tom approved CC0 sources).** Need: throttle-linked motor whine per motor fraction (server publishes `motorFraction()` already), arm/disarm beeps, low-battery and link-warning beeps (OSD already flashes), explosion layers (close/far), rifle shots (replace vanilla stand-in `SoundEvents.DISPENSER_LAUNCH` in `SoldierEntity.shootAt`), tank cannon (replace `GENERIC_EXPLODE` stand-in in `TankEntity`), footsteps/gear rattle, wind. Add `assets/kamikaze_skies/sounds.json` + `.ogg`, register `SoundEvent`s in `KsRegistries`. Sources: freesound.org CC0, kenney.nl, opengameart CC0. Record each file's source and licence in `docs/ASSETS.md` (create it). Mix: distance attenuation, muffled FPV audio while piloting (goggles) vs open-air audio.
2. **Touch / on-screen controls.** Implement `InputSource` for touch (two virtual sticks + buttons) in `client/`; show only when a touchscreen is detected or enabled in settings. Same pipeline as keyboard/gamepad (`input/`). Unit-test the stick mapping.
3. **Briefing UI / diegetic tablet.** A screen (or item) to pick a mission, read the briefing, see objectives and bonuses; HUD objective tracker (currently chat + overlay text). Mission results screen.
4. **More missions** (JSON + builders): recon-then-strike (spotter drone, mark targets), defend the operator (waves), bunker assault, convoy ambush. Extend `MissionDef` objective types (`kill`, `destroy_blocks/structure`, `survive`, `photograph/recon`, `reach`). Blueprint unlock rewards (tier gating from DESIGN).
5. **Anti-drone defences:** AA gun emplacement (model + animation through the art pipeline, entity, AI using `Marksmanship`), electronic-warfare vehicle that jams (reuse `RadioLink.jammingAt`), spotters that alert squads (squad communication: shared fear/awareness within radius).
6. **Free roam:** patrol/convoy spawns in the warzone dimension, villages ruined/intact variants, trench lines and wrecks as structure features, fuel/resource drops for the crafting economy.
7. **Gore (Tom: "some gore and blood, not extreme, good"):** blood particles/decals on hits, gibs for blast kills, tuned by a config toggle.
8. **Visual polish:** better terrain/biome for Eastern-European farmland (custom biome with field patterns, tree lines, dirt roads), recommended shader pack list (Complementary/Photon) tested on this GPU.
9. **Localisation + polish:** move all literal chat strings in `MissionManager` and commands into `lang/en_us.json`.

## Phase 9 - Multiplayer
Co-op missions first, then PvP dogfights; client-side prediction for the pilot view; per-player mission instances (MissionManager is already per-player, but the world pads are shared coordinates: give each instance its own region); later a racing mode (gates exist in `TrainingRange`).

## Backlog / known gaps
- Dimension terrain is plain noise terrain with a plains biome (can generate sea); tune with a custom noise setting or biome.
- Soldiers ignore each other's death rattle audio, have no squads, no patrol routes, no cover-seeking (they only crouch in place).
- Tank turret does not traverse independently (hull rotates). A separate turret yaw synched value + a bone rotation driven in a render hook would fix it.
- `MissionRegistry` reads built-in missions from the jar only (no datapack extensibility). Switch to a `SimpleJsonResourceReloadListener` equivalent if datapack missions are wanted.
- Demo world creation uses a flat world; the `warzone` dimension prompt must be clicked once (script does it with `xdotool` on Linux).
- Flight-feel preset values (open question in DESIGN): tune on the real machine with real gamepad.
- Throttle incremental mode exists in code but has no setting.
- Jammer and drone damage models: drones are invulnerable except via `shotDown()` and detonation; add health/armour variants by tier.
- Recipes balance not playtested.

## Suggested order for the next session
Real-PC verification -> audio -> briefing UI -> touch controls -> more missions -> AA/EW defences -> terrain/biome polish -> gore -> multiplayer.
