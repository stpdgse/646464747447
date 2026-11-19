# mc-web — Plan

Private, local-only port of Minecraft: Java Edition to the browser.
Not published, not redistributed. Built on the owner's machine from the owner's own client jar.

## Decisions

| Topic | Decision |
|---|---|
| Target version | 26.3 (latest release; Mojang ships unobfuscated jars) |
| Scope | Singleplayer only (integrated server in the browser), no multiplayer |
| Gameplay goals | Survival (mining, crafting, mobs) and Creative |
| Dimensions | Overworld, Nether, End |
| Compiler | TeaVM, WASM GC backend |
| Code origin | Written from scratch. No Eaglercraft code. Only TeaVM and WASM GC are reused tooling. |
| Native layer | Our own LWJGL-compatible shim in Java (GLFW, OpenGL to WebGL2, OpenAL to Web Audio, STB). Game code stays unmodified where possible. |
| Audio | Web Audio API behind an OpenAL-style shim |
| Threads | Designed for Web Workers from day one (needs cross-origin isolation headers: COOP/COEP). Keep trying workers even if costly. |
| Failures | Patch around TeaVM gaps. If it becomes far too bad, switch to another approach (ask first). |
| Assets | Build step reads the owner's client jar locally and bakes assets into build output. Nothing from Mojang is committed to git. |
| Saves | Browser storage (OPFS / IndexedDB) |
| Platforms | Desktop Chrome first. iPad Chrome later. |
| Controls | Keyboard + mouse with pointer lock. Touch controls for iPad in a later phase. |
| UI additions | Loading screen with progress bar, pre-launch settings (memory, render distance), debug/performance overlay |
| Performance | Get it working first, optimize after |
| Verification | Screenshots and logs from headless Chromium |
| Location | `mc-web/` in this repo, English docs and comments |
| Workflow | Phased, report after each phase |

## Phases

1. **Feasibility.** Download the 26.3 client jar from Mojang for local use. Set up Gradle and TeaVM. Try compiling the jar to WASM GC and catalogue what fails (missing classes, unsupported Java features, LWJGL references).
2. **Build pipeline.** Reproducible local build: fetch jar, extract assets, compile, bundle, serve with COOP/COEP headers.
3. **Platform shim, part 1.** GLFW window/input and OpenGL to WebGL2 enough to render the title screen.
4. **Platform shim, part 2.** Audio, file I/O on OPFS, networking stubs for the integrated server.
5. **World rendering.** Superflat first, then normal terrain. Chunk meshing off the main thread.
6. **Gameplay.** Creative, then survival, mobs, Nether and End.
7. **Browser extras.** Loading screen, settings page, debug overlay.
8. **iPad.** Touch controls and memory tuning.

## Open risks

- TeaVM may not support everything the 26.3 client uses (Java 21+ features, reflection, invokedynamic, Netty, concurrency utilities).
- Worker-based threading with a Java game not built for it is the hardest architectural part.
- WASM GC memory limits and startup size, especially on iPad.
- GL 3.3 core features mapping to WebGL2.
