# Decisions and how Tom likes to work

## Product decisions (from ~11 rounds of questions; `docs/DESIGN.md` has the full table)
- NeoForge on Minecraft Java 26.3, Java 25, **GeckoLib is the only hard dependency**. Independent of OpenFPV/OpenComputers (an older sibling project in the same repo; do not depend on it).
- Singleplayer + AI first; multiplayer later (co-op missions, then PvP dogfights, racing much later).
- Tone: gritty realistic war, "arcade-leaning realism", **fictional and not tied to any real conflict**; blood and gore: "some, not extreme, but good". He explicitly said to keep the war content.
- Controls: keyboard, gamepad, touch all **equal**; angle mode default; drone types: FPV strike + recon quad; analog video, OSD, signal loss; battery realistic; warheads impact/shaped/frag.
- Diegetic UI (goggles, radio, tablet); operator can be targeted; mixed mission campaign + free roam; craft/resource economy with tiers and mission rewards.
- Destruction: blast craters + structural collapse.
- Enemies: "reactive and scared"; vehicles, structures, defences, soldiers - each modelled individually and animated.
- Textures 64-128 (we use 128/256), shader packs recommended (Complementary/Photon) but not required.
- Audio: **CC0 sounds**, record sources/licences. License: All Rights Reserved. Title "Kamikaze Skies" (working).
- Target machines: RTX 3060 / Ryzen 7 5700X / 32 GB at 60 FPS (main PC, not available now) and this second PC (i5-9400F / 32 GB / GTX 1660) - the 1660 is a good "minimum spec" test: if it holds ~45-60 FPS with HIGH destruction quality, great; otherwise tune.
- Tom wants me (the AI) to do **all the coding** and to pick how to proceed; he checks in at milestones.

## Working preferences (user preferences text, verbatim intent)
- Never skimp or leave placeholders/half-finished work. If a task needs a real SDK/tool/package/infrastructure, install it - do not use a thinner workaround. Only stub when >90 % sure; otherwise research (search the web, read docs/source) first. No artificial limits on installing/downloading/compiling what a task needs. **Speak English, not German.** His name is Tom Pearl.
- He said "Yeah download what u want as long as it works and doesn't virus us."
- He dislikes hangs: every script that launches Minecraft must have a hard cap and fail fast on fatal log lines ("add a hard cap so we don't wait 10 minutes again").
- Big chunks of work, then a concise milestone message with what changed, what is verified, what is not, and screenshots/GIFs. Be honest about what is unverified.
- He gets annoyed by sloppy visuals (the rifle floating at the chest: "make it in his arms, make him hold it and fire animation realistically"). Check animations/models visually before claiming done.
- Commit and push regularly to the working branch; draft PR; stop hooks complain about uncommitted/untracked files.

## Git
Repo `stpdgse/646464747447` (a monorepo containing other projects: `openfpv`, `gbc-handheld`, `tools/`). This project: `fpv-war-mod/` + `tools/blockbench/`. Working branch `claude/elegant-pascal-hr9x9b`. Commit trailer lines are added by the cloud harness (not needed on the PC).
