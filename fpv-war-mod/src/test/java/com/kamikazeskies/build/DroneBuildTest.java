package com.kamikazeskies.build;

import static org.junit.jupiter.api.Assertions.*;

import org.junit.jupiter.api.Test;

import com.kamikazeskies.drone.Warhead;
import com.kamikazeskies.sim.DroneParams;
import com.kamikazeskies.sim.FlightInput;
import com.kamikazeskies.sim.FlightSim;
import com.kamikazeskies.sim.Quat;
import com.kamikazeskies.sim.Vec3d;

class DroneBuildTest {
    private static final double DT = 1.0 / 80;

    /** Every one of the 3^7 x 3 x 2 combinations. */
    private interface Visitor { void visit(DroneBuild b); }

    private static void forEveryBuild(Visitor v) {
        for (int f = 1; f <= 3; f++) for (int m = 1; m <= 3; m++) for (int p = 1; p <= 3; p++) for (int b = 1; b <= 3; b++)
            for (int c = 1; c <= 3; c++) for (int cam = 1; cam <= 3; cam++) for (int a = 1; a <= 3; a++)
                for (Warhead w : Warhead.values()) for (boolean bb : new boolean[] {false, true})
                    v.visit(new DroneBuild(f, m, p, b, c, cam, a, w, bb));
    }

    @Test
    void standardBuildIsABalancedFiveInch() {
        DroneBuild.Stats s = DroneBuild.STANDARD.stats();
        assertEquals(0.62, s.massKg(), 0.05);
        assertTrue(s.thrustToWeight() > 5 && s.thrustToWeight() < 7, "T/W " + s.thrustToWeight());
        assertTrue(s.hoverMinutes() > 4 && s.hoverMinutes() < 12, "hover " + s.hoverMinutes() + " min");
        assertEquals(1.0, s.rangeMultiplier());
    }

    @Test
    void everyCombinationIsFlyableAndSensible() {
        int[] count = {0};
        forEveryBuild(b -> {
            count[0]++;
            DroneBuild.Stats s = b.stats();
            assertTrue(s.thrustToWeight() > 2.5 && s.thrustToWeight() < 9.5, b + " T/W " + s.thrustToWeight());
            assertTrue(s.massKg() > 0.45 && s.massKg() < 1.0, b + " mass " + s.massKg());
            assertTrue(s.hoverMinutes() > 2.0 && s.hoverMinutes() < 30, b + " hover " + s.hoverMinutes());
            DroneParams p = b.toParams();
            assertTrue(p.hoverThrustFraction() > 0.05 && p.hoverThrustFraction() < 0.5, b + " hover fraction " + p.hoverThrustFraction());
            assertEquals(b, DroneBuild.decode(b.encode()), "encoding round trip");
            assertTrue(b.encode() >= 0 && b.encode() < (1 << 17));
        });
        assertEquals(3 * 3 * 3 * 3 * 3 * 3 * 3 * 3 * 2, count[0]);
    }

