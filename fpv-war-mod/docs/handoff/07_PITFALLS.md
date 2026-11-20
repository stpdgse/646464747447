# Pitfalls already hit (read before debugging)

See also the phase lessons in `docs/TESTING.md`. Highlights:

## Minecraft 26.x / NeoForge API
- Config types are `LOCAL|CLIENT|SYNCED|STARTUP`; SYNCED values unreadable at common setup.
- `Blocks.<COLOR>_CONCRETE` does not exist: use `Blocks.CONCRETE.pick(DyeColor.X)`. Vanilla tag contents changed (`#dirt` no longer has grass).
- Entity APIs seen: `hurtServer(ServerLevel, DamageSource, float)`, `customServerAiStep(ServerLevel)`, `snapTo`, `spawnAtLocation(ServerLevel, ItemStack)` (no `drop(ItemStack,boolean)`), `ServerPlayer.teleportTo(level,x,y,z,Set.of(),yaw,pitch,true)`, `entityTags()`, `sendOverlayMessage`, `sendSystemMessage`, `BuiltInRegistries.ITEM.getValue(Identifier)`. Identifiers are `net.minecraft.resources.Identifier`.
- Dimension type JSON now needs attributes/timelines/clock; we avoid it by reusing `minecraft:overworld`. Any non-vanilla dimension triggers the vanilla "Experimental Settings" prompt once per world.
- `Mob.tickDeath` removes the entity after 20 ticks: override it for corpses/wrecks (done for soldier/tank). Death tilt in GeckoLib: `getDeathMaxRotation`.
- LWJGL has SDL3 (no GLFW in MC 26): gamepad via SDL.
- Data components, not NBT; storage via `ValueInput/ValueOutput`.

## GameTests
- **Entities not ticking is the #1 flake.** Force-load chunks (`KsGameTests.keepAreaTicking`), give every test its own solo environment, spawn with `h.spawn`, wait with `succeedWhen` on counters (`aiTicks()`, `simTicks()`, `deathTime`), never fixed delays; entities added to a chunk that is not fully loaded are invisible to queries until it is.
- Mock players (`makeMockServerPlayerInLevel`, deprecated) are not killable and are not in the player list: keep a direct reference and post `LivingDeathEvent` manually (see `MissionGameTests`).
- Without a floor a spawned mob falls into the void and is removed -> use `KsGameTests.floor`.
- A hovering test drone may drift; assert "ever fired" via latches, not a final state.

## Rendering / client
- FPV post-effect: use `LocalPlayer.setActivePostEffects` (per-tick additions are wiped).
- GeckoLib renderer generics: use `EntityRenderState` and raw `RenderPassInfo` overrides (ClassCastException otherwise).
- Cloud-only: OpenGL fails (no GLX visual) - expected; Vulkan on lavapipe used. On the real PC the OpenGL backend should just work.

## Art
- Euler/axis conventions: see `05_ART_PIPELINE.md` (mirror x for Bedrock animation files!).
- Skipped keyframes break loops; atlas full errors mean enlarge the atlas.

## Process
- Do not put "Failed to create backend OpenGL" in a fatal-error regex (it is expected on the cloud).
- Stale Xvfb/Java processes (`pkill -9 Xvfb; pkill -9 java; rm /tmp/.X98-lock`) can make demo scripts hang (Linux).
- Always give long-running commands caps/timeouts.
- Check visually (screenshots/GIFs) before calling visual work done.
