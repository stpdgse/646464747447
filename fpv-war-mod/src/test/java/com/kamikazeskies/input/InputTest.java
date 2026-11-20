package com.kamikazeskies.input;

import static org.junit.jupiter.api.Assertions.*;

import org.junit.jupiter.api.Test;

import com.kamikazeskies.input.GamepadSticks.Axes;
import com.kamikazeskies.input.GamepadSticks.ThrottleMode;
import com.kamikazeskies.input.KeyboardSticks.Keys;
import com.kamikazeskies.sim.FlightInput;

class InputTest {
    private static final double DT = 0.05;

    private static Keys keys(boolean fwd, boolean back, boolean left, boolean right, boolean yl, boolean yr, boolean up, boolean down) {
        return new Keys(fwd, back, left, right, yl, yr, up, down);
    }

    // ---- StickMath --------------------------------------------------------------------------------
    @Test
    void deadzoneZeroesSmallValuesAndRescalesToFullRange() {
        assertEquals(0, StickMath.deadzone(0.05, 0.1), 1e-12);
        assertEquals(0, StickMath.deadzone(-0.1, 0.1), 1e-12);
        assertEquals(1, StickMath.deadzone(1, 0.1), 1e-12);
        assertEquals(-1, StickMath.deadzone(-1, 0.1), 1e-12);
        assertEquals(0.5, StickMath.deadzone(0.55, 0.1), 1e-12);          // (0.55-0.1)/0.9
        assertEquals(-0.5, StickMath.deadzone(-0.55, 0.1), 1e-12);
    }

    // ---- switches ---------------------------------------------------------------------------------
    @Test
    void armingIsRefusedWithHighThrottleAndAllowedWithLow() {
        PilotSwitches s = new PilotSwitches();
        assertFalse(s.toggleArm(0.5));
        assertTrue(s.armRefused());
        assertTrue(s.toggleArm(0.0));
        assertFalse(s.armRefused());
        assertFalse(s.toggleArm(0.9));          // disarming always works
    }

    @Test
    void assistCyclesThroughAllModes() {
        PilotSwitches s = new PilotSwitches();
        assertEquals(AssistMode.ANGLE, s.assist());
        s.cycleAssist(); assertEquals(AssistMode.MIXED, s.assist());
        s.cycleAssist(); assertEquals(AssistMode.ACRO, s.assist());
        s.cycleAssist(); assertEquals(AssistMode.ANGLE, s.assist());
    }

    // ---- keyboard ---------------------------------------------------------------------------------
    @Test
    void keyboardStickRampsUpAndRecentres() {
        PilotSwitches sw = new PilotSwitches();
        KeyboardSticks kb = new KeyboardSticks(sw);
        kb.setKeys(keys(true, false, false, false, false, false, false, false));
        double first = kb.sample(DT).pitch();
        assertTrue(first > 0 && first < 1, "ramps, not snaps: " + first);
        for (int i = 0; i < 10; i++) kb.sample(DT);
        assertEquals(1.0, kb.sample(DT).pitch(), 1e-12);
        kb.setKeys(Keys.NONE);
        for (int i = 0; i < 10; i++) kb.sample(DT);
        assertEquals(0.0, kb.sample(DT).pitch(), 1e-12);
    }

    @Test
    void keyboardDirectionsMatchTheSimConventions() {
        KeyboardSticks kb = new KeyboardSticks(new PilotSwitches());
        kb.setKeys(keys(false, false, false, true, false, true, false, false));   // roll right + yaw right
        FlightInput in = null;
        for (int i = 0; i < 10; i++) in = kb.sample(DT);
        assertTrue(in.roll() > 0.99 && in.yaw() > 0.99);                           // right = +
        kb.setKeys(keys(true, false, true, false, true, false, false, false));    // pitch forward + roll left + yaw left
        for (int i = 0; i < 40; i++) in = kb.sample(DT);
        assertTrue(in.pitch() > 0.99);                                             // forward = nose down = +
        assertTrue(in.roll() < -0.99 && in.yaw() < -0.99);
    }

    @Test
    void oppositeKeysCancelAndReversalIsFast() {
        KeyboardSticks kb = new KeyboardSticks(new PilotSwitches());
        kb.setKeys(keys(true, true, false, false, false, false, false, false));
        for (int i = 0; i < 5; i++) assertEquals(0, kb.sample(DT).pitch(), 1e-12);
        kb.setKeys(keys(true, false, false, false, false, false, false, false));
        for (int i = 0; i < 10; i++) kb.sample(DT);
        kb.setKeys(keys(false, true, false, false, false, false, false, false));
        double after = 0;
        for (int i = 0; i < 3; i++) after = kb.sample(DT).pitch();
        assertTrue(after < 0.5, "reverses within a few samples: " + after);
    }