    @Test
    void betterPartsDoWhatTheirNamesSay() {
        DroneBuild b = DroneBuild.STANDARD;
        assertTrue(b.withTier(PartKind.MOTOR, 3).stats().thrustToWeight() > b.withTier(PartKind.MOTOR, 1).stats().thrustToWeight());
        assertTrue(b.withTier(PartKind.BATTERY, 3).stats().hoverMinutes() > b.withTier(PartKind.BATTERY, 1).stats().hoverMinutes());
        assertTrue(b.withTier(PartKind.BATTERY, 3).massKg() > b.withTier(PartKind.BATTERY, 1).massKg(), "a bigger pack is heavier");
        assertTrue(b.withTier(PartKind.FRAME, 3).crashSpeed() > b.withTier(PartKind.FRAME, 1).crashSpeed());
        assertTrue(b.withTier(PartKind.FRAME, 3).massKg() < b.withTier(PartKind.FRAME, 1).massKg(), "better frames are lighter");
        assertTrue(b.withTier(PartKind.ANTENNA, 3).rangeMultiplier() > b.withTier(PartKind.ANTENNA, 1).rangeMultiplier());
        assertTrue(b.withTier(PartKind.CAMERA, 3).fovDegrees() > b.withTier(PartKind.CAMERA, 1).fovDegrees());
        assertTrue(b.withTier(PartKind.CAMERA, 3).videoSignal(0.2) > b.withTier(PartKind.CAMERA, 1).videoSignal(0.2));
        assertTrue(b.withTier(PartKind.PROP, 3).toParams().motorTimeConstant() < b.withTier(PartKind.PROP, 1).toParams().motorTimeConstant());
        assertTrue(b.withTier(PartKind.PROP, 3).maxCurrentA() < b.withTier(PartKind.PROP, 1).maxCurrentA(), "efficient props draw less");
        assertTrue(b.withTier(PartKind.FLIGHT_CONTROLLER, 3).toParams().ratePGain() > b.withTier(PartKind.FLIGHT_CONTROLLER, 1).toParams().ratePGain());
    }

    @Test
    void warheadsAndBlackboxesCostWeightAndFlightPerformance() {
        DroneBuild plain = DroneBuild.STANDARD;
        assertTrue(plain.withWarhead(Warhead.IMPACT).massKg() > plain.massKg());
        assertTrue(plain.withWarhead(Warhead.SHAPED).massKg() > plain.withWarhead(Warhead.IMPACT).massKg());
        assertTrue(plain.withWarhead(Warhead.SHAPED).stats().thrustToWeight() < plain.stats().thrustToWeight());
        assertTrue(plain.withBlackbox(true).massKg() > plain.massKg());
    }

    @Test
    void videoSignalNeverInventsALostLinkAndNeverExceedsOne() {
        for (int t = 1; t <= 3; t++) {
            DroneBuild b = DroneBuild.STANDARD.withTier(PartKind.CAMERA, t);
            assertEquals(0.0, b.videoSignal(0.0), 1e-12, "a dead link stays dead");
            assertTrue(b.videoSignal(0.99) <= 1.0);
            if (t == 3) assertEquals(1.0, b.videoSignal(0.99), 1e-12);
            assertTrue(b.videoSignal(0.3) >= 0.3);
        }
    }

    @Test
    void blackboxNeedsAtLeastATierTwoController() {
        assertNotNull(DroneBuild.STANDARD.withTier(PartKind.FLIGHT_CONTROLLER, 1).withBlackbox(true).validate());
        assertNull(DroneBuild.STANDARD.withTier(PartKind.FLIGHT_CONTROLLER, 2).withBlackbox(true).validate());
        assertNull(DroneBuild.STANDARD.validate());
    }

    @Test
    void tiersAreClampedAndNonTieredKindsRejectTierEdits() {
        DroneBuild b = new DroneBuild(0, 9, -3, 2, 2, 2, 2, null, false);
        assertEquals(1, b.frame()); assertEquals(3, b.motor()); assertEquals(1, b.prop());
        assertEquals(Warhead.NONE, b.warhead());
        assertThrows(IllegalArgumentException.class, () -> b.withTier(PartKind.BLACKBOX, 2));
        assertEquals(DroneBuild.STANDARD, DroneBuild.decode(DroneBuild.STANDARD.encode()));
        assertEquals(DroneBuild.STANDARD, DroneBuild.decode(0x1FFFF & ~0).withTier(PartKind.FRAME, 2).withTier(PartKind.MOTOR, 2).withTier(PartKind.PROP, 2)
                .withTier(PartKind.BATTERY, 2).withTier(PartKind.FLIGHT_CONTROLLER, 2).withTier(PartKind.CAMERA, 2).withTier(PartKind.ANTENNA, 2)
                .withWarhead(Warhead.NONE).withBlackbox(false));
    }

