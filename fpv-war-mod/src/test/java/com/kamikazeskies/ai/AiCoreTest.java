package com.kamikazeskies.ai;

import static org.junit.jupiter.api.Assertions.*;

import org.junit.jupiter.api.Test;

class AiCoreTest {
    @Test void calmWithoutThreat() {
        FearModel f = new FearModel(0.5);
        for (int i = 0; i < 400; i++) f.tick(Double.NaN, 0);
        assertEquals(0, f.fear(), 1e-9);
        assertEquals(Behaviour.IDLE, Brain.decide(f.fear(), 0.5, false));
    }

    @Test void divingDroneBuildsFearFasterThanHoveringOne() {
        FearModel hover = new FearModel(0.5), dive = new FearModel(0.5);
        for (int i = 0; i < 60; i++) { hover.tick(20, 0); dive.tick(20, 20); }
        assertTrue(dive.fear() > hover.fear() * 1.3, hover.fear() + " vs " + dive.fear());
    }

    @Test void closerIsScarier() {
        FearModel far = new FearModel(0.5), near = new FearModel(0.5);
        for (int i = 0; i < 100; i++) { far.tick(40, 5); near.tick(8, 5); }
        assertTrue(near.fear() > far.fear());
    }

    @Test void courageProtects() {
        FearModel cons = new FearModel(Role.CONSCRIPT.courage), vet = new FearModel(Role.VETERAN.courage);
        for (int i = 0; i < 100; i++) { cons.tick(10, 15); vet.tick(10, 15); }
        assertTrue(cons.fear() > vet.fear());
        assertEquals(Behaviour.FLEE, Brain.decide(cons.fear(), Role.CONSCRIPT.courage, true), "a conscript runs from a drone diving at 10 m");
        assertNotEquals(Behaviour.FLEE, Brain.decide(vet.fear(), Role.VETERAN.courage, true), "a veteran holds");
    }

    @Test void blastsAndDeathsSpikeFear() {
        FearModel f = new FearModel(0.5);
        f.blast(1.0);
        assertTrue(f.fear() > 0.5);
        FearModel g = new FearModel(0.5); g.allyDied();
        assertTrue(g.fear() > 0.1 && g.fear() < f.fear());
    }

    @Test void fearDecaysOnceTheThreatIsGone() {
        FearModel f = new FearModel(0.5); f.set(0.9);
        for (int i = 0; i < 600; i++) f.tick(Double.NaN, 0);
        assertTrue(f.fear() < 0.3, "" + f.fear());
    }

    @Test void brainBehaviours() {
        assertEquals(Behaviour.ENGAGE, Brain.decide(0.1, 0.5, true));
        assertEquals(Behaviour.COVER, Brain.decide(0.5, 0.25, true));
        assertEquals(Behaviour.ENGAGE, Brain.decide(0.5, 0.85, true));
        assertEquals(Behaviour.FLEE, Brain.decide(0.95, 0.85, true));
        assertEquals(Behaviour.IDLE, Brain.decide(0.0, 0.5, false));
        assertTrue(Brain.panicThreshold(0.25) < Brain.panicThreshold(0.85));
    }

    @Test void marksmanship() {
        double hover = Marksmanship.hitChance(25, 0, 0.7, 0), fast = Marksmanship.hitChance(25, 20, 0.7, 0);
        assertTrue(hover > 0.2 && fast < 0.03, hover + " / " + fast);
        assertTrue(Marksmanship.hitChance(25, 5, 0.7, 0.9) < Marksmanship.hitChance(25, 5, 0.7, 0));
        assertTrue(Marksmanship.hitChance(60, 5, 0.7, 0) < Marksmanship.hitChance(15, 5, 0.7, 0));
        assertTrue(Marksmanship.cannonChance(30, 20) < 0.01 && Marksmanship.cannonChance(30, 0) > 0.2);
    }

    @Test void rolePickRespectsWeights() {
        int[] n = new int[3];
        for (int i = 0; i < 1000; i++) n[Role.pick(i / 1000.0).ordinal()]++;
        assertTrue(n[0] > 350 && n[1] > 400 && n[2] > 100 && n[2] < 200, java.util.Arrays.toString(n));
    }
}
