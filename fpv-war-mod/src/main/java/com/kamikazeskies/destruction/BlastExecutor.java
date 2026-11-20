package com.kamikazeskies.destruction;

import java.util.ArrayList;
import java.util.List;

import org.jspecify.annotations.Nullable;

import com.kamikazeskies.core.KamikazeSkies;
import com.kamikazeskies.core.KsConfig;
import com.kamikazeskies.destruction.core.BlastProfile;
import com.kamikazeskies.destruction.core.BlastSolver;
import com.kamikazeskies.destruction.core.CellKey;
import com.kamikazeskies.destruction.core.QualityPreset;
import com.kamikazeskies.sim.Vec3d;

import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.core.particles.ParticleTypes;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.sounds.SoundEvents;
import net.minecraft.sounds.SoundSource;
import net.minecraft.util.RandomSource;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.entity.item.FallingBlockEntity;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;

/** Applies a warhead's blast to the world: breaks blocks, throws debris, hurts entities, and triggers structural collapse. */
public final class BlastExecutor {
    /** Send to clients, no neighbour updates / shape updates / drops: cheap bulk removal. */
    private static final int REMOVE_FLAGS = 2 | 16 | 32;

    private BlastExecutor() {}

    public record Report(int blocksDestroyed, int debris, double radius) {}

    public static Report detonate(ServerLevel level, Vec3 center, Vec3 axis, BlastProfile profile, @Nullable Entity source) {
        QualityPreset q = KsConfig.QUALITY.get();
        RandomSource rnd = level.getRandom();

        BlastSolver.Result result = BlastSolver.solve(new LevelField(level), toSim(center), profile, toSim(axis), q.blastRays, rnd.nextLong());
        List<Long> cells = new ArrayList<>(result.destroyed());
        cells.sort((a, b) -> Double.compare(dist2(a, center), dist2(b, center)));
        double radius = cells.isEmpty() ? 0 : Math.sqrt(dist2(cells.get(cells.size() - 1), center));

        // --- blocks: some become flying debris (capped), the rest are simply removed ---
        int existing = level.getEntitiesOfClass(FallingBlockEntity.class, new AABB(center, center).inflate(96)).size();
        int debrisBudget = Math.max(0, Math.min(q.maxFlyingDebris, q.maxFallingBlocks - existing));
        int debris = 0, destroyed = 0;
        BlockPos.MutableBlockPos mp = new BlockPos.MutableBlockPos();
        for (long k : cells) {
            mp.set(CellKey.x(k), CellKey.y(k), CellKey.z(k));
            BlockState st = level.getBlockState(mp);
            if (st.isAir() || !st.getFluidState().isEmpty()) continue;
            destroyed++;
            boolean throwIt = debris < debrisBudget && st.isSolid() && isExposed(level, mp) && rnd.nextFloat() < 0.30f;
            if (throwIt) {
                BlockPos fixed = mp.immutable();
                FallingBlockEntity fb = FallingBlockEntity.fall(level, fixed, st);
                fb.dropItem = false;
                Vec3 out = new Vec3(fixed.getX() + 0.5, fixed.getY() + 0.5, fixed.getZ() + 0.5).subtract(center);
                out = out.lengthSqr() < 1e-6 ? new Vec3(0, 1, 0) : out.normalize();
                double speed = 0.30 + rnd.nextDouble() * 0.45;
                fb.setDeltaMovement(out.x * speed, out.y * speed + 0.25, out.z * speed);
                fb.setHurtsEntities(2.0f, 20);
                debris++;
            } else {
                level.setBlock(mp, Blocks.AIR.defaultBlockState(), REMOVE_FLAGS);
            }
        }

        hurtEntities(level, center, profile, Math.max(4.0, radius + 1.5), source);
        effects(level, center, radius, q, rnd);

        CollapseManager.schedule(level, BlockPos.containing(center), (int) Math.ceil(radius));
        if (com.kamikazeskies.drone.DemoMode.ENABLED) com.kamikazeskies.drone.DemoMode.onBlast(level);
        KamikazeSkies.LOGGER.info("Blast {} at {}: {} blocks destroyed ({} thrown), radius {}", profile.name(), BlockPos.containing(center),
                destroyed, debris, String.format("%.1f", radius));
        return new Report(destroyed, debris, radius);
    }

    private static void hurtEntities(ServerLevel level, Vec3 center, BlastProfile profile, double reach, @Nullable Entity source) {
        AABB box = new AABB(center, center).inflate(reach);
        for (Entity e : level.getEntities((Entity) null, box, ent -> ent != source && !(ent instanceof FallingBlockEntity) && ent.isAlive())) {
            Vec3 mid = e.position().add(0, e.getBbHeight() * 0.5, 0);
            double d = mid.distanceTo(center);
            if (d > reach) continue;
            double f = 1.0 - d / reach;
            float damage = (float) (profile.power() * 2.2 * f * f);
            if (e instanceof LivingEntity || e instanceof com.kamikazeskies.drone.JammerEntity) {
                e.hurtServer(level, level.damageSources().explosion(source, source), damage);
            }
            Vec3 away = mid.subtract(center);
            away = away.lengthSqr() < 1e-6 ? new Vec3(0, 1, 0) : away.normalize();
            e.pushFromExplosion(new Vec3(away.x * f * 1.2, away.y * f * 1.2 + 0.3 * f, away.z * f * 1.2));
            e.syncVelocity = true;
        }
    }

    private static void effects(ServerLevel level, Vec3 c, double radius, QualityPreset q, RandomSource rnd) {
        level.sendParticles(ParticleTypes.EXPLOSION_EMITTER, c.x, c.y, c.z, 1, 0, 0, 0, 0);
        int smoke = (int) Math.round(70 * q.particleScale);
        double spread = Math.max(1.0, radius * 0.35);
        if (smoke > 0) level.sendParticles(ParticleTypes.LARGE_SMOKE, c.x, c.y + 0.5, c.z, smoke, spread, spread * 0.8, spread, 0.02);
        int flash = (int) Math.round(12 * q.particleScale);
        if (flash > 0) level.sendParticles(ParticleTypes.EXPLOSION, c.x, c.y + 0.3, c.z, flash, spread, spread * 0.6, spread, 0.0);
        level.playSound(null, c.x, c.y, c.z, SoundEvents.GENERIC_EXPLODE.value(), SoundSource.BLOCKS, 4.0f, 0.75f + rnd.nextFloat() * 0.3f);
    }

    private static boolean isExposed(ServerLevel level, BlockPos p) {
        for (Direction d : Direction.values()) {
            if (level.getBlockState(p.relative(d)).isAir()) return true;
        }
        return false;
    }

    private static double dist2(long k, Vec3 c) {
        double dx = CellKey.x(k) + 0.5 - c.x, dy = CellKey.y(k) + 0.5 - c.y, dz = CellKey.z(k) + 0.5 - c.z;
        return dx * dx + dy * dy + dz * dz;
    }

    private static Vec3d toSim(Vec3 v) { return new Vec3d(v.x, v.y, v.z); }
}
