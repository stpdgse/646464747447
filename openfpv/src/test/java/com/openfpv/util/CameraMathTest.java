package com.openfpv.util;

import static org.junit.Assert.assertArrayEquals;
import static org.junit.Assert.assertEquals;
import static org.junit.Assert.assertTrue;

import org.junit.Test;

public class CameraMathTest {
    private static final double EPS = 1e-9;

    private static double dot(double[] a, double[] b) {
        return a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    }

    private static double len(double[] a) {
        return Math.sqrt(dot(a, a));
    }

    @Test
    public void cameraFrameIsOrthonormalForAnyOrientation() {
        for (double yaw = -360; yaw <= 360; yaw += 22.5) {
            for (double pitch = -89; pitch <= 89; pitch += 17) {
                double[] f = CameraMath.forward(yaw, pitch);
                double[] r = CameraMath.right(yaw);
                double[] u = CameraMath.up(yaw, pitch);
                assertEquals(1.0, len(f), EPS);
                assertEquals(1.0, len(r), EPS);
                assertEquals(1.0, len(u), EPS);
                assertEquals("f.r yaw=" + yaw + " pitch=" + pitch, 0.0, dot(f, r), EPS);
                assertEquals("f.u yaw=" + yaw + " pitch=" + pitch, 0.0, dot(f, u), EPS);
                assertEquals("r.u yaw=" + yaw + " pitch=" + pitch, 0.0, dot(r, u), EPS);
                assertTrue("up points skyward", u[1] > 0);
            }
        }
    }

    @Test
    public void forwardMatchesMinecraftConventions() {
        assertArrayEquals(new double[]{0, 0, 1}, CameraMath.forward(0, 0), EPS);      // south = +Z
        assertArrayEquals(new double[]{-1, 0, 0}, CameraMath.forward(90, 0), EPS);    // west = -X
        assertArrayEquals(new double[]{0, 0, -1}, CameraMath.forward(180, 0), EPS);   // north = -Z
        assertArrayEquals(new double[]{1, 0, 0}, CameraMath.forward(-90, 0), EPS);    // east = +X
        assertEquals(-1.0, CameraMath.forward(0, 90)[1], EPS);                        // positive pitch = down
        // Facing south, your right hand points west (-X).
        assertArrayEquals(new double[]{-1, 0, 0}, CameraMath.right(0), EPS);
    }

    @Test
    public void centreRayOfOddGridIsForward() {
        double[] f = CameraMath.forward(33, -12);
        double[] ray = CameraMath.gridRay(33, -12, 70, 5, 5, 2, 2);
        assertArrayEquals(f, ray, 1e-9);
    }

    @Test
    public void gridRaysAreUnitAndSymmetric() {
        int w = 9, h = 5;
        for (int row = 0; row < h; row++) {
            for (int col = 0; col < w; col++) {
                assertEquals(1.0, len(CameraMath.gridRay(10, 5, 90, w, h, col, row)), EPS);
            }
        }
        double[] f = CameraMath.forward(0, 0);
        double[] r = CameraMath.right(0);
        double[] left = CameraMath.gridRay(0, 0, 90, w, h, 0, 2);
        double[] right = CameraMath.gridRay(0, 0, 90, w, h, w - 1, 2);
        assertEquals(dot(left, r), -dot(right, r), 1e-9);
        assertTrue("right-hand column leans right", dot(right, r) > 0);
        assertEquals(dot(left, f), dot(right, f), 1e-9);
    }

    @Test
    public void topRowLooksUpBottomRowLooksDown() {
        double[] top = CameraMath.gridRay(0, 0, 70, 8, 8, 4, 0);
        double[] bottom = CameraMath.gridRay(0, 0, 70, 8, 8, 4, 7);
        assertTrue(top[1] > 0);
        assertTrue(bottom[1] < 0);
        assertEquals(top[1], -bottom[1], 1e-9);
    }

    @Test
    public void edgeColumnsCoverTheFieldOfView() {
        // With a 1xN-wide grid the single column is dead centre.
        double[] f = CameraMath.forward(0, 0);
        assertArrayEquals(f, CameraMath.gridRay(0, 0, 70, 1, 1, 0, 0), EPS);
        // For a very wide grid the outermost ray is just inside half the FOV.
        int w = 1000;
        double[] edge = CameraMath.gridRay(0, 0, 80, w, 1, w - 1, 0);
        double angle = Math.toDegrees(Math.acos(dot(edge, f)));
        assertEquals(40.0, angle, 0.1);
    }

