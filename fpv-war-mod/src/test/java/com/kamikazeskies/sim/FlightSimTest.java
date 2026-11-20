package com.kamikazeskies.sim;

import static org.junit.jupiter.api.Assertions.*;

import java.util.Random;

import org.junit.jupiter.api.Test;

class FlightSimTest {
    private static final double DT = 1.0 / 80;
    private final DroneParams p = DroneParams.fiveInch();

    private FlightSim newSim(boolean spinUp) {
        FlightSim s = new FlightSim(p);
        s.setState(new Vec3d(0, 200, 0), Vec3d.ZERO, Quat.IDENTITY, Vec3d.ZERO);
        if (spinUp) s.spinUpToHover();
        return s;
    }

    private static void run(FlightSim s, FlightInput in, double seconds) {
        int n = (int) Math.round(seconds / DT);
        for (int i = 0; i < n; i++) s.stepFree(in, DT);
    }

    private FlightInput hover(double roll, double pitch, double yaw, double assist) {
        return new FlightInput(p.hoverStick(), roll, pitch, yaw, true, assist);
    }

    @Test
    void throttleCurveHitsHoverPointAndIsMonotonic() {
        assertEquals(p.hoverThrustFraction(), p.throttleToThrustFraction(p.hoverStick()), 1e-9);
        assertEquals(0, p.throttleToThrustFraction(0), 1e-12);
        assertEquals(1, p.throttleToThrustFraction(1), 1e-12);
        double prev = -1;
        for (int i = 0; i <= 100; i++) {
            double f = p.throttleToThrustFraction(i / 100.0);
            assertTrue(f >= prev);
            prev = f;
        }
    }

    @Test
    void inverseThrottleCurveRoundTrips() {
        for (double stick : new double[] {0.1, 0.25, p.hoverStick(), 0.6, 0.9}) {
            assertEquals(stick, p.stickForThrustFraction(p.throttleToThrustFraction(stick)), 1e-9);
        }
        assertEquals(p.hoverStick(), p.stickForThrustFraction(p.hoverThrustFraction()), 1e-9);
    }

    @Test
    void thrustToWeightIsRealisticFor5Inch() {
        double tw = 4 * p.maxThrustPerMotor() / (p.mass() * DroneParams.GRAVITY);
        assertTrue(tw > 4 && tw < 8, "T/W " + tw);
    }

    @Test
    void hoversWithoutDrift() {
        FlightSim s = newSim(true);
        run(s, hover(0, 0, 0, 1), 5);
        assertEquals(0, s.velocity().y(), 0.05);
        assertEquals(200, s.position().y(), 0.1);
        assertTrue(s.orientation().tiltDeg() < 0.5);
    }

    @Test
    void disarmedDroneFreeFalls() {
        FlightSim s = newSim(false);
        run(s, FlightInput.DISARMED, 1.0);
        assertTrue(s.velocity().y() < -9.0 && s.velocity().y() > -9.9, "vy " + s.velocity().y());
    }

    @Test
    void fallingReachesDragLimitedTerminalVelocity() {
        FlightSim s = newSim(false);
        run(s, FlightInput.DISARMED, 25);
        double expected = Math.sqrt(p.mass() * DroneParams.GRAVITY / p.dragCoefficient());
        assertEquals(expected, -s.velocity().y(), expected * 0.03);
    }

    @Test
    void fullThrottleClimbsHard() {
        FlightSim s = newSim(true);
        run(s, new FlightInput(1, 0, 0, 0, true, 1), 0.5);
        assertTrue(s.velocity().y() > 10, "vy " + s.velocity().y());
    }

    @Test
    void acroRollRateTracksStick() {
        FlightSim s = newSim(true);
        run(s, hover(0.5, 0, 0, 0), 0.35);
        double expected = Math.toRadians(p.rollPitchRates().rate(0.5));
        assertEquals(expected, s.angularVelocity().z(), expected * 0.08);   // +z = roll right
        assertTrue(s.orientation().rollDeg() > 20);                          // right side is down
    }

    @Test
    void acroPitchForwardStickIsNoseDown() {
        FlightSim s = newSim(true);
        run(s, hover(0, 0.5, 0, 0), 0.3);
        assertTrue(s.angularVelocity().x() > 0);
        assertTrue(s.orientation().minecraftPitchDeg() > 10);
    }

    @Test
    void acroRatesReturnToZeroWhenSticksCentered() {
        FlightSim s = newSim(true);
        run(s, hover(1, 1, 1, 0), 0.3);
        run(s, hover(0, 0, 0, 0), 0.5);
        assertEquals(0, s.angularVelocity().length(), 0.1);
    }

    @Test
    void yawRightTurnsNoseRightAndKeepsLevel() {
        FlightSim s = newSim(true);
        run(s, hover(0, 0, 0.3, 1), 0.4);
        double yaw = s.orientation().minecraftYawDeg();
        assertTrue(yaw > 20 && yaw < 90, "yaw " + yaw);      // right turn = increasing Minecraft yaw
        assertTrue(s.orientation().tiltDeg() < 2);
    }

    @Test
    void angleModeLevelsFromTilt() {
        FlightSim s = newSim(true);
        s.setState(s.position(), Vec3d.ZERO, Quat.fromAxisAngle(new Vec3d(0, 0, 1), Math.toRadians(35)), Vec3d.ZERO);
        run(s, hover(0, 0, 0, 1), 2.5);
        assertTrue(s.orientation().tiltDeg() < 1.0, "tilt " + s.orientation().tiltDeg());
    }

