package com.kamikazeskies.drone;

import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.item.DyeColor;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Blocks;

/**
 * A small practice course: a launch pad, a slalom of six gates, and a wall with a window to fly through. Built in
 * front of the player so new pilots can learn the controls, the OSD and the flight modes.
 */
public final class TrainingRange {
    /** Distance from the pad to each gate, sideways offset of each gate (positive = to the right), in blocks. */
    public static final int[] GATE_DISTANCE = {14, 28, 42, 56, 70, 84};
    public static final int[] GATE_OFFSET = {0, 4, -4, 5, -5, 0};
    public static final int WALL_DISTANCE = 102;

    private TrainingRange() {}

    /** Coloured concrete (colour blocks are a collection in this Minecraft version). */
    public static Block concrete(DyeColor color) { return Blocks.CONCRETE.pick(color); }

    /**
     * @param origin the block the pad is centred on (its top surface is at {@code origin.getY() - 1}, i.e. the ground)
     * @param facing the horizontal direction the course runs in
     * @return the number of blocks placed
     */
    public static int build(ServerLevel level, BlockPos origin, Direction facing) {
        Direction right = facing.getClockWise();
        int[] count = {0};
        int gy = origin.getY();

        // launch pad 7x7 flush with the ground
        for (int f = -3; f <= 3; f++) {
            for (int r = -3; r <= 3; r++) {
                put(level, origin, facing, right, f, r, gy - 1, (Math.abs(f) == 3 || Math.abs(r) == 3) ? concrete(DyeColor.YELLOW) : concrete(DyeColor.LIGHT_GRAY), count);
                for (int y = gy; y < gy + 3; y++) put(level, origin, facing, right, f, r, y, Blocks.AIR, count);   // clear headroom
            }
        }

        // gates: 7 wide x 6 tall frames of lime concrete, 5 x 4 opening, standing on short posts
        for (int k = 0; k < GATE_DISTANCE.length; k++) {
            int f = GATE_DISTANCE[k], off = GATE_OFFSET[k];
            Block frame = k == GATE_DISTANCE.length - 1 ? concrete(DyeColor.RED) : concrete(DyeColor.LIME);   // last gate is red
            for (int r = -3; r <= 3; r++) {
                for (int y = gy + 1; y <= gy + 6; y++) {
                    boolean ring = Math.abs(r) == 3 || y == gy + 1 || y == gy + 6;
                    put(level, origin, facing, right, f, off + r, y, ring ? frame : Blocks.AIR, count);
                }
            }
            put(level, origin, facing, right, f, off - 3, gy, concrete(DyeColor.WHITE), count);     // posts
            put(level, origin, facing, right, f, off + 3, gy, concrete(DyeColor.WHITE), count);
        }

        // wall with a 4 x 3 window to fly through
        for (int r = -7; r <= 7; r++) {
            for (int y = gy; y < gy + 9; y++) {
                boolean window = Math.abs(r) <= 1 + 1 && r >= -2 && r <= 1 && y >= gy + 3 && y <= gy + 5;
                put(level, origin, facing, right, WALL_DISTANCE, r, y, window ? Blocks.AIR : Blocks.STONE_BRICKS, count);
            }
        }
        return count[0];
    }

    /** Position of a gate's centre, for tests and tools. */
    public static BlockPos gateCenter(BlockPos origin, Direction facing, int index) {
        Direction right = facing.getClockWise();
        int f = GATE_DISTANCE[index], off = GATE_OFFSET[index];
        return origin.relative(facing, f).relative(right, off).above(3);
    }

    private static void put(ServerLevel level, BlockPos origin, Direction facing, Direction right, int f, int r, int y, Block b, int[] count) {
        BlockPos p = new BlockPos(
                origin.getX() + facing.getStepX() * f + right.getStepX() * r, y,
                origin.getZ() + facing.getStepZ() * f + right.getStepZ() * r);
        level.setBlock(p, b.defaultBlockState(), 2);
        count[0]++;
    }
}