    // ---- the build really drives the flight model ----------------------------------------------------
    private static double climbAfter(DroneBuild b, double seconds, double stick) {
        FlightSim s = new FlightSim(b.toParams());
        s.setState(new Vec3d(0, 200, 0), Vec3d.ZERO, Quat.IDENTITY, Vec3d.ZERO);
        s.spinUpToHover();
        for (int i = 0; i < (int) (seconds / DT); i++) s.stepFree(new FlightInput(stick, 0, 0, 0, true, 1), DT);
        return s.position().y() - 200;
    }

    @Test
    void everyBuildHoversAtTheSameThrottleStickPosition() {
        for (DroneBuild b : new DroneBuild[] {DroneBuild.STANDARD,
                new DroneBuild(1, 1, 1, 3, 1, 1, 1, Warhead.SHAPED, false),
                new DroneBuild(3, 3, 3, 1, 3, 3, 3, Warhead.NONE, true)}) {
            double y = climbAfter(b, 4.0, b.toParams().hoverStick());
            assertEquals(0, y, 0.2, "hover drift for " + b);
        }
    }

    @Test
    void lighterStrongerBuildsClimbHarderThanHeavyWeakOnes() {
        DroneBuild racer = new DroneBuild(3, 3, 3, 1, 3, 3, 1, Warhead.NONE, false);
        DroneBuild hauler = new DroneBuild(1, 1, 1, 3, 1, 1, 1, Warhead.SHAPED, true);
        assertTrue(climbAfter(racer, 1.0, 1.0) > climbAfter(hauler, 1.0, 1.0) + 5);
    }

    // ---- flight recorder ----------------------------------------------------------------------------
    @Test
    void recorderKeepsTheLastThirtySecondsAndTheMaxima() {
        FlightRecorder r = new FlightRecorder();
        for (int t = 0; t < 1000; t++) r.record(t, t * 0.1, 64 + Math.min(t, 300) * 0.05, 0, Math.min(t, 400) * 0.1, 16.0 - t * 0.003, 1.0 - t * 0.0005, 64);
        assertEquals(FlightRecorder.CAPACITY, r.size());
        FlightRecorder.Sample[] s = r.samples();
        assertEquals(FlightRecorder.CAPACITY, s.length);
        for (int i = 1; i < s.length; i++) assertTrue(s[i].tick() > s[i - 1].tick(), "samples are oldest first");
        assertEquals(995, s[s.length - 1].tick());
        FlightRecorder.Summary sum = r.summary();
        assertEquals(999 / 20.0, sum.flightSeconds(), 0.06);
        assertEquals(40.0 * 3.6, sum.maxSpeedKmh(), 1e-6);
        assertEquals(15.0, sum.maxAltitude(), 1e-6);
        assertTrue(sum.minVolts() < 13.1 && sum.minRssi() < 0.52);
    }

    @Test
    void recorderSamplesRoundTripThroughTheIntegerEncoding() {
        FlightRecorder r = new FlightRecorder();
        for (int t = 0; t < 60; t++) r.record(t, 100.25 + t, 70.5, -33.75, 12.34, 15.67, 0.789, 64);
        FlightRecorder.Sample[] before = r.samples();
        FlightRecorder.Sample[] after = FlightRecorder.decodeSamples(r.encodeSamples());
        assertEquals(before.length, after.length);
        for (int i = 0; i < before.length; i++) {
            assertEquals(before[i].tick(), after[i].tick());
            assertEquals(before[i].x(), after[i].x(), 0.06);
            assertEquals(before[i].speed(), after[i].speed(), 0.006);
            assertEquals(before[i].volts(), after[i].volts(), 0.006);
            assertEquals(before[i].rssi(), after[i].rssi(), 0.0006);
        }
    }

    @Test
    void emptyRecorderHasAnEmptySummary() {
        FlightRecorder r = new FlightRecorder();
        assertEquals(0, r.size());
        assertEquals(0, r.summary().flightSeconds());
        assertEquals(0, r.samples().length);
    }
}
