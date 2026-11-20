# CLAUDE.md - Kamikaze Skies

Minecraft NeoForge mod (MC 26.3, NeoForge 26.3.0.114, Java 25, GeckoLib 5.5.7) for Tom Pearl. Read `docs/handoff/00_START_HERE.md` first, then the rest of `docs/handoff/` and `docs/DESIGN.md`.

Rules: no placeholders or half-finished work; install real tools; research below 90 % confidence; speak English; fictional war setting (no real insignia/conflict, war content stays);
pure-Java cores + unit tests, glue + GameTests (solo environment each, wait on entity counters, never fixed delays); demos for visual features; hard caps on anything that launches the game;
milestone summaries with honest verified/unverified lists; commit and push often.

Commands: `./gradlew build`, `./gradlew test`, `./gradlew runGameTestServer`, `./gradlew runClient`. Phases 0-7 done; next: real-PC verification, then Phase 8 (see `docs/handoff/03_ROADMAP.md`).
