package com.kamikazeskies.sim;

import static org.junit.jupiter.api.Assertions.*;

import org.junit.jupiter.api.Test;

class BatteryLinkTest {
    // ---- battery ----------------------------------------------------------------------------------
    @Test
    void freshPackIsAboutFourPointTwoVoltsPerCell() {
        Battery b = new Battery(Battery.Spec.fourS1300());
        assertEquals(4.2, b.restingCellVoltage(), 0.001);
        assertEquals(16.8, b.voltage(0), 0.01);
        assertEquals(1.0, b.soc(), 1e-9);
    }

    @Test
    void voltageCurveIsMonotonicWithStateOfCharge() {
        Battery b = new Battery(Battery.Spec.fourS1300());
        double prev = Double.MAX_VALUE;
        for (int i = 100; i >= 0; i--) {
            b.setSoc(i / 100.0);
            double v = b.restingCellVoltage();
            assertTrue(v <= prev + 1e-12, "not monotonic at " + i + "%");
            prev = v;
        }
        assertEquals(3.0, b.restingCellVoltage(), 0.001);
    }

    @Test
    void voltageSagsUnderLoadAndMoreSoWhenWeak() {
        Battery full = new Battery(Battery.Spec.fourS1300());
        Battery weak = new Battery(Battery.Spec.fourS1300());
        weak.setSoc(0.15);
        double fullSag = full.voltage(0) - full.voltage(100);
        double weakSag = weak.voltage(0) - weak.voltage(100);
        assertTrue(fullSag > 1.0, "a 100 A punch sags more than a volt on 4S: " + fullSag);
        assertEquals(fullSag, weakSag, 1e-9);                       // I*R does not depend on charge...
        assertTrue(weak.cellVoltage(100) < full.cellVoltage(100)); // ...but the result is far lower
        assertTrue(weak.powerScale(100) < full.powerScale(100));
    }

    @Test
    void drainFollowsCurrentTimesTime() {
        Battery b = new Battery(Battery.Spec.fourS1300());
        b.drain(36.0, 100.0);          // 36 A for 100 s = 1000 mAh
        assertEquals(1000, b.usedMah(), 1e-6);
        assertEquals(1 - 1000.0 / 1300, b.soc(), 1e-9);
        b.drain(1000, 1000);
        assertTrue(b.empty());
        assertEquals(0, b.soc(), 1e-12);
    }

    @Test
    void hoverFlightTimeIsRealisticForA5InchOn1300mAh() {
        Battery b = new Battery(Battery.Spec.fourS1300());
        double hoverCurrent = Battery.currentFor(DroneParams.fiveInch().hoverThrustFraction());
        double seconds = 0;
        while (!b.empty() && seconds < 3600) { b.drain(hoverCurrent, 1.0); seconds++; }
        double minutes = seconds / 60;
        assertTrue(minutes > 4 && minutes < 12, "hover time " + minutes + " min at " + hoverCurrent + " A");
    }

    @Test
    void bigPackFliesLonger() {
        double cur = Battery.currentFor(0.3);
        Battery small = new Battery(Battery.Spec.fourS1300()), big = new Battery(Battery.Spec.fourS2200());
        small.drain(cur, 120); big.drain(cur, 120);
        assertTrue(big.soc() > small.soc());
    }

    @Test
    void currentRisesWithThrustAndIsBounded() {
        assertEquals(Battery.IDLE_CURRENT_A, Battery.currentFor(0), 1e-9);
        assertEquals(Battery.MAX_CURRENT_A, Battery.currentFor(1), 1e-9);
        assertTrue(Battery.currentFor(0.5) > Battery.currentFor(0.2));
        assertEquals(Battery.currentFor(1), Battery.currentFor(5), 1e-9);
    }

    @Test
    void powerScaleIsFullWhenHealthyAndFallsToACutoffFloor() {
        Battery b = new Battery(Battery.Spec.fourS1300());
        assertEquals(1.0, b.powerScale(10), 1e-9);
        double prev = 1.0;
        for (int i = 100; i >= 0; i -= 5) {
            b.setSoc(i / 100.0);
            double s = b.powerScale(60);
            assertTrue(s <= prev + 1e-9, "power scale must not increase as the pack empties");
            prev = s;
        }
        assertEquals(0.1, b.powerScale(60), 1e-9);
    }

    @Test
    void warningsTriggerAtLowVoltage() {
        Battery b = new Battery(Battery.Spec.fourS1300());
        assertFalse(b.low());
        b.setSoc(0.08); assertTrue(b.low()); assertFalse(b.critical());
        b.setSoc(0.03); assertTrue(b.critical());
    }

