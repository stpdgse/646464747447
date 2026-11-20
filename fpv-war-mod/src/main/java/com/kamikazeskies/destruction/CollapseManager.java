package com.kamikazeskies.destruction;

import java.util.ArrayDeque;
import java.util.ArrayList;
import java.util.Iterator;
import java.util.List;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;

import com.kamikazeskies.core.KamikazeSkies;
import com.kamikazeskies.core.KsConfig;
import com.kamikazeskies.destruction.core.CellKey;
import com.kamikazeskies.destruction.core.QualityPreset;
import com.kamikazeskies.destruction.core.SupportSolver;

import net.minecraft.core.BlockPos;
import net.minecraft.core.particles.BlockParticleOption;
import net.minecraft.core.particles.ParticleTypes;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.entity.item.FallingBlockEntity;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;

/**
 * Structural collapse after a blast. The world region around the blast is copied on the main thread, the stability
 * analysis runs on a worker thread, and the resulting collapse is applied on the main thread a few hundred blocks
 * per tick (the quality preset's budget), nearest-to-the-blast first so it spreads outward like a real failure.
 * Collapsing blocks become falling-block entities up to a cap; beyond it they crumble into dust.
 */
public final class CollapseManager {
    private static final ExecutorService POOL = Executors.newSingleThreadExecutor(r -> {
        Thread t = new Thread(r, "kamikaze-collapse");
        t.setDaemon(true);
        return t;
    });

    private record Pending(ServerLevel level, Vec3 origin, CompletableFuture<List<Long>> future) {}

    private static final class Job {
        final ServerLevel level;
        final long[] cells;
        int next;
        Job(ServerLevel level, long[] cells) { this.level = level; this.cells = cells; }
    }

    private static final List<Pending> PENDING = new ArrayList<>();
    private static final ArrayDeque<Job> JOBS = new ArrayDeque<>();

    /** Counters for tests and tuning. */
    public static final class Stats {
        public volatile int lastPerTick, maxPerTick;
        public volatile long converted, spawnedFalling, crumbled, jobsFinished;

        public void reset() { lastPerTick = 0; maxPerTick = 0; converted = 0; spawnedFalling = 0; crumbled = 0; jobsFinished = 0; }
    }

    public static final Stats STATS = new Stats();

    private CollapseManager() {}

    /** True while any analysis is running or any collapse is still being applied. */
    public static boolean busy() { return !PENDING.isEmpty() || !JOBS.isEmpty(); }

    /** Analyse the region around a blast asynchronously; the collapse is applied by {@link #tick}. */
    public static void schedule(ServerLevel level, BlockPos center, int blastRadius) {
        QualityPreset q = KsConfig.QUALITY.get();
        SupportSolver.Snapshot snap = snapshot(level, center, blastRadius + q.analysisPadding);
        Vec3 origin = Vec3.atCenterOf(center);
        PENDING.add(new Pending(level, origin, CompletableFuture.supplyAsync(() -> SupportSolver.findUnsupported(snap, SupportSolver.Params.DEFAULT), POOL)));
    }

    /** Synchronous analysis (tests, tools): the blocks that would collapse around {@code center}. */
    public static List<Long> analyzeNow(ServerLevel level, BlockPos center, int radius) {
        return SupportSolver.findUnsupported(snapshot(level, center, radius), SupportSolver.Params.DEFAULT);
    }

    /** Queue a known collapse list for budgeted application (tests, tools). */
    public static void enqueue(ServerLevel level, BlockPos origin, List<Long> cells) { addJob(level, Vec3.atCenterOf(origin), cells); }

