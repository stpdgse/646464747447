package com.kamikazeskies.destruction.core;

import java.util.HashSet;
import java.util.Set;
import java.util.SplittableRandom;

import com.kamikazeskies.sim.Vec3d;

/**
 * Computes which blocks a blast destroys by casting energy rays from the centre, like vanilla explosions: every ray
 * starts with {@code power * (0.7..1.3)} energy and loses energy to air distance and to each block's resistance; a
 * block is destroyed when a ray still has energy after reaching it. High-resistance blocks (obsidian) shield what is
 * behind them; bedrock-like blocks are never destroyed. Deterministic for a given seed.
 */
public final class BlastSolver {
    private BlastSolver() {}

    public record Result(Set<Long> destroyed, int raysCast) {}

    /**
     * @param center centre of the explosion in block coordinates (block (0,0,0) spans 0..1)
     * @param axis   jet direction for directional profiles (ignored for omnidirectional ones)
     * @param rays   number of rays; more rays give smoother, less noisy shapes (omnidirectional: spread over the
     *               whole sphere; directional: spread over the cone)
     */
    public static Result solve(BlockField world, Vec3d center, BlastProfile profile, Vec3d axis, int rays, long seed) {
        SplittableRandom rng = new SplittableRandom(seed);
        Set<Long> destroyed = new HashSet<>();
        Vec3d ax = axis.normalized();
        boolean directional = profile.isDirectional() && ax.lengthSq() > 0;
        double cosCone = Math.cos(Math.toRadians(profile.coneHalfAngle()));

        // Fibonacci lattice: well-spread directions without clumping
        double golden = Math.PI * (3 - Math.sqrt(5));
        for (int i = 0; i < rays; i++) {
            Vec3d dir;
            if (!directional) {
                double y = 1 - 2.0 * (i + 0.5) / rays;
                double r = Math.sqrt(Math.max(0, 1 - y * y));
                dir = new Vec3d(Math.cos(golden * i) * r, y, Math.sin(golden * i) * r);
            } else {
                dir = coneDirection(ax, cosCone, (i + 0.5) / rays, golden * i);
            }
            castRay(world, center, dir, profile, rng.nextDouble(), destroyed);
        }
        return new Result(destroyed, rays);
    }

    private static Vec3d coneDirection(Vec3d axis, double cosCone, double t, double phi) {
        double cosT = 1 - t * (1 - cosCone);          // uniform over the spherical cap
        double sinT = Math.sqrt(Math.max(0, 1 - cosT * cosT));
        Vec3d helper = Math.abs(axis.y()) < 0.99 ? Vec3d.UP : new Vec3d(1, 0, 0);
        Vec3d u = axis.cross(helper).normalized();
        Vec3d v = axis.cross(u);
        return axis.scale(cosT).add(u.scale(Math.cos(phi) * sinT)).add(v.scale(Math.sin(phi) * sinT));
    }

    private static void castRay(BlockField w, Vec3d c, Vec3d dir, BlastProfile p, double rand, Set<Long> out) {
        double energy = p.power() * (0.7 + 0.6 * rand);
        double x = c.x(), y = c.y(), z = c.z();
        double dx = dir.x() * BlastProfile.STEP, dy = dir.y() * BlastProfile.STEP, dz = dir.z() * BlastProfile.STEP;
        double airLoss = p.airAttenuation() * BlastProfile.STEP;
        int maxSteps = (int) Math.ceil(p.maxRange() / BlastProfile.STEP);
        int steps = 0;
        while (energy > 0 && steps++ < maxSteps) {
            int bx = (int) Math.floor(x), by = (int) Math.floor(y), bz = (int) Math.floor(z);
            double res = w.resistance(bx, by, bz);
            if (Double.isInfinite(res)) return;               // unbreakable: the ray stops here
            energy -= (res + 0.3) * BlastProfile.STEP;
            if (energy > 0 && w.isSolid(bx, by, bz)) out.add(CellKey.pack(bx, by, bz));
            energy -= airLoss;
            x += dx; y += dy; z += dz;
        }
    }
}