    @Test
    void keyboardThrottleStaysWhereYouLeaveItWhenArmed() {
        PilotSwitches sw = new PilotSwitches();
        sw.toggleArm(0);
        KeyboardSticks kb = new KeyboardSticks(sw);
        kb.setKeys(keys(false, false, false, false, false, false, true, false));
        for (int i = 0; i < 8; i++) kb.sample(DT);                                 // 0.4 s * 0.7 = 0.28
        double held = kb.throttle();
        assertEquals(0.28, held, 0.02);
        kb.setKeys(Keys.NONE);
        for (int i = 0; i < 40; i++) kb.sample(DT);
        assertEquals(held, kb.throttle(), 1e-12);                                  // no spring
        kb.setKeys(keys(false, false, false, false, false, false, false, true));
        for (int i = 0; i < 40; i++) kb.sample(DT);
        assertEquals(0, kb.throttle(), 1e-12);
    }

    @Test
    void keyboardThrottleIsForcedLowWhenDisarmed() {
        PilotSwitches sw = new PilotSwitches();           // disarmed
        KeyboardSticks kb = new KeyboardSticks(sw);
        kb.setKeys(keys(false, false, false, false, false, false, true, false));
        for (int i = 0; i < 6; i++) kb.sample(DT);
        kb.setKeys(Keys.NONE);
        assertEquals(0, kb.sample(DT).throttle(), 1e-12);
        assertFalse(kb.sample(DT).armed());
    }

    @Test
    void keyboardReportsActivityOnlyWhileKeysAreDown() {
        KeyboardSticks kb = new KeyboardSticks(new PilotSwitches());
        assertFalse(kb.isActive());
        kb.setKeys(keys(true, false, false, false, false, false, false, false));
        assertTrue(kb.isActive());
    }

    // ---- gamepad ----------------------------------------------------------------------------------
    @Test
    void gamepadMode2LayoutAndDirections() {
        PilotSwitches sw = new PilotSwitches();
        GamepadSticks gp = new GamepadSticks(sw);
        // left stick: x=+1 (yaw right), y=-1 (up => full throttle); right stick: x=-1 (roll left), y=-1 (up => pitch forward)
        gp.setAxes(new Axes(1, -1, -1, -1));
        FlightInput in = gp.sample(DT);
        assertEquals(1.0, in.throttle(), 1e-12);
        assertEquals(1.0, in.yaw(), 1e-12);
        assertEquals(-1.0, in.roll(), 1e-12);
        assertEquals(1.0, in.pitch(), 1e-12);
    }

    @Test
    void gamepadDirectThrottleIdlesAtHalfAndZeroAtBottom() {
        GamepadSticks gp = new GamepadSticks(new PilotSwitches());
        gp.setAxes(Axes.CENTERED);
        assertEquals(0.5, gp.sample(DT).throttle(), 1e-12);
        gp.setAxes(new Axes(0, 1, 0, 0));
        assertEquals(0.0, gp.sample(DT).throttle(), 1e-12);
    }

    @Test
    void gamepadDeadzoneIgnoresStickDrift() {
        GamepadSticks gp = new GamepadSticks(new PilotSwitches());
        gp.setAxes(new Axes(0.07, 0, -0.08, 0.09));
        FlightInput in = gp.sample(DT);
        assertEquals(0, in.roll(), 1e-12);
        assertEquals(0, in.pitch(), 1e-12);
        assertEquals(0, in.yaw(), 1e-12);
        assertFalse(gp.isActive());
    }

    @Test
    void gamepadIncrementalThrottleHoldsPosition() {
        PilotSwitches sw = new PilotSwitches();
        sw.toggleArm(0);
        GamepadSticks gp = new GamepadSticks(sw, ThrottleMode.INCREMENTAL, GamepadSticks.DEFAULT_DEADZONE);
        gp.setAxes(new Axes(0, -1, 0, 0));
        for (int i = 0; i < 10; i++) gp.sample(DT);                                // 0.5 s * 0.9
        double held = gp.throttle();
        assertEquals(0.45, held, 0.01);
        gp.setAxes(Axes.CENTERED);
        for (int i = 0; i < 20; i++) gp.sample(DT);
        assertEquals(held, gp.throttle(), 1e-12);
    }

    @Test
    void gamepadCarriesSwitchState() {
        PilotSwitches sw = new PilotSwitches();
        GamepadSticks gp = new GamepadSticks(sw);
        assertFalse(gp.sample(DT).armed());
        sw.toggleArm(0);
        sw.setAssist(AssistMode.ACRO);
        FlightInput in = gp.sample(DT);
        assertTrue(in.armed());
        assertEquals(0.0, in.assist(), 1e-12);
    }
}
