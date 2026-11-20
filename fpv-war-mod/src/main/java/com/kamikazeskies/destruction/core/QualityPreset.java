package com.kamikazeskies.destruction.core;

/**
 * Performance budgets for destruction. The reference rig (RTX 3060 / Ryzen 7 5700X) targets HIGH; weaker machines or
 * big servers pick MEDIUM/LOW. All numbers are per server.
 *
 * @param blastRays         rays cast per blast
 * @param blocksPerTick     blocks converted per tick when applying collapses/blasts (spreads the work over ticks)
 * @param maxFallingBlocks  concurrent falling-block entities; excess collapsing blocks just crumble into particles
 * @param maxFlyingDebris   debris entities thrown outward by a blast (subset of falling blocks)
 * @param analysisPadding   blocks around the blast included in the stability analysis
 * @param particleScale     multiplier for dust and smoke
 */
public enum QualityPreset {
    LOW(700, 32, 96, 24, 10, 0.4),
    MEDIUM(1100, 80, 256, 64, 14, 0.7),
    HIGH(1800, 160, 512, 128, 18, 1.0);

    public final int blastRays, blocksPerTick, maxFallingBlocks, maxFlyingDebris, analysisPadding;
    public final double particleScale;

    QualityPreset(int blastRays, int blocksPerTick, int maxFallingBlocks, int maxFlyingDebris, int analysisPadding, double particleScale) {
        this.blastRays = blastRays;
        this.blocksPerTick = blocksPerTick;
        this.maxFallingBlocks = maxFallingBlocks;
        this.maxFlyingDebris = maxFlyingDebris;
        this.analysisPadding = analysisPadding;
        this.particleScale = particleScale;
    }
}
