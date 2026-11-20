# Setting up on the Windows PC (i5-9400F, 32 GB, GTX 1660)

Everything below was only ever run on Linux; Windows steps are standard NeoForge practice but **untested by me**. Fix and note anything that differs.

## 1. Install
1. **JDK 25** (Eclipse Temurin 25, x64 .msi). Check: `java -version` prints 25. (`gradle.properties` toolchain is pinned to 25; Gradle can also auto-download it via the foojay resolver if `settings.gradle` has it.)
2. **Git** and an IDE: IntelliJ IDEA Community (open the folder as a Gradle project) or VS Code with Java extensions. Optional.
3. **Python 3.10+** with `pip install pillow` (only to regenerate art/recipes; assets are already committed).
4. **Blockbench desktop app** (optional, only to look at models; the headless pipeline is Linux-only).
5. Latest NVIDIA driver. Minecraft 26.x tries OpenGL first; Vulkan backend exists as a fallback (the cloud ran Vulkan on software lavapipe).

## 2. Get the code
Unzip `kamikaze-skies-handoff.zip` into e.g. `C:\dev\kamikaze-skies\` (contains `fpv-war-mod/` and `tools/blockbench/`). If Tom also has the git repo (branch `claude/elegant-pascal-hr9x9b` in `stpdgse/646464747447`), prefer `git clone`/`git pull` and use the zip only as reference.

## 3. Build and run (PowerShell, inside `fpv-war-mod`)
```powershell
.\gradlew.bat build            # compiles + unit tests (first run downloads Minecraft, NeoForge, GeckoLib: several minutes)
.\gradlew.bat test             # pure-Java unit tests only
.\gradlew.bat runGameTestServer   # headless GameTests; expect "All 55 required tests passed"
.\gradlew.bat runClient        # real game window 1280x720
```
- In the cloud we used `--max-workers=1 -Dorg.gradle.parallel=false` only because of a proxy; **not needed on a normal connection**. `gradle.properties` has `org.gradle.parallel=true` and the configuration cache on.
- The GeckoLib Maven repo is `https://dl.cloudsmith.io/public/geckolib3/geckolib/maven/` (already in `build.gradle`).
- First `runClient` creates `run/`. A GameTest/clients write into `run/`; it is git-ignored.

## 4. Quick play loop (in the game)
1. Create a Creative world (Flat is fine) or use `/kamikaze warzone` for the dimension (confirm the one-time "Experimental Settings" prompt).
2. `/kamikaze training` builds a practice range in front of you and gives you a drone.
3. Place the drone (right-click ground with the item), right-click the drone to put on the goggles. Controls: see `docs/CONTROLS.md` (WASD pitch/roll, Q/E yaw, Space/Shift throttle, R arm, F assist mode, G goggles off). Arm needs throttle low.
4. `/kamikaze soldier veteran`, `/kamikaze tank`, `/kamikaze mission start outpost_alpha` to see the enemy and the mission.
5. Survival crafting: Drone Workbench + part recipes (`docs/CRAFTING.md`, `docs/RECIPES.md`); creative tab "Kamikaze Skies" has everything.

## 5. Useful dev switches
- Demo modes (scripted client demos used for screenshots): `.\gradlew.bat runClient -Pks.world=DemoWorld -Pks.demo=<mode>` where mode is `fly|model|strike|link|battery|parts|soldiers|tank|ai|mission`. Needs a world named `DemoWorld` in `run/saves` (create one in-game with that name, Flat/Peaceful/Creative). The Linux scripts in `scripts/*.sh` automate this with Xvfb and screenshots; on Windows use WSL2 or just run the gradle command and look.
- Config: `run/config/kamikaze_skies-*.toml` (see `01_PROJECT_STATE.md`). `destructionQuality=LOW` if collapses stutter.
- F3 shows FPS. For a proper measurement see `08_REAL_PC_VERIFICATION.md`.

## 6. Scripts (Linux only, here for reference)
`scripts/server-smoke.sh`, `client-screenshot.sh`, `make-demo-world.sh`, `demo-screenshots.sh` (Xvfb + Mesa lavapipe + hard caps). `scripts/gen_assets.py` (recipes/loot/models/lang for parts, runs anywhere with Python+Pillow), `scripts/gen_fpv_post.py` (post-effect shader JSON).

## 7. Regenerating assets (any OS with Python + Pillow)
`python art/build_rifleman.py`, `python art/tank.py`, `python art/tank_anim.py`, `python art/fpv_drone.py` write `.bbmodel`/`.png`/animation JSON into `art/out/`. The Bedrock export (`*.geo.json`) is done by Blockbench: open the `.bbmodel` in the desktop app, **File > Export > Export Bedrock Geometry**, then run `python art/install.py <name>` after copying the exported file to `art/out/export/export_bedrock_<name>.txt` (see `05_ART_PIPELINE.md`). Installed assets are already in `src/main/resources`, so this is only needed when changing a model.
