package com.kamikazeskies.destruction;

import com.kamikazeskies.destruction.core.BlockField;

import net.minecraft.core.BlockPos;
import net.minecraft.world.level.Level;
import net.minecraft.world.level.block.state.BlockState;

/** {@link BlockField} backed by a live level. Main thread only. */
public final class LevelField implements BlockField {
    private final Level level;
    private final BlockPos.MutableBlockPos pos = new BlockPos.MutableBlockPos();

    public LevelField(Level level) { this.level = level; }

    private BlockState state(int x, int y, int z) {
        if (y < level.getMinY() || y >= level.getMaxY()) return net.minecraft.world.level.block.Blocks.AIR.defaultBlockState();
        return level.getBlockState(pos.set(x, y, z));
    }

    @Override
    public double resistance(int x, int y, int z) {
        BlockState s = state(x, y, z);
        if (s.isAir() || !s.getFluidState().isEmpty()) return 0;
        float r = s.getBlock().getExplosionResistance();
        return r >= MaterialClassifier.UNBREAKABLE ? Double.POSITIVE_INFINITY : r;
    }

    @Override
    public boolean isSolid(int x, int y, int z) {
        BlockState s = state(x, y, z);
        return !s.isAir() && s.getFluidState().isEmpty();
    }
}
