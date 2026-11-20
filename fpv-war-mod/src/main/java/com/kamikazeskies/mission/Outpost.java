package com.kamikazeskies.mission;

import net.minecraft.core.BlockPos;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockState;
import net.minecraft.world.level.levelgen.Heightmap;

/**
 * Builds the playable ground of a mission: a levelled pad around the enemy position, a dirt road, a sandbagged trench line, a concrete bunker and a
 * fence. Deterministic for a given origin; everything is made of ordinary blocks, so the drone's blasts and the collapse solver apply as usual.
 */
public final class Outpost {
    private Outpost() {}
    private static final int FLAGS = 2;                      // send to clients, no neighbour updates (fast, nothing drops)

    /** @return the pad's ground level (y of the top grass block) */
    public static int build(ServerLevel level, BlockPos origin) {
        int y = level.getHeight(Heightmap.Types.WORLD_SURFACE_WG, origin.getX(), origin.getZ() + 60) - 1;
        y = Math.max(level.getMinY() + 5, y);
        int x0 = origin.getX() - 28, x1 = origin.getX() + 32, z0 = origin.getZ() - 6, z1 = origin.getZ() + 84;
        BlockState air = Blocks.AIR.defaultBlockState(), grass = Blocks.GRASS_BLOCK.defaultBlockState(), dirt = Blocks.DIRT.defaultBlockState();
        for (int x = x0; x <= x1; x++) for (int z = z0; z <= z1; z++) {
            for (int yy = y + 14; yy > y; yy--) level.setBlock(new BlockPos(x, yy, z), air, FLAGS);      // clears trees and bumps
            level.setBlock(new BlockPos(x, y, z), grass, FLAGS);
            for (int d = 1; d <= 4; d++) level.setBlock(new BlockPos(x, y - d, z), dirt, FLAGS);
        }
        int ox = origin.getX(), oz = origin.getZ();
        for (int z = z0; z <= z1; z++) for (int w = -1; w <= 1; w++) level.setBlock(new BlockPos(ox + 14 + w + (z / 9 % 2), y, z), Blocks.DIRT_PATH.defaultBlockState(), FLAGS);   // the road, with a wobble
        // trench: two blocks wide, two deep, sandbag (hay) parapet on the enemy side
        for (int x = ox - 12; x <= ox + 8; x++) {
            for (int dz = 0; dz < 2; dz++) for (int d = 0; d < 2; d++) level.setBlock(new BlockPos(x, y - d, oz + 56 + dz), air, FLAGS);
            level.setBlock(new BlockPos(x, y + 1, oz + 58), Blocks.HAY_BLOCK.defaultBlockState(), FLAGS);
            level.setBlock(new BlockPos(x, y + 1, oz + 55), Blocks.COARSE_DIRT.defaultBlockState(), FLAGS);
        }
        bunker(level, ox - 4, y, oz + 72);
        for (int x = ox - 24; x <= ox - 14; x++) { level.setBlock(new BlockPos(x, y + 1, oz + 50), Blocks.OAK_FENCE.defaultBlockState(), FLAGS); }
        for (int i = 0; i < 6; i++) level.setBlock(new BlockPos(ox + 20 + i, y + 1, oz + 52 + i % 3), Blocks.COBBLESTONE.defaultBlockState(), FLAGS);          // rubble
        return y;
    }

    private static void bunker(ServerLevel level, int bx, int y, int bz) {
        BlockState wall = Blocks.STONE_BRICKS.defaultBlockState(), air = Blocks.AIR.defaultBlockState();
        for (int x = 0; x < 8; x++) for (int z = 0; z < 6; z++) for (int h = 1; h <= 3; h++) {
            boolean edge = x == 0 || x == 7 || z == 0 || z == 5;
            level.setBlock(new BlockPos(bx + x, y + h, bz + z), edge ? wall : air, FLAGS);
        }
        for (int x = 0; x < 8; x++) for (int z = 0; z < 6; z++) level.setBlock(new BlockPos(bx + x, y + 4, bz + z), wall, FLAGS);
        for (int h = 1; h <= 2; h++) level.setBlock(new BlockPos(bx + 3, y + h, bz), air, FLAGS);            // door facing the operator
        level.setBlock(new BlockPos(bx + 3, y + 3, bz), air, FLAGS);
        for (int x = 1; x < 7; x++) level.setBlock(new BlockPos(bx + x, y + 3, bz + 5), Blocks.AIR.defaultBlockState(), FLAGS);               // firing slit
    }
}
