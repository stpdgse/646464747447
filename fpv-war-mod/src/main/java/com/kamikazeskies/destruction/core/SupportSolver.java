package com.kamikazeskies.destruction.core;

import java.util.ArrayList;
import java.util.List;
import java.util.PriorityQueue;

/**
 * Structural stability, in the style of Valheim / 7 Days to Die. Natural terrain anchors the structure with full
 * support (1.0); support then spreads block to block and decays: a little going up (columns are cheap), a lot going
 * sideways (overhangs are not), and in between for hanging from above. Weak materials decay faster. A structure
 * block whose best support path ends at or below zero has nothing holding it up and collapses.
 *
 * <p>The decay model means no single collapse can create instability that this one pass missed: removing blocks that
 * have no path to the ground cannot reduce the support of blocks that do have one.
 */
public final class SupportSolver {
    public static final byte AIR = 0, TERRAIN = 1, STRUCTURE = 2;

    /** Decay per block moved, divided by the receiving material's strength factor. */
    public record Params(double up, double sideways, double down) {
        /** Tuned so stone overhangs ~4 blocks, planks ~3, glass ~1; columns stand ~80 blocks. */
        public static final Params DEFAULT = new Params(0.012, 0.22, 0.30);
    }

    /**
     * A copy of a region of the world. {@code kind[i]} is AIR/TERRAIN/STRUCTURE; {@code strength[i]} in (0, 1] scales
     * how well a structure block carries load (1 = stone-like). Structure blocks on the outer boundary of the region are
     * treated as supported from outside, so analysing a slice of a larger structure never causes false collapses.
     */
    public record Snapshot(int minX, int minY, int minZ, int sx, int sy, int sz, byte[] kind, float[] strength) {
        public Snapshot {
            if (kind.length != sx * sy * sz || strength.length != kind.length) throw new IllegalArgumentException("size mismatch");
        }

        public static Snapshot empty(int minX, int minY, int minZ, int sx, int sy, int sz) {
            return new Snapshot(minX, minY, minZ, sx, sy, sz, new byte[sx * sy * sz], new float[sx * sy * sz]);
        }

        public int index(int x, int y, int z) { return ((y - minY) * sz + (z - minZ)) * sx + (x - minX); }

        public boolean contains(int x, int y, int z) {
            return x >= minX && x < minX + sx && y >= minY && y < minY + sy && z >= minZ && z < minZ + sz;
        }

        public void set(int x, int y, int z, byte k, float s) { int i = index(x, y, z); kind[i] = k; strength[i] = s; }
        public byte kindAt(int x, int y, int z) { return kind[index(x, y, z)]; }
    }

    private SupportSolver() {}

    /** @return the packed coordinates ({@link CellKey}) of every structure block that has lost its support. */
    public static List<Long> findUnsupported(Snapshot s, Params p) {
        int n = s.kind().length;
        float[] support = new float[n];
        java.util.Arrays.fill(support, Float.NEGATIVE_INFINITY);
        PriorityQueue<long[]> pq = new PriorityQueue<>((a, b) -> Float.compare(Float.intBitsToFloat((int) b[0]), Float.intBitsToFloat((int) a[0])));

        // seeds: structure blocks touching terrain, and structure blocks on the region boundary
        for (int y = s.minY(); y < s.minY() + s.sy(); y++) {
            for (int z = s.minZ(); z < s.minZ() + s.sz(); z++) {
                for (int x = s.minX(); x < s.minX() + s.sx(); x++) {
                    if (s.kindAt(x, y, z) != STRUCTURE) continue;
                    int i = s.index(x, y, z);
                    boolean boundary = x == s.minX() || x == s.minX() + s.sx() - 1 || y == s.minY() || y == s.minY() + s.sy() - 1
                            || z == s.minZ() || z == s.minZ() + s.sz() - 1;
                    if (boundary) { offer(pq, support, i, 1.0f); continue; }
                    double best = Double.NEGATIVE_INFINITY;
                    best = Math.max(best, fromTerrain(s, p, x, y, z, x, y - 1, z, p.up()));         // terrain below: carries up
                    best = Math.max(best, fromTerrain(s, p, x, y, z, x + 1, y, z, p.sideways()));
                    best = Math.max(best, fromTerrain(s, p, x, y, z, x - 1, y, z, p.sideways()));
                    best = Math.max(best, fromTerrain(s, p, x, y, z, x, y, z + 1, p.sideways()));
                    best = Math.max(best, fromTerrain(s, p, x, y, z, x, y, z - 1, p.sideways()));
                    best = Math.max(best, fromTerrain(s, p, x, y, z, x, y + 1, z, p.down()));       // terrain above: hangs
                    if (best > Double.NEGATIVE_INFINITY) offer(pq, support, i, (float) best);
                }
            }
        }

        int[][] dirs = {{0, 1, 0}, {0, -1, 0}, {1, 0, 0}, {-1, 0, 0}, {0, 0, 1}, {0, 0, -1}};
        while (!pq.isEmpty()) {
            long[] top = pq.poll();
            int i = (int) (top[1]);
            float sup = Float.intBitsToFloat((int) top[0]);
            if (sup < support[i]) continue;                    // stale entry
            int cx = s.minX() + i % s.sx(), cz = s.minZ() + (i / s.sx()) % s.sz(), cy = s.minY() + i / (s.sx() * s.sz());
            for (int[] d : dirs) {
                int nx = cx + d[0], ny = cy + d[1], nz = cz + d[2];
                if (!s.contains(nx, ny, nz) || s.kindAt(nx, ny, nz) != STRUCTURE) continue;
                // moving from the current block to the neighbour: neighbour above = carried up, below = hanging
                double cost = d[1] > 0 ? p.up() : d[1] < 0 ? p.down() : p.sideways();
                int ni = s.index(nx, ny, nz);
                double f = Math.max(0.05, Math.min(s.strength()[i], s.strength()[ni]));
                double ns = sup - cost / f;
                if (ns > support[ni]) offer(pq, support, ni, (float) ns);
            }
        }

        List<Long> out = new ArrayList<>();
        for (int y = s.minY(); y < s.minY() + s.sy(); y++) {
            for (int z = s.minZ(); z < s.minZ() + s.sz(); z++) {
                for (int x = s.minX(); x < s.minX() + s.sx(); x++) {
                    if (s.kindAt(x, y, z) == STRUCTURE && support[s.index(x, y, z)] <= 0) out.add(CellKey.pack(x, y, z));
                }
            }
        }
        return out;
    }

    private static double fromTerrain(Snapshot s, Params p, int x, int y, int z, int tx, int ty, int tz, double cost) {
        if (!s.contains(tx, ty, tz) || s.kindAt(tx, ty, tz) != TERRAIN) return Double.NEGATIVE_INFINITY;
        double f = Math.max(0.05, s.strength()[s.index(x, y, z)]);
        return 1.0 - cost / f;
    }

    private static void offer(PriorityQueue<long[]> pq, float[] support, int i, float value) {
        if (value > support[i]) {
            support[i] = value;
            pq.add(new long[] {Float.floatToIntBits(value) & 0xFFFFFFFFL, i});
        }
    }
}
