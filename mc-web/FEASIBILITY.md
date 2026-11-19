# Phase 1: feasibility result

Setup: Minecraft 26.3 client jar (SHA-1 e877b6a0..., class version 69 = Java 25), all non-native
libraries from the version JSON, TeaVM 0.16.0, WASM GC backend, entry point
`net.minecraft.client.main.Main`. Project: `mc-web/feasibility/`. Jar and tooling live in
`mc-web/work/` (gitignored, never committed).

## What works
- TeaVM reads the Java 25 class files and the whole dependency graph (client, Guava, Netty,
  DFU, Brigadier, fastutil, JOML, ICU, LWJGL Java classes, ...) without parser crashes.
- The compile needs a large heap (default ran out; `-Xmx24g` worked, about 2 minutes).
- Dependency analysis finishes and reports a closed list of what is missing, rather than failing
  on the first problem.

## What fails (analysis errors, not yet runtime behaviour)
About 96 distinct missing items, nearly all JDK APIs that TeaVM's class library lacks:

| Area | Examples |
|---|---|
| Concurrency | `ReentrantLock`, `ReentrantReadWriteLock`, `CompletableFuture`, `Executors`, `ForkJoinPool`, `LinkedBlockingQueue`, `ConcurrentLinkedQueue`, `Atomic*Array`, `ConcurrentHashMap.newKeySet` |
| Files / NIO | `FileChannel`, `FileLock`, `Path.resolve(String, String...)`, `Files.getFileStore`, `File.toPath` |
| Reflection / loaders | `ClassLoader.getResource(s)`, `Class.getModule`, `getGenericSuperclass`, `StackWalker`, `MethodHandles.lookup` |
| System / process | `System.load*`, `exit`, `getenv`, `Runtime.exec`, `addShutdownHook`, `maxMemory`, `ManagementFactory`, `Process` |
| Misc | `sun.misc.Unsafe`, `HexFormat`, `MessageDigest`, `UUID(long,long)`, `Locale.getScript`, `Math.fma`, `Thread.threadId`, `ScopedValue` |

Most of the Guava/JNA failures are consequences of the above (for example Guava segments extend
`ReentrantLock`).

LWJGL is not reported as missing: its Java classes are on the classpath, but they call native code.
That native layer is the shim work in Phase 3 and later, and is invisible to this analysis.

## Verdict
Feasible to continue. The missing set is small and well bounded, and each item can be handled with
a TeaVM class-library patch or a stub of our own. Not yet tested: whether the compiled output
runs (static analysis passing does not prove runtime correctness), startup size, memory use, and
the Worker threading design. Expect new errors to appear as each batch is fixed, because the
analysis only reports what is reachable from `Main` so far.

## Next
Phase 2: patch layer (our own replacements for the missing JDK classes), reproducible local build
script, and a first successful `.wasm` output.

## Phase 2 progress (in progress)

- New module `mc-web/classlib-ext/`: our own TeaVM class-library extension, written from scratch.
  Classes named `T<Name>` under `org.teavm.classlib.java...` replace the missing JDK classes.
  Done so far: `ReentrantLock`, `ReentrantReadWriteLock` (+ `Lock`, `Condition`, `ReadWriteLock`),
  `Future`, `FutureTask`, `ExecutorService`, `AbstractExecutorService`, `ThreadPoolExecutor`,
  `Executors`, `ForkJoinPool`, `ThreadFactory`, `ConcurrentLinkedQueue`, `LinkedBlockingQueue`,
  `TimeoutException`, `RejectedExecutionException`. Verified: the lock and Guava segment errors
  disappeared in the next TeaVM run.
- `log4j-core` and the log4j SLF4J binding are removed from the build (not needed in a browser).
- Heap: use `-Xmx11g`. A 24 GB setting gets the process killed on this 15 GB machine.
- Error count is 116 distinct items (was 125 after the first extension pass, 104 at the start).
  The count is not monotonic because fixing one layer lets TeaVM see the next.
- Remaining groups: `CompletableFuture`, atomic arrays, `LockSupport`, scheduled executors;
  `ClassLoader`/`Class`/`Package` resource and reflection methods; `System.exit/getenv/load*`,
  `Runtime.maxMemory/exec`; `File`/`NIO` channel methods; `Locale`, `UUID`, `Date`, `Character`,
  `Math` gaps; `java.net`/`javax.net.ssl` (HTTP, not needed offline); `sun.misc.Unsafe`
  (JOML, LWJGL, Guava); JNA internals; LWJGL `system` internals (the native shim, Phase 3).
- Each full compile takes about 8 minutes.
