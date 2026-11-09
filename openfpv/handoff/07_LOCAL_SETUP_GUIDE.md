# 07 — Local Setup Guide (continue on your own machine)

## 1. What you need

* **Git**, **JDK 8** (Temurin/Adoptium 8; *not* 11/17/21 — ForgeGradle 3 + Gradle 4.9 need Java 8).
* Internet (Forge Maven, Mojang, CIL Maven, Modrinth for the OC jar).
* To *play*: Minecraft 1.12.2 with Forge 14.23.5.2860 (or your ATM3 install) + OpenComputers.
* Optional: `ffmpeg` on PATH (DVR mp4 recording).
* To run the **end-to-end self test** you also need a headless-capable GL stack. On **Linux**: `xvfb`, Mesa
  (`libgl1-mesa-dri`), `libopenal1`. On Windows/macOS you can instead run the dev client *with a real window*
  (`./gradlew runClient`), which is easier (see §5).
* Optional: **Claude Code** locally (see §7).

## 2. Get the code

Option A — from GitHub (needs access to `stpdgse/646464747447`):
```bash
git clone https://github.com/stpdgse/646464747447.git
cd 646464747447
git checkout claude/gallant-edison-yxpv26      # the OpenFPV work; PR #4
cd openfpv
```
Option B — from the zip: unzip `openfpv-handoff.zip`; the project is in `openfpv/`. A **git bundle**
(`openfpv-repo.bundle`, the whole repo incl. history) is included:
```bash
git clone openfpv-repo.bundle restored-repo && cd restored-repo && git checkout claude/gallant-edison-yxpv26
```

## 3. Build

```bash
export JAVA_HOME=/path/to/jdk8          # Windows (Git Bash): export JAVA_HOME="/c/Program Files/Eclipse Adoptium/jdk-8..."
./scripts/fetch-deps.sh                 # downloads libs/*.jar (OC API for compiling, OC 1.8.10 for running)
./gradlew build                         # jar: build/libs/openfpv-1.12.2-1.0.0.jar ; 38 unit tests
```
(Windows without bash: download the two jars manually — URLs are in `scripts/fetch-deps.sh` — then use `gradlew.bat build`.)

## 4. Use the mod in your game
Copy `build/libs/openfpv-1.12.2-1.0.0.jar` into the `mods/` folder of your Forge 1.12.2 / ATM3 profile (next to OpenComputers).
In game: creative tab **OpenFPV** → get *Camera Upgrade*, modules, antennas, goggles. Build a drone in an OC Assembler
(creative drone case lets you use many upgrades), flash an EEPROM with the example in doc 05 §1, power the drone on
(sneak + right-click it), stand near it and press **V**.
Quick creative setup: `/give @p openfpv:camera_upgrade` etc.; the bay GUI opens with right-click.

## 5. Run the dev client locally (with a window)

```bash
./scripts/fetch-deps.sh
./gradlew classes                       # first time: creates the Forge dev jars in ~/.gradle
./scripts/fix-dev-side-enum.sh          # REQUIRED workaround (bash; Linux/macOS/Git Bash) — see doc 03 §5
mkdir -p run/mods && cp libs/OpenComputers-MC1.12.2-1.8.10.jar run/mods/
./gradlew runClient                      # normal dev client; add -Pautotest=full to run the scripted scenario
```
If you skip the workaround you get `NullPointerException at NetworkRegistry.newChannel` at startup.
If the dev client cannot find models/lang, check that `build.gradle` still has the `resourcesDir` line.

## 6. Run the automated end-to-end test (Linux)
```bash
sudo apt install xvfb libgl1-mesa-dri libopenal1 ffmpeg python3 unzip zip curl
JAVA_HOME=/path/to/jdk8 ./scripts/run-selftest.sh      # ~6-9 min; expects 138 passes
```
Output: `run/autotest/summary.txt`, `run/autotest/screenshots/`. On macOS/Windows run `-Pautotest=full` with a real window
instead (the harness is window-independent but screenshots depend on window size).

## 7. Continue with Claude Code locally

1. Install Claude Code (see Anthropic docs), run `claude` in the repo root (`646464747447/`) or in `openfpv/`.
2. Paste the prompt from `handoff/00_START_HERE_PROMPT.md` (or just tell it to read the `handoff/` documents).
3. Give it the same working rules: test in the real game (harness), commit small, push to the branch, keep the PR updated,
   be honest about verification gaps.
4. Useful first tasks: run `./scripts/run-selftest.sh` locally; fly the drone yourself and write down feel feedback;
   then pick from doc 06 §D.

## 8. Git / PR housekeeping
* Branch: `claude/gallant-edison-yxpv26`. Draft PR #4 (base `claude/session-notes-migmyj`).
* Commit message style used: subject line `OpenFPV: …` + body bullets. (The cloud environment appended
  `Co-Authored-By` / `Claude-Session` trailers; not required locally.)
* Don't commit: `libs/*.jar`, `run*/`, `build/`, `.gradle/` (already in `.gitignore`).
* The release jar is also shipped in the zip root for convenience.

## 9. If something breaks
| Symptom | See |
|---|---|
| startup NPE `NetworkRegistry.newChannel` | doc 03 §5 (`fix-dev-side-enum.sh`) |
| `AIOOBE LinuxDisplay.getAvailableDisplayModes` | Xvfb needs `+extension RANDR` |
| dev client: `FileNotFoundException openfpv:models/…` | `resourcesDir` line in `build.gradle` |
| `Failed to get asset` | rerun `./gradlew downloadAssets` |
| drone doesn't move when you press keys (tests) | drone not started via `Drone.start()` / chunk unloaded |
| far drone disappears | add OC chunkloader upgrade and `setActive(true)` |
| no DVR mp4, only PNGs | `ffmpeg` not on PATH → set `client.ffmpegPath` |
