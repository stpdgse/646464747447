# Missions and the warzone (Phase 7)

## Warzone dimension
`data/kamikaze_skies/dimension/warzone.json`: the overworld dimension type with the overworld noise generator and a fixed plains biome, so the terrain is rolling grassland
with vanilla's plains villages, which is the Eastern-European farmland look. `/kamikaze warzone` teleports there. Vanilla shows its one-time "Experimental Settings" prompt
for any world that gains a custom dimension; confirm once ("I Know What I'm Doing"). The demo script clicks it automatically.

## Mission framework
- **Data-driven** (`data/kamikaze_skies/missions/<id>.json`, listed in `index.json`): briefing, time limit, enemy spawns (`soldier` with a role, `tank`), kill objectives, bonus goals, item rewards.
  `MissionDef` / `MissionState` / `MissionRegistry` are pure Java (`MissionTest`).
- **`MissionManager`** (server): `/kamikaze mission list | start <id> | status | abort`. Starting a mission force-loads the area, builds the ground (`Outpost`: levelled pad, dirt road, sandbagged trench,
  stone-brick bunker, fence, rubble; all ordinary blocks so blasts and the collapse solver apply), spawns the tagged enemy, teleports the player to the launch point in the warzone
  and hands out a drone if they have none. Kills are counted from death events; the operator dying or the clock running out fails the mission; completion pays the rewards and any bonus.
- **Outpost Alpha** (first mission): destroy the tank, eliminate four infantry (conscripts, a rifleman, a veteran), bonus for finishing within four minutes; rewards a tier-2 battery and motors.
- **Tests**: `MissionGameTests` (ground built and enemy spawned, completion pays rewards, operator death fails). `KS_DEMO_MODE=mission` flies a drone at the outpost in the real game.

## Not yet
Recon-then-strike and defend missions, mission selection UI (a briefing tablet), blueprint unlocks as rewards, convoys and patrol spawns in free roam, hand-built map templates.
