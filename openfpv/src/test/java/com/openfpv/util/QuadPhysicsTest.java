package com.openfpv.util;

import static org.junit.Assert.assertArrayEquals;
import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import org.junit.Test;

public class QuadPhysicsTest {
    private static final double EPS = 1e-9;

    @Test
    public void levelDroneHasNoHorizontalAcceleration() {
        double[] a = QuadPhysics.acceleration(123, 0, 0, 0, 0.12, 0.05);
        assertArrayEquals(new double[]{0, 0, 0}, a, EPS);
    }

    @Test
    public void leaningForwardAcceleratesAlongTheHeading() {
        double[] south = QuadPhysics.acceleration(0, 30, 0, 0, 0.12, 0.05);
        assertEquals(0.0, south[0], EPS);
        assertEquals(0.12 * Math.sin(Math.toRadians(30)), south[2], EPS);
        double[] west = QuadPhysics.acceleration(90, 30, 0, 0, 0.12, 0.05);
        assertTrue(west[0] < 0);
        assertEquals(0.0, west[2], 1e-9);
        double[] north = QuadPhysics.acceleration(180, 30, 0, 0, 0.12, 0.05);
        assertTrue(north[2] < 0);
    }

    @Test
    public void rightBankAcceleratesToTheRight() {
        // Facing south, right is west (-X).
        double[] a = QuadPhysics.acceleration(0, 0, 30, 0, 0.12, 0.05);
        assertTrue(a[0] < 0);
        assertEquals(0.0, a[2], EPS);
        // Facing west, right is north (-Z).
        double[] b = QuadPhysics.acceleration(90, 0, 30, 0, 0.12, 0.05);
        assertTrue(b[2] < 0);
    }

    @Test
    public void tiltBackBrakes() {
        double[] fwd = QuadPhysics.acceleration(40, 25, 0, 0, 0.12, 0.05);
        double[] back = QuadPhysics.acceleration(40, -25, 0, 0, 0.12, 0.05);
        assertArrayEquals(new double[]{-fwd[0], 0, -fwd[2]}, new double[]{back[0], 0, back[2]}, EPS);
    }

    @Test
    public void throttleIsVerticalAndClamped() {
        assertEquals(0.05, QuadPhysics.acceleration(0, 0, 0, 1, 0.12, 0.05)[1], EPS);
        assertEquals(-0.05, QuadPhysics.acceleration(0, 0, 0, -1, 0.12, 0.05)[1], EPS);
        assertEquals(0.05, QuadPhysics.acceleration(0, 0, 0, 9, 0.12, 0.05)[1], EPS);
    }

    @Test
    public void accelerationGrowsWithTiltButNotLinearlyPastNinety() {
        double a10 = QuadPhysics.acceleration(0, 10, 0, 0, 0.12, 0.05)[2];
        double a35 = QuadPhysics.acceleration(0, 35, 0, 0, 0.12, 0.05)[2];
        assertTrue(a35 > a10 * 2);
        assertEquals(0.12, QuadPhysics.acceleration(0, 90, 0, 0, 0.12, 0.05)[2], EPS);
    }

    @Test
    public void clampLengthOnlyShrinks() {
        double[] v = {0.3, 0.4, 0.0};
        assertArrayEquals(new double[]{0.3, 0.4, 0.0}, QuadPhysics.clampLength(v, 1.0), EPS);
        double[] c = QuadPhysics.clampLength(v, 0.25);
        assertEquals(0.25, Math.sqrt(c[0] * c[0] + c[1] * c[1] + c[2] * c[2]), 1e-12);
        assertEquals(0.6, c[0] / c[1] * 0.8, 1e-12);
        assertArrayEquals(new double[]{0, 0, 0}, QuadPhysics.clampLength(new double[]{0, 0, 0}, 0.1), EPS);
    }

    @Test
    public void attitudeApproachesTargetWithoutOvershoot() {
        float v = 0f;
        float prev = -1f;
        for (int i = 0; i < 60; i++) {
            v = QuadPhysics.approach(v, 35f, 0.2f);
            assertTrue(v >= prev && v <= 35f);
            prev = v;
        }
        assertEquals(35f, v, 0.01f);
        assertEquals(10f, QuadPhysics.approach(0f, 10f, 5f), 0f); // response is clamped to 1
        assertEquals(0f, QuadPhysics.approach(0f, 10f, -1f), 0f);
    }

    @Test
    public void anglesWrapAndInterpolateTheShortWay() {
        assertEquals(-170f, QuadPhysics.wrapDegrees(190f), 1e-4f);
        assertEquals(170f, QuadPhysics.wrapDegrees(-190f), 1e-4f);
        assertEquals(179f, QuadPhysics.lerpAngle(170f, -170f, 0.45f), 1e-3f);   // crosses 180
        assertEquals(-180f, QuadPhysics.wrapDegrees(180f), 1e-4f);
        assertEquals(5f, QuadPhysics.lerpAngle(0f, 10f, 0.5f), 1e-4f);
    }

    @Test
    public void terminalSpeedMatchesSimulation() {
        double a = 0.06, drag = 0.9, v = 0;
        for (int i = 0; i < 400; i++) v = (v + a) * drag;   // OC: add acceleration, then drag
        // after the drag step the settled value is a*drag/(1-drag); before it, a/(1-drag)
        assertEquals(QuadPhysics.terminalSpeed(a, drag), v / drag, 1e-6);
        assertTrue(QuadPhysics.terminalSpeed(0.06, 0.8) < QuadPhysics.terminalSpeed(0.06, 0.9));
        assertTrue(Double.isInfinite(QuadPhysics.terminalSpeed(0.06, 1.0)));
    }
}