    @Test
    void angleModeStickSetsTiltAndMovesForward() {
        FlightSim s = newSim(true);
        run(s, hover(0, 0.5, 0, 1), 2.0);
        assertEquals(0.5 * p.maxAngleDeg(), s.orientation().tiltDeg(), 3.0);
        assertTrue(s.orientation().minecraftPitchDeg() > 15);
        assertTrue(s.velocity().z() > 1.0, "vz " + s.velocity().z());       // facing +Z: forward = +Z
        assertEquals(0, s.velocity().x(), 0.3);
    }

    @Test
    void angleModeRollRightMovesPilotRight() {
        FlightSim s = newSim(true);
        run(s, hover(0.5, 0, 0, 1), 2.0);
        assertTrue(s.orientation().rollDeg() > 15);
        assertTrue(s.velocity().x() < -1.0, "vx " + s.velocity().x());       // facing +Z: right = -X
    }

    @Test
    void angleModeForwardFollowsHeadingAfterYaw() {
        FlightSim s = newSim(true);
        s.setState(s.position(), Vec3d.ZERO, Quat.fromMinecraftYaw(90), Vec3d.ZERO);   // facing west (-X)
        run(s, hover(0, 0.5, 0, 1), 2.0);
        assertTrue(s.velocity().x() < -1.0, "vx " + s.velocity().x());
        assertEquals(0, s.velocity().z(), 0.4);
    }

    @Test
    void assistBlendsBetweenAcroAndAngle() {
        FlightSim acro = newSim(true), angle = newSim(true), mid = newSim(true);
        run(acro, hover(0.5, 0, 0, 0), 1.0);
        run(angle, hover(0.5, 0, 0, 1), 1.0);
        run(mid, hover(0.5, 0, 0, 0.5), 1.0);
        assertTrue(angle.orientation().tiltDeg() < mid.orientation().tiltDeg());
        assertTrue(mid.orientation().tiltDeg() < acro.orientation().tiltDeg());
    }

    @Test
    void motorsNeverLeaveTheirLimitsUnderRandomStickAbuse() {
        FlightSim s = newSim(true);
        Random r = new Random(42);
        for (int i = 0; i < 4000; i++) {
            FlightInput in = new FlightInput(r.nextDouble(), r.nextDouble() * 2 - 1, r.nextDouble() * 2 - 1,
                    r.nextDouble() * 2 - 1, true, r.nextBoolean() ? 1 : 0);
            s.stepFree(in, DT);
            for (int m = 0; m < 4; m++) {
                double t = s.motorThrust(m);
                assertTrue(t >= -1e-9 && t <= p.maxThrustPerMotor() + 1e-9, "motor " + m + " = " + t);
            }
            assertTrue(Double.isFinite(s.position().length()) && Double.isFinite(s.angularVelocity().length()));
        }
    }

    @Test
    void disarmingStopsTheMotors() {
        FlightSim s = newSim(true);
        run(s, FlightInput.DISARMED, 0.5);
        assertTrue(s.averageMotorFraction() < 0.001);
    }

    @Test
    void idleThrottleKeepsMotorsOffEvenWhenArmed() {
        FlightSim s = newSim(false);
        run(s, new FlightInput(0, 0.5, 0.5, 0.5, true, 0), 0.5);
        assertTrue(s.averageMotorFraction() < 0.001);
    }

    @Test
    void commitRemovesBlockedVelocityAndReportsImpact() {
        FlightSim s = newSim(false);
        s.setState(new Vec3d(0, 64, 0), new Vec3d(3, -5, 0), Quat.IDENTITY, Vec3d.ZERO);
        s.commit(new Vec3d(0.1, -0.2, 0), new Vec3d(0.1, 0, 0));
        assertEquals(3, s.velocity().x(), 1e-12);
        assertEquals(0, s.velocity().y(), 1e-12);
        assertEquals(5, s.impactSpeed(), 1e-12);
        assertEquals(0.1, s.position().x(), 1e-12);
        assertEquals(64, s.position().y(), 1e-12);
    }

    @Test
    void simulationIsDeterministic() {
        FlightSim a = newSim(true), b = newSim(true);
        Random ra = new Random(7), rb = new Random(7);
        for (int i = 0; i < 2000; i++) {
            a.stepFree(new FlightInput(ra.nextDouble(), ra.nextDouble() - .5, ra.nextDouble() - .5, ra.nextDouble() - .5, true, 0.3), DT);
            b.stepFree(new FlightInput(rb.nextDouble(), rb.nextDouble() - .5, rb.nextDouble() - .5, rb.nextDouble() - .5, true, 0.3), DT);
        }
        assertEquals(a.position(), b.position());
        assertEquals(a.orientation(), b.orientation());
    }

    @Test
    void flightInputSanitizesNaNAndInfinity() {
        FlightInput in = new FlightInput(Double.NaN, Double.NaN, Double.POSITIVE_INFINITY, Double.NEGATIVE_INFINITY, true, Double.NaN);
        assertEquals(0, in.throttle());
        assertEquals(0, in.roll());
        assertEquals(1, in.pitch());
        assertEquals(-1, in.yaw());
        assertEquals(0, in.assist());
    }
}
