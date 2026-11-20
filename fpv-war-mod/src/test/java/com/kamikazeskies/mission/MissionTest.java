package com.kamikazeskies.mission;

import static org.junit.jupiter.api.Assertions.*;

import java.io.InputStream;
import java.nio.charset.StandardCharsets;

import org.junit.jupiter.api.Test;

class MissionTest {
    private static MissionDef outpost() throws Exception {
        try (InputStream in = MissionTest.class.getResourceAsStream("/data/kamikaze_skies/missions/outpost_alpha.json")) {
            return MissionDef.parse(new String(in.readAllBytes(), StandardCharsets.UTF_8));
        }
    }

    @Test void parsesTheShippedMission() throws Exception {
        MissionDef m = outpost();
        assertEquals("outpost_alpha", m.id());
        assertEquals(2, m.objectives().size());
        assertEquals(4, m.spawns().stream().filter(s -> s.entity().equals("soldier")).mapToInt(MissionDef.Spawn::count).sum(), "spawned soldiers match the objective");
        assertEquals(1, m.spawns().stream().filter(s -> s.entity().equals("tank")).mapToInt(MissionDef.Spawn::count).sum());
        assertFalse(m.rewards().isEmpty());
    }

    @Test void completesWhenEveryObjectiveIsMet() throws Exception {
        MissionState s = new MissionState(outpost());
        assertTrue(s.onKill("tank"));
        assertEquals(MissionState.Status.ACTIVE, s.status());
        for (int i = 0; i < 3; i++) s.onKill("soldier");
        assertEquals(MissionState.Status.ACTIVE, s.status());
        s.onKill("soldier");
        assertEquals(MissionState.Status.COMPLETE, s.status());
        assertFalse(s.onKill("soldier"), "kills after completion do not count");
    }

    @Test void extraKillsDoNotOverfillOrCountForOtherObjectives() throws Exception {
        MissionState s = new MissionState(outpost());
        for (int i = 0; i < 10; i++) s.onKill("soldier");
        assertEquals(4, s.progress(1));
        assertEquals(0, s.progress(0));
        assertFalse(s.onKill("jammer"));
    }

    @Test void failsWhenTheOperatorDiesOrTimeRunsOut() throws Exception {
        MissionState a = new MissionState(outpost());
        a.onOperatorDied();
        assertEquals(MissionState.Status.FAILED, a.status());
        assertFalse(a.onKill("tank"));
        MissionState b = new MissionState(outpost());
        for (int i = 0; i < 600 * 20 + 2; i++) b.tick();
        assertEquals(MissionState.Status.FAILED, b.status());
        assertEquals("Out of time", b.failReason());
    }

    @Test void bonusDependsOnTime() throws Exception {
        MissionState fast = new MissionState(outpost());
        for (int i = 0; i < 100; i++) fast.tick();
        fast.onKill("tank"); for (int i = 0; i < 4; i++) fast.onKill("soldier");
        assertEquals(1, fast.earnedBonus().size());
        MissionState slow = new MissionState(outpost());
        for (int i = 0; i < 300 * 20; i++) slow.tick();
        slow.onKill("tank"); for (int i = 0; i < 4; i++) slow.onKill("soldier");
        assertEquals(MissionState.Status.COMPLETE, slow.status());
        assertTrue(slow.earnedBonus().isEmpty());
    }
}
