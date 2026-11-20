# Art pipeline (models, textures, animations)

Everything is generated from code and previewed through the real Blockbench (headless), so art is reproducible and reviewable.

```
art/rifleman.py          model: bones + box-UV cubes + procedural texture  -> art/out/rifleman.bbmodel + .png
art/rifleman_pose.py     pose solver: rifle held by the right hand, left hand glued to the foregrip by 2-bone IK
art/rifleman_anim.py     animation set (14 clips), each frame IK-solved, exported as deltas from the baked low-ready rest pose
art/build_rifleman.py    bakes the rest pose, writes the .bbmodel and animation JSON
tools/blockbench/bb-driver.js  opens the .bbmodel in headless Blockbench and exports the Bedrock geometry
tools/blockbench/bb-anim.js    renders frames of the exported animation with Blockbench's own animator (what you see is what ships)
art/install.py rifleman  copies geo (identifier geometry.kamikaze_skies.rifleman), animations and texture into src/main/resources
```

Steps: `python3 art/build_rifleman.py`, start `tools/blockbench/run-blockbench.sh`, `node tools/blockbench/bb-driver.js art/out/rifleman.bbmodel art/out/export`, `python3 art/install.py rifleman`.

## Conventions (all verified against Blockbench, not assumed)
- Authoring: bone rotation `R = Rz*Ry*Rx`, +x pitch swings a hanging limb forward (-z), +x is the entity's right. Pose solving happens in this frame.
- Bedrock/GeckoLib files mirror x: rotation `(-x, -y, z)`, position `(-x, y, z)`. `art/lib/anim.py` applies this when writing keys.
- Animations are additive to the bind pose, so the low-ready hold is baked into the model and clips store deltas.
- Every frame of a clip is solved, not hand-keyed: the right hand follows the rifle grip, the left hand follows the foregrip, the magazine
  or the pouch, so hands never float off the weapon. Tracks are written complete (no skipped keys) so loops close seamlessly.

## Rifleman clips
idle, walk, run, aim (hold), aim_idle, fire (recoil kick, muzzle climb, torso and head reaction), reload (pull in, drop mag, pouch, seat, return),
crouch (hold), crouch_idle, panic, flee, flinch, death_a (fall back), death_b (fall forward). The entity picks death_a/death_b from where the hit came from.

## Tests
`SoldierAssetsTest` (unit): every animated bone exists in the model, every animation the entity can request exists, loops close, one-shots do not loop.
`SoldierGameTests` (headless server): spawn, stance persistence, damage, corpse stays after the death animation.
`/kamikaze soldier [stance]` spawns one; `KS_DEMO_MODE=soldiers scripts/demo-screenshots.sh` plays the whole set in the real client.

## Tank
`art/tank.py` (34 cubes, 256x256 atlas, plus a procedurally burnt `tank_wreck.png`) and `art/tank_anim.py`: idle (engine rumble), drive (road wheels roll,
hull pitches on the suspension), turret_scan (turret and commander MG sweep independently), fire (barrel slides back 7 units, hull rocks, gun kicks),
destroyed (hull sags, turret thrown clear, gun droops; holds). The renderer swaps to the wreck texture when the tank dies.
`TankEntity`: 120 HP, unpushable, explodes on death (no terrain damage; the destruction system handles that), wreck stays 5 minutes.
`TankAssetsTest` + `TankGameTests` cover it. `/kamikaze tank` spawns one; `KS_DEMO_MODE=tank` plays the sequence.
Export a second model with `bb-driver.js ... art/out/export_tank`, copy `export_bedrock.txt` to `art/out/export/export_bedrock_tank.txt`, then `python3 art/install.py tank`.

## Drone (hero pass)
`art/fpv_drone.py` replaced the 16x16 palette placeholder with a 37-cube model on a 128x128 procedural atlas: twill carbon plates on metal standoffs,
ESC/FC boards with chips and traces, strapped LiPo with printed label, FPV camera with a glass lens and red cheeks, anodised motors, smoked
props with bright tips, VTX mast and receiver pod. Bone names (`body`, `arm_N`, `prop_N`) are unchanged, so the spin animation and renderer needed no code changes.
Export: `bb-driver.js art/out/fpv_drone.bbmodel art/out/export_drone`, copy `export_bedrock.txt` to `art/out/export/export_bedrock_fpv_drone.txt`, `python3 art/install.py fpv_drone`.
