# Enemy AI (Phase 6)

All decision logic is pure Java in `ai/` (unit-tested in `AiCoreTest`); `SoldierEntity` and `TankEntity` only sense the world and act on the result.

## Soldiers
- **Roles** (`Role`): conscript (courage 0.25, skill 0.25), rifleman (0.55 / 0.45), veteran (0.85 / 0.70), spawned 40 / 45 / 15 %. `/kamikaze soldier <role|stance>`.
- **Fear** (`FearModel`, 0..1): rises with an armed drone in sight, more the closer it is and the faster it closes (a 25 m/s dive is the worst case),
  spikes on nearby blasts (`SoldierEntity.alarm`, called by `BlastExecutor`) and on seeing a friend die; decays slowly once the threat is gone. Courage damps every source.
- **Behaviour** (`Brain`): IDLE (wander) -> ENGAGE (aim and fire) -> COVER (crouch; only conscripts hide, veterans fight on) -> FLEE (panic run away from the drone).
  Thresholds scale with courage, behaviours are held 10-60 ticks so nobody flickers.
- **Shooting** (`Marksmanship`): 30-round mags, a shot every 10 ticks, 56-tick reload. Hit chance falls with range, target speed squared and the shooter's fear:
  a hovering drone at 25 m is in real danger (about 30 % per round for a good shot), a drone at 20 m/s is nearly untouchable (under 3 %). A hit calls `DroneEntity.shotDown()`.
- **The operator can be targeted**: when the drone's pilot stands within 30 m in line of sight, soldiers sometimes shoot at the player instead (3 damage per hit).
- Sounds are vanilla stand-ins until the audio phase.

## Tank
Scans with its turret until it sees an armed drone within 64 m, then traverses the hull onto it (3 degrees per tick; the turret is fixed in this model) and fires the cannon every
4 s once aligned. `Marksmanship.cannonChance`: dangerous only to a hovering drone, hopeless against a fast one. Tanks never flee.

## Tests
`AiGameTests` (headless server): flees when terrified, veteran engages, conscript takes cover and does not fire, blast alarm scales with distance, friend's death spreads fear,
calm stays idle, tank traverses onto a drone. All tests now use isolated environments so entity ticking is deterministic. `KS_DEMO_MODE=ai` shows a squad and a diving drone.

## Not yet
Anti-drone guns and electronic-warfare vehicles, patrol routes, squad communication and mission objectives (Phase 7).
