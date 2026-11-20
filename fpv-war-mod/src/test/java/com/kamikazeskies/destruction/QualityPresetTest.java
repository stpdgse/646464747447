package com.kamikazeskies.destruction;

import static org.junit.jupiter.api.Assertions.*;

import org.junit.jupiter.api.Test;

import com.kamikazeskies.destruction.core.QualityPreset;

class QualityPresetTest {
    @Test
    void presetsScaleUpMonotonically() {
        QualityPreset[] p = QualityPreset.values();
        for (int i = 1; i < p.length; i++) {
            assertTrue(p[i].blastRays > p[i - 1].blastRays);
            assertTrue(p[i].blocksPerTick > p[i - 1].blocksPerTick);
            assertTrue(p[i].maxFallingBlocks > p[i - 1].maxFallingBlocks);
            assertTrue(p[i].maxFlyingDebris > p[i - 1].maxFlyingDebris);
            assertTrue(p[i].analysisPadding > p[i - 1].analysisPadding);
            assertTrue(p[i].particleScale > p[i - 1].particleScale);
        }
    }

    @Test
    void flyingDebrisFitsInsideTheFallingBudget() {
        for (QualityPreset p : QualityPreset.values()) assertTrue(p.maxFlyingDebris <= p.maxFallingBlocks);
    }
}
