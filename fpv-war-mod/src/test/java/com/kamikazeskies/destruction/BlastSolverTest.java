package com.kamikazeskies.destruction;

import static org.junit.jupiter.api.Assertions.*;

import java.util.Set;

import org.junit.jupiter.api.Test;

import com.kamikazeskies.destruction.core.BlastProfile;
import com.kamikazeskies.destruction.core.BlastSolver;
import com.kamikazeskies.destruction.core.BlockField;
import com.kamikazeskies.destruction.core.CellKey;
import com.kamikazeskies.sim.Vec3d;

class BlastSolverTest {
    /** A uniform block of material with the given resistance filling the whole world. */
    private static BlockField solid(double resistance) {
        return new BlockField() {
            public double resistance(int x, int y, int z) { return resistance; }
            public boolean isSolid(int x, int y, int z) { return true; }
        };
    }

    private static final BlockField AIR = new BlockField() {
        public double resistance(int x, int y, int z) { return 0; }
        public boolean isSolid(int x, int y, int z) { return false; }
    };

    private static final Vec3d C = new Vec3d(0.5, 0.5, 0.5);

    @Test
    void cellKeyRoundTripsIncludingNegatives() {
        int[][] pts = {{0, 0, 0}, {1, 2, 3}, {-1, -64, -1}, {-30000, 319, 29999}, {12345678, -5, -987654}};
        for (int[] p : pts) {
            long k = CellKey.pack(p[0], p[1], p[2]);
            assertEquals(p[0], CellKey.x(k)); assertEquals(p[1], CellKey.y(k)); assertEquals(p[2], CellKey.z(k));
        }
    }

    @Test
    void airOnlyWorldDestroysNothing() {
        assertTrue(BlastSolver.solve(AIR, C, BlastProfile.IMPACT, Vec3d.UP, 800, 1).destroyed().isEmpty());
    }

    @Test
    void impactBlastCarvesAnApproximatelySphericalCraterInSoftMaterial() {
        Set<Long> d = BlastSolver.solve(solid(0.5), C, BlastProfile.IMPACT, Vec3d.UP, 1500, 7).destroyed();
        assertTrue(d.size() > 80, "crater size " + d.size());
        assertTrue(d.contains(CellKey.pack(0, 0, 0)), "the centre block is destroyed");
        double maxR = 0;
        for (long k : d) {
            double dx = CellKey.x(k) + 0.5 - C.x(), dy = CellKey.y(k) + 0.5 - C.y(), dz = CellKey.z(k) + 0.5 - C.z();
            maxR = Math.max(maxR, Math.sqrt(dx * dx + dy * dy + dz * dz));
        }
        assertTrue(maxR < 7, "crater radius " + maxR);
        // roughly symmetric: similar reach up and down, left and right
        int up = 0, down = 0;
        for (long k : d) { if (CellKey.y(k) > 0) up++; if (CellKey.y(k) < 0) down++; }
        assertEquals(1.0, (double) up / down, 0.25);
    }

    @Test
    void harderMaterialMeansSmallerCrater() {
        int soft = BlastSolver.solve(solid(0.5), C, BlastProfile.IMPACT, Vec3d.UP, 1500, 3).destroyed().size();
        int wood = BlastSolver.solve(solid(3), C, BlastProfile.IMPACT, Vec3d.UP, 1500, 3).destroyed().size();
        int stone = BlastSolver.solve(solid(6), C, BlastProfile.IMPACT, Vec3d.UP, 1500, 3).destroyed().size();
        assertTrue(soft > wood && wood > stone, soft + " > " + wood + " > " + stone);
        assertTrue(stone >= 1, "even stone loses the centre block");
    }

    @Test
    void obsidianAndBedrockAreNotBroken() {
        assertTrue(BlastSolver.solve(solid(1200), C, BlastProfile.IMPACT, Vec3d.UP, 1500, 5).destroyed().isEmpty(), "obsidian survives");
        assertTrue(BlastSolver.solve(solid(Double.POSITIVE_INFINITY), C, BlastProfile.SHAPED, Vec3d.UP, 800, 5).destroyed().isEmpty());
    }