    // ---- radio link -------------------------------------------------------------------------------
    @Test
    void distanceFactorHasAPlateauThenFallsToZeroAtRange() {
        assertEquals(1.0, RadioLink.distanceFactor(0, 500), 1e-12);
        assertEquals(1.0, RadioLink.distanceFactor(150, 500), 1e-12);
        double prev = 1.0;
        for (int d = 150; d <= 500; d += 10) {
            double f = RadioLink.distanceFactor(d, 500);
            assertTrue(f <= prev + 1e-12);
            prev = f;
        }
        assertEquals(0.0, RadioLink.distanceFactor(500, 500), 1e-12);
        assertEquals(0.0, RadioLink.distanceFactor(900, 500), 1e-12);
    }

    @Test
    void blocksInTheWayKillTheSignal() {
        double clear = RadioLink.rssi(100, 500, 0, 0);
        double wall = RadioLink.rssi(100, 500, 1, 0);
        double thick = RadioLink.rssi(100, 500, 8, 0);
        assertEquals(1.0, clear, 1e-12);
        assertTrue(wall < clear && thick < wall);
        assertTrue(wall > 0.7 && thick < 0.15, "one block " + wall + ", eight blocks " + thick);
    }

    @Test
    void jammersFadeWithDistanceAndCompound() {
        assertEquals(1.0, RadioLink.jammingAt(5, 60, 1.0), 1e-12);
        assertEquals(0.0, RadioLink.jammingAt(60, 60, 1.0), 1e-12);
        assertTrue(RadioLink.jammingAt(40, 60, 1.0) < RadioLink.jammingAt(25, 60, 1.0));
        assertEquals(0.5, RadioLink.jammingAt(5, 60, 0.5), 1e-12);
        assertEquals(0.75, RadioLink.combine(0.5, 0.5), 1e-12);
        assertEquals(RadioLink.rssi(50, 500, 0, 0) * 0.0, RadioLink.rssi(50, 500, 0, 1.0), 1e-12);
    }

    @Test
    void signalLevelsAreOrderedAndSeparateCleanFromLost() {
        assertEquals(SignalLevel.CLEAN, SignalLevel.of(1.0));
        assertEquals(SignalLevel.NOISY, SignalLevel.of(0.35));
        assertEquals(SignalLevel.HEAVY_BREAKUP, SignalLevel.of(0.10));
        assertEquals(SignalLevel.LOST, SignalLevel.of(0.02));
        SignalLevel prev = SignalLevel.CLEAN;
        for (int i = 100; i >= 0; i--) {
            SignalLevel l = SignalLevel.of(i / 100.0);
            assertTrue(l.ordinal() >= prev.ordinal(), "levels never improve as the signal falls");
            prev = l;
        }
    }

    @Test
    void hysteresisStopsFlickerAroundAThreshold() {
        SignalLevel l = SignalLevel.NOISY;
        int changes = 0;
        for (double r : new double[] {0.29, 0.305, 0.29, 0.31, 0.295, 0.31}) {
            SignalLevel n = SignalLevel.withHysteresis(l, r, 0.03);
            if (n != l) changes++;
            l = n;
        }
        assertTrue(changes <= 1, "flickered " + changes + " times");
    }

    @Test
    void monitorFailsSafeAfterSustainedLossAndRecoversWithHysteresis() {
        LinkMonitor m = new LinkMonitor();
        for (int i = 0; i < 5; i++) m.update(0.0);
        assertFalse(m.controlLost(), "a short dropout is not a failsafe");
        for (int i = 0; i < 20; i++) m.update(0.0);
        assertTrue(m.controlLost());
        assertEquals(SignalLevel.LOST, m.level());
        for (int i = 0; i < 3; i++) m.update(1.0);
        assertTrue(m.controlLost(), "recovery needs a few good ticks");
        for (int i = 0; i < 30; i++) m.update(1.0);
        assertFalse(m.controlLost());
        assertEquals(SignalLevel.CLEAN, m.level());
    }

    @Test
    void packetDropsOnlyWhenTheSignalIsWeakAndGrowAsItFalls() {
        LinkMonitor strong = new LinkMonitor();
        strong.update(1.0);
        assertEquals(0.0, strong.dropProbability(), 1e-12);
        LinkMonitor weak = new LinkMonitor(), weaker = new LinkMonitor();
        for (int i = 0; i < 40; i++) { weak.update(0.2); weaker.update(0.08); }
        assertTrue(weak.dropProbability() > 0 && weaker.dropProbability() > weak.dropProbability());
        assertTrue(weaker.dropProbability() <= 0.6);
        assertTrue(weaker.dropPacket(0.0) && !weaker.dropPacket(0.99));
    }
}
