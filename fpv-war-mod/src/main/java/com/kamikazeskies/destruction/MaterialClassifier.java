package com.kamikazeskies.destruction;

import com.kamikazeskies.destruction.core.SupportSolver;

import net.minecraft.tags.BlockTags;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.level.block.state.BlockState;
import net.neoforged.neoforge.common.Tags;

/**
 * Decides how a block takes part in structural stability:
 * <ul>
 *   <li><b>TERRAIN</b> (natural ground, bedrock): anchors structures and never collapses.</li>
 *   <li><b>STRUCTURE</b> (anything built: planks, bricks, stone, glass...): carries load and can collapse.</li>
 *   <li><b>AIR</b> (air, fluids, plants, leaves, non-solid blocks): takes no part.</li>
 * </ul>
 */
public final class MaterialClassifier {
    /** Resistance at or above this counts as unbreakable (bedrock is 3,600,000). */
    public static final float UNBREAKABLE = 3_000_000f;

    private MaterialClassifier() {}

    public static byte kind(BlockState s) {
        if (s.isAir() || !s.getFluidState().isEmpty() || s.is(BlockTags.LEAVES) || !s.isSolid()) return SupportSolver.AIR;
        if (isTerrain(s)) return SupportSolver.TERRAIN;
        return SupportSolver.STRUCTURE;
    }

    /**
     * Natural ground. Vanilla's {@code #dirt} tag no longer contains grass blocks, so several tags are combined:
     * soil that supports vegetation (grass, podzol, mycelium, farmland...), sand, natural stones, ores, gravel,
     * netherrack/end stone, mud and clay, plus anything unbreakable (bedrock). Built blocks such as cobblestone,
     * bricks and planks are deliberately <i>not</i> terrain.
     */
    public static boolean isTerrain(BlockState s) {
        return s.is(BlockTags.DIRT) || s.is(BlockTags.SUPPORTS_VEGETATION) || s.is(BlockTags.MUD) || s.is(BlockTags.SAND)
                || s.is(BlockTags.BASE_STONE_OVERWORLD) || s.is(BlockTags.BASE_STONE_NETHER)
                || s.is(Tags.Blocks.STONES) || s.is(Tags.Blocks.ORES) || s.is(Tags.Blocks.GRAVELS) || s.is(Tags.Blocks.NETHERRACKS)
                || s.is(Tags.Blocks.END_STONES) || s.is(Tags.Blocks.SANDS)
                || s.is(Blocks.GRASS_BLOCK) || s.is(Blocks.CLAY) || s.is(Blocks.SNOW_BLOCK) || s.is(Blocks.DIRT_PATH)
                || s.getBlock().getExplosionResistance() >= UNBREAKABLE;
    }

    /** Load-carrying strength in (0, 1]: stone-like materials 1.0, wood ~0.7, glass ~0.4. */
    public static float strength(BlockState s) {
        float res = s.getBlock().getExplosionResistance();
        return Math.max(0.35f, Math.min(1.0f, 0.35f + res / 8f));
    }
}