    @Test
    public void flightVectorFollowsLookYaw() {
        assertArrayEquals(new double[]{0, 0, 1}, CameraMath.flightVector(0, 1, 0, 0), EPS);
        assertArrayEquals(new double[]{-1, 0, 0}, CameraMath.flightVector(90, 1, 0, 0), EPS);
        assertArrayEquals(new double[]{0, 0, -1}, CameraMath.flightVector(0, -1, 0, 0), EPS);
        // strafe right while facing south goes west (-X)
        assertArrayEquals(new double[]{-1, 0, 0}, CameraMath.flightVector(0, 0, 1, 0), EPS);
        assertArrayEquals(new double[]{0, 1, 0}, CameraMath.flightVector(123, 0, 0, 1), EPS);
        assertArrayEquals(new double[]{0, 0, 0}, CameraMath.flightVector(45, 0, 0, 0), EPS);
    }

    @Test
    public void diagonalFlightIsNotFaster() {
        double[] d = CameraMath.flightVector(30, 1, 1, 1);
        assertEquals(1.0, len(d), 1e-9);
        double[] straight = CameraMath.flightVector(30, 0.5, 0, 0);
        assertEquals(0.5, len(straight), 1e-9);
    }

    @Test
    public void signalQualityFallsOffOverTheOuterHalf() {
        assertEquals(1f, CameraMath.signalQuality(0, 100), 0f);
        assertEquals(1f, CameraMath.signalQuality(50, 100), 0f);
        assertEquals(0.5f, CameraMath.signalQuality(75, 100), 1e-6f);
        assertEquals(0f, CameraMath.signalQuality(100, 100), 0f);
        assertEquals(0f, CameraMath.signalQuality(500, 100), 0f);
        assertEquals(0f, CameraMath.signalQuality(10, 0), 0f);
        float prev = 1f;
        for (double d = 0; d <= 110; d += 5) {
            float q = CameraMath.signalQuality(d, 100);
            assertTrue("monotonic at " + d, q <= prev + 1e-6f);
            prev = q;
        }
    }

    @Test
    public void fovAndZoomAreClampedAndInverse() {
        assertEquals(10f, CameraMath.clampFov(-5), 0f);
        assertEquals(110f, CameraMath.clampFov(500), 0f);
        assertEquals(70f, CameraMath.clampFov(Float.NaN), 0f);
        assertEquals(70f, CameraMath.zoomToFov(1f), 1e-4f);
        assertEquals(35f, CameraMath.zoomToFov(2f), 1e-4f);
        assertEquals(10f, CameraMath.zoomToFov(1000f), 1e-4f);   // max zoom 8x -> 8.75 -> clamped to 10
        assertEquals(70f, CameraMath.zoomToFov(0f), 0f);
        assertEquals(2f, CameraMath.fovToZoom(35f), 1e-4f);
        for (float z = 1f; z <= 7f; z += 0.5f) {
            assertEquals(z, CameraMath.fovToZoom(CameraMath.zoomToFov(z)), 1e-3f);
        }
    }

    @Test
    public void gimbalLimits() {
        assertEquals(90f, CameraMath.clampTilt(200), 0f);
        assertEquals(-90f, CameraMath.clampTilt(-200), 0f);
        assertEquals(0f, CameraMath.clampTilt(Float.NaN), 0f);
        assertEquals(170f, CameraMath.wrapPan(170), 0f);
        assertEquals(-170f, CameraMath.wrapPan(190), 1e-4f);
        assertEquals(180f, CameraMath.wrapPan(-180), 1e-4f);
        assertEquals(10f, CameraMath.wrapPan(370), 1e-4f);
        assertEquals(0f, CameraMath.wrapPan(Float.POSITIVE_INFINITY), 0f);
    }

    @Test
    public void compassHeadingUsesNorthZero() {
        assertEquals(180f, CameraMath.compassHeading(0f), 1e-4f);     // facing south
        assertEquals(0f, CameraMath.compassHeading(180f), 1e-4f);     // facing north
        assertEquals(90f, CameraMath.compassHeading(-90f), 1e-4f);    // facing east
        assertEquals(270f, CameraMath.compassHeading(90f), 1e-4f);    // facing west
        assertEquals(0f, CameraMath.compassHeading(-180f), 1e-4f);
        for (float y = -720; y <= 720; y += 13) {
            float h = CameraMath.compassHeading(y);
            assertTrue(h >= 0f && h < 360f);
        }
    }

    @Test
    public void energyFractionIsSafe() {
        assertEquals(0.25, CameraMath.energyFraction(25, 100), 0);
        assertEquals(1.0, CameraMath.energyFraction(500, 100), 0);
        assertEquals(0.0, CameraMath.energyFraction(5, 0), 0);
        assertEquals(0.0, CameraMath.energyFraction(-5, 100), 0);
    }
}
