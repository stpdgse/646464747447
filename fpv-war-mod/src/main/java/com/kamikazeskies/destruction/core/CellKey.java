package com.kamikazeskies.destruction.core;

/** Packs integer block coordinates into a long (same layout idea as Minecraft's BlockPos, kept Minecraft-free). */
public final class CellKey {
    private static final int XZ_BITS = 26, Y_BITS = 12;
    private static final long XZ_MASK = (1L << XZ_BITS) - 1, Y_MASK = (1L << Y_BITS) - 1;

    private CellKey() {}

    public static long pack(int x, int y, int z) {
        return ((x & XZ_MASK) << (XZ_BITS + Y_BITS)) | ((z & XZ_MASK) << Y_BITS) | (y & Y_MASK);
    }

    public static int x(long k) { return (int) (k << (64 - XZ_BITS - XZ_BITS - Y_BITS) >> (64 - XZ_BITS)); }
    public static int y(long k) { return (int) (k << (64 - Y_BITS) >> (64 - Y_BITS)); }
    public static int z(long k) { return (int) (k << (64 - XZ_BITS - Y_BITS) >> (64 - XZ_BITS)); }
}
