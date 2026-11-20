package com.kamikazeskies.destruction;

import static com.kamikazeskies.destruction.core.SupportSolver.*;
import static org.junit.jupiter.api.Assertions.*;

import java.util.HashSet;
import java.util.List;
import java.util.Set;

import org.junit.jupiter.api.Test;

import com.kamikazeskies.destruction.core.CellKey;
import com.kamikazeskies.destruction.core.SupportSolver;

class SupportSolverTest {
    /** 64-wide world, terrain slab at y=0..1, so structures start at y=2. The region boundary is far from the action. */
    private static Snapshot world() {
        Snapshot s = Snapshot.empty(-32, 0, -32, 64, 64, 64);
        for (int x = -32; x < 32; x++) for (int z = -32; z < 32; z++) { s.set(x, 0, z, TERRAIN, 1f); s.set(x, 1, z, TERRAIN, 1f); }
        return s;
    }

    private static void block(Snapshot s, int x, int y, int z) { s.set(x, y, z, STRUCTURE, 1f); }
    private static void block(Snapshot s, int x, int y, int z, float strength) { s.set(x, y, z, STRUCTURE, strength); }

    private static Set<Long> unsupported(Snapshot s) { return new HashSet<>(SupportSolver.findUnsupported(s, Params.DEFAULT)); }
    private static long k(int x, int y, int z) { return CellKey.pack(x, y, z); }

    @Test
    void anEmptyWorldHasNothingToCollapse() {
        assertTrue(unsupported(world()).isEmpty());
    }

    @Test
    void aTallColumnOnTheGroundStands() {
        Snapshot s = world();
        for (int y = 2; y < 40; y++) block(s, 0, y, 0);
        assertTrue(unsupported(s).isEmpty());
    }

    @Test
    void aFloatingBlockWithNothingBelowFalls() {
        Snapshot s = world();
        block(s, 0, 20, 0);
        assertEquals(Set.of(k(0, 20, 0)), unsupported(s));
    }

    @Test
    void aMutuallySupportingFloatingClusterStillFalls() {
        Snapshot s = world();
        for (int x = 0; x < 3; x++) for (int z = 0; z < 3; z++) for (int y = 20; y < 23; y++) block(s, x, y, z);
        assertEquals(27, unsupported(s).size());
    }

    @Test
    void aShortStoneOverhangHoldsAndALongOneBreaksAtTheTip() {
        Snapshot s = world();
        for (int y = 2; y < 8; y++) block(s, 0, y, 0);       // wall
        for (int x = 1; x <= 3; x++) block(s, x, 7, 0);      // 3-block overhang: fine
        assertTrue(unsupported(s).isEmpty());

        Snapshot t = world();
        for (int y = 2; y < 8; y++) block(t, 0, y, 0);
        for (int x = 1; x <= 9; x++) block(t, x, 7, 0);      // 9-block overhang: the outer part fails
        Set<Long> bad = unsupported(t);
        assertFalse(bad.isEmpty());
        assertTrue(bad.contains(k(9, 7, 0)), "the tip falls");
        assertFalse(bad.contains(k(1, 7, 0)), "the root stays");
        assertFalse(bad.contains(k(0, 7, 0)), "the wall stays");
        // contiguous from some point outward
        int firstBad = 99;
        for (int x = 1; x <= 9; x++) if (bad.contains(k(x, 7, 0))) { firstBad = Math.min(firstBad, x); }
        for (int x = firstBad; x <= 9; x++) assertTrue(bad.contains(k(x, 7, 0)), "gap at " + x);
        assertTrue(firstBad >= 3 && firstBad <= 6, "breaks after about 4 blocks, was " + firstBad);
    }

    @Test
    void aBridgeBetweenTwoPillarsHoldsUntilTheMiddleIsTooFarFromBothEnds() {
        // pillars at x=0 and x=span+1, deck at y=8
        for (int span : new int[] {6, 8}) {
            Snapshot s = world();
            for (int y = 2; y <= 8; y++) { block(s, 0, y, 0); block(s, span + 1, y, 0); }
            for (int x = 1; x <= span; x++) block(s, x, 8, 0);
            assertTrue(unsupported(s).isEmpty(), "span " + span + " should stand");
        }
        Snapshot s = world();
        int span = 12;
        for (int y = 2; y <= 8; y++) { block(s, 0, y, 0); block(s, span + 1, y, 0); }
        for (int x = 1; x <= span; x++) block(s, x, 8, 0);
        Set<Long> bad = unsupported(s);
        assertTrue(bad.contains(k(6, 8, 0)) || bad.contains(k(7, 8, 0)), "the middle gives out");
        assertFalse(bad.contains(k(1, 8, 0)));
        assertFalse(bad.contains(k(span, 8, 0)));
    }