    /** Server tick: finish completed analyses, then apply up to the preset's block budget. */
    public static void tick(MinecraftServer server) {
        for (Iterator<Pending> it = PENDING.iterator(); it.hasNext();) {
            Pending p = it.next();
            if (!p.future().isDone()) continue;
            it.remove();
            try {
                List<Long> cells = p.future().get();
                if (!cells.isEmpty()) addJob(p.level(), p.origin(), cells);
            } catch (Exception e) {
                KamikazeSkies.LOGGER.error("Collapse analysis failed", e);
            }
        }
        if (JOBS.isEmpty()) { STATS.lastPerTick = 0; return; }

        QualityPreset q = KsConfig.QUALITY.get();
        int budget = q.blocksPerTick, converted = 0;
        Job first = JOBS.peek();
        int existing = first.level.getEntitiesOfClass(FallingBlockEntity.class, new AABB(BlockPos.of(first.cells[0])).inflate(96)).size();
        int[] allowance = {Math.max(0, q.maxFallingBlocks - existing)};

        while (budget > 0 && !JOBS.isEmpty()) {
            Job j = JOBS.peek();
            while (budget > 0 && j.next < j.cells.length) {
                converted += convert(j.level, j.cells[j.next++], allowance, q) ? 1 : 0;
                budget--;
            }
            if (j.next >= j.cells.length) { JOBS.poll(); STATS.jobsFinished++; }
        }
        STATS.lastPerTick = converted;
        STATS.maxPerTick = Math.max(STATS.maxPerTick, converted);
    }

    public static void clear() { PENDING.clear(); JOBS.clear(); }

    // ---- internals ----------------------------------------------------------------------------------
    private static void addJob(ServerLevel level, Vec3 origin, List<Long> cells) {
        long[] sorted = cells.stream().sorted((a, b) -> Double.compare(d2(a, origin), d2(b, origin))).mapToLong(Long::longValue).toArray();
        JOBS.add(new Job(level, sorted));
        KamikazeSkies.LOGGER.info("Collapse: {} unsupported blocks queued near {}", sorted.length, BlockPos.containing(origin));
    }

    private static boolean convert(ServerLevel level, long key, int[] fallingAllowance, QualityPreset q) {
        BlockPos pos = new BlockPos(CellKey.x(key), CellKey.y(key), CellKey.z(key));
        if (!level.hasChunkAt(pos)) return false;
        BlockState st = level.getBlockState(pos);
        if (MaterialClassifier.kind(st) != SupportSolver.STRUCTURE) return false;   // changed since the snapshot
        STATS.converted++;
        if (fallingAllowance[0] > 0) {
            fallingAllowance[0]--;
            FallingBlockEntity fb = FallingBlockEntity.fall(level, pos, st);
            fb.dropItem = false;
            fb.setHurtsEntities(2.0f, 20);
            STATS.spawnedFalling++;
        } else {
            level.setBlock(pos, Blocks.AIR.defaultBlockState(), 2 | 16 | 32);
            if (q.particleScale > 0.5 && (key & 3) == 0) {
                level.sendParticles(new BlockParticleOption(ParticleTypes.BLOCK, st), pos.getX() + 0.5, pos.getY() + 0.5, pos.getZ() + 0.5, 4, 0.3, 0.3, 0.3, 0.05);
            }
            STATS.crumbled++;
        }
        return true;
    }

    private static SupportSolver.Snapshot snapshot(ServerLevel level, BlockPos c, int r) {
        int minX = c.getX() - r, minZ = c.getZ() - r;
        int minY = Math.max(level.getMinY(), c.getY() - r), maxY = Math.min(level.getMaxY() - 1, c.getY() + r);
        int sx = 2 * r + 1, sz = 2 * r + 1, sy = maxY - minY + 1;
        SupportSolver.Snapshot s = SupportSolver.Snapshot.empty(minX, minY, minZ, sx, sy, sz);
        BlockPos.MutableBlockPos mp = new BlockPos.MutableBlockPos();
        for (int y = minY; y <= maxY; y++) {
            for (int z = minZ; z < minZ + sz; z++) {
                for (int x = minX; x < minX + sx; x++) {
                    mp.set(x, y, z);
                    if (!level.hasChunkAt(mp)) { s.set(x, y, z, SupportSolver.TERRAIN, 1f); continue; }   // unloaded = supported from outside
                    BlockState st = level.getBlockState(mp);
                    byte kind = MaterialClassifier.kind(st);
                    if (kind != SupportSolver.AIR) s.set(x, y, z, kind, MaterialClassifier.strength(st));
                }
            }
        }
        return s;
    }

    private static double d2(long k, Vec3 o) {
        double dx = CellKey.x(k) + 0.5 - o.x, dy = CellKey.y(k) + 0.5 - o.y, dz = CellKey.z(k) + 0.5 - o.z;
        return dx * dx + dy * dy + dz * dz;
    }
}