    @Test
    void anObsidianShieldProtectsWhatIsBehindIt() {
        BlockField shielded = new BlockField() {
            public double resistance(int x, int y, int z) { return x == 3 ? 1200 : 0.5; }   // obsidian wall at x=3
            public boolean isSolid(int x, int y, int z) { return true; }
        };
        Set<Long> d = BlastSolver.solve(shielded, C, BlastProfile.IMPACT, Vec3d.UP, 1500, 9).destroyed();
        boolean behind = d.stream().anyMatch(k -> CellKey.x(k) > 3);
        boolean front = d.stream().anyMatch(k -> CellKey.x(k) < 3);
        assertFalse(behind, "nothing behind the shield is destroyed");
        assertTrue(front);
        assertFalse(d.contains(CellKey.pack(3, 0, 0)), "the shield itself survives");
    }

    @Test
    void shapedChargePenetratesFarDeeperThanImpactAlongItsAxis() {
        Vec3d axis = new Vec3d(1, 0, 0);
        double impactDepth = depthAlongAxis(BlastSolver.solve(solid(6), C, BlastProfile.IMPACT, axis, 1500, 11).destroyed());
        double shapedDepth = depthAlongAxis(BlastSolver.solve(solid(6), C, BlastProfile.SHAPED, axis, 1500, 11).destroyed());
        assertTrue(shapedDepth >= impactDepth + 3, "shaped " + shapedDepth + " vs impact " + impactDepth);
        assertTrue(shapedDepth >= 4, "a shaped charge goes through ~5 blocks of stone, got " + shapedDepth);
        assertTrue(shapedDepth <= BlastProfile.SHAPED.maxRange() + 1, "range is capped");
    }

    @Test
    void shapedChargeIsNarrow() {
        Set<Long> d = BlastSolver.solve(solid(0.5), C, BlastProfile.SHAPED, new Vec3d(1, 0, 0), 1500, 13).destroyed();
        // everything destroyed lies in the forward half-space: nothing behind the charge
        assertTrue(d.stream().noneMatch(k -> CellKey.x(k) < -1), "no damage behind a shaped charge");
        int impact = BlastSolver.solve(solid(0.5), C, BlastProfile.IMPACT, new Vec3d(1, 0, 0), 1500, 13).destroyed().size();
        assertTrue(d.stream().filter(k -> Math.abs(CellKey.y(k)) > 4).count() < d.size() * 0.1 || d.size() < impact * 4);
    }

    @Test
    void sameSeedGivesIdenticalResultAndDifferentSeedsDiffer() {
        Set<Long> a = BlastSolver.solve(solid(1.0), C, BlastProfile.IMPACT, Vec3d.UP, 1000, 42).destroyed();
        Set<Long> b = BlastSolver.solve(solid(1.0), C, BlastProfile.IMPACT, Vec3d.UP, 1000, 42).destroyed();
        Set<Long> c = BlastSolver.solve(solid(1.0), C, BlastProfile.IMPACT, Vec3d.UP, 1000, 43).destroyed();
        assertEquals(a, b);
        assertNotEquals(a, c);
    }

    @Test
    void moreRaysFillTheCraterWithoutHoles() {
        // the centre-adjacent shell of blocks must all be destroyed in soft material
        Set<Long> d = BlastSolver.solve(solid(0.5), C, BlastProfile.IMPACT, Vec3d.UP, 1800, 17).destroyed();
        for (int x = -1; x <= 1; x++) for (int y = -1; y <= 1; y++) for (int z = -1; z <= 1; z++) {
            assertTrue(d.contains(CellKey.pack(x, y, z)), "hole at " + x + "," + y + "," + z);
        }
    }

    private static double depthAlongAxis(Set<Long> destroyed) {
        return destroyed.stream().filter(k -> Math.abs(CellKey.y(k)) <= 1 && Math.abs(CellKey.z(k)) <= 1)
                .mapToInt(CellKey::x).max().orElse(0);
    }
}