    @Test
    void cuttingTheOnlyLinkToTheGroundDropsEverythingAbove() {
        Snapshot s = world();
        for (int y = 2; y < 12; y++) block(s, 0, y, 0);       // pillar
        for (int x = -1; x <= 1; x++) for (int z = -1; z <= 1; z++) block(s, x, 12, z);   // 3x3 platform on top
        assertTrue(unsupported(s).isEmpty());
        for (int y = 2; y < 5; y++) s.set(0, y, 0, AIR, 0);   // blast out the bottom of the pillar
        Set<Long> bad = unsupported(s);
        assertEquals(7 + 9, bad.size(), "pillar remainder + platform fall");
        assertTrue(bad.contains(k(0, 12, 0)));
    }

    @Test
    void aWideSlabOnASingleColumnIsAMushroomThatFails() {
        Snapshot s = world();
        for (int y = 2; y < 12; y++) block(s, 0, y, 0);
        for (int x = -2; x <= 2; x++) for (int z = -2; z <= 2; z++) block(s, x, 12, z);   // corners are 4 blocks from the column
        assertTrue(unsupported(s).contains(k(2, 12, 2)), "the corners are beyond the cantilever limit");
        assertFalse(unsupported(s).contains(k(0, 12, 0)));
    }

    @Test
    void aHolePunchedInAWallLeavesTheRestStanding() {
        Snapshot s = world();
        for (int x = -6; x <= 6; x++) for (int y = 2; y < 10; y++) block(s, x, y, 0);
        for (int x = -1; x <= 1; x++) for (int y = 4; y <= 6; y++) s.set(x, y, 0, AIR, 0);   // 3x3 hole
        assertTrue(unsupported(s).isEmpty(), "the lintel above the hole is carried by the wall to its sides");
    }

    @Test
    void weakMaterialsSpanLessThanStrongOnes() {
        Snapshot stone = world(), glass = world();
        for (int y = 2; y < 8; y++) { block(stone, 0, y, 0); block(glass, 0, y, 0); }
        for (int x = 1; x <= 4; x++) { block(stone, x, 7, 0, 1.0f); block(glass, x, 7, 0, 0.3f); }
        assertTrue(unsupported(stone).isEmpty(), "4 stone blocks hold");
        assertFalse(unsupported(glass).isEmpty(), "4 glass blocks do not");
    }

    @Test
    void structureTouchingTheRegionBoundaryIsAssumedSupportedFromOutside() {
        Snapshot s = Snapshot.empty(0, 0, 0, 8, 8, 8);           // no terrain at all in this slice
        for (int x = 0; x < 8; x++) block(s, x, 4, 0);          // a beam running along the boundary face z=0
        assertTrue(unsupported(s).isEmpty(), "analysing a slice of a larger structure must not invent collapses");
    }

    @Test
    void hangingFromTerrainAboveIsSupportedButCostsMore() {
        Snapshot s = Snapshot.empty(-8, 0, -8, 16, 16, 16);
        for (int x = -3; x <= 3; x++) for (int z = -3; z <= 3; z++) s.set(x, 12, z, TERRAIN, 1f);   // a cave ceiling
        for (int y = 9; y <= 11; y++) block(s, 0, y, 0);                                              // stalactite of blocks
        assertTrue(unsupported(s).isEmpty());
        for (int y = 2; y <= 11; y++) block(s, 0, y, 0);                                              // 10 long: too heavy to hang
        assertFalse(unsupported(s).isEmpty());
    }

    @Test
    void solvingIsFastEvenOnAFullyBuiltRegion() {
        Snapshot s = Snapshot.empty(0, 0, 0, 60, 60, 60);
        for (int i = 0; i < s.kind().length; i++) { s.kind()[i] = (i % 7 == 0) ? AIR : STRUCTURE; s.strength()[i] = 0.8f; }
        long t0 = System.nanoTime();
        SupportSolver.findUnsupported(s, Params.DEFAULT);
        long ms = (System.nanoTime() - t0) / 1_000_000;
        assertTrue(ms < 3000, "216k-cell region took " + ms + " ms");
    }

    @Test
    void resultIsIndependentOfScanOrderAndRepeatable() {
        Snapshot s = world();
        for (int y = 2; y < 10; y++) block(s, 0, y, 0);
        for (int x = 1; x <= 9; x++) block(s, x, 9, 0);
        List<Long> a = SupportSolver.findUnsupported(s, Params.DEFAULT);
        List<Long> b = SupportSolver.findUnsupported(s, Params.DEFAULT);
        assertEquals(a, b);
    }
}
