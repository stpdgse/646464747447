package com.kamikazeskies.destruction.core;

/** Read-only view of the world used by the blast solver. */
public interface BlockField {
    /**
     * Blast resistance of the block at (x, y, z). {@code 0} for air/fluids that do not stop a blast.
     * {@link Double#POSITIVE_INFINITY} for blocks no blast may break (bedrock). Vanilla values: dirt 0.5, planks 3,
     * stone/cobblestone/bricks 6, obsidian 1200.
     */
    double resistance(int x, int y, int z);

    /** True if the block exists (is not air). Air still lets the blast travel but is never destroyed. */
    boolean isSolid(int x, int y, int z);
}
