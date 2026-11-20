package com.kamikazeskies.sim;

import static org.junit.jupiter.api.Assertions.*;

import org.junit.jupiter.api.Test;

class QuatTest {
    private static void assertVec(Vec3d expected, Vec3d actual, double tol) {
        assertEquals(expected.x(), actual.x(), tol, "x");
        assertEquals(expected.y(), actual.y(), tol, "y");
        assertEquals(expected.z(), actual.z(), tol, "z");
    }

    @Test
    void identityKeepsMinecraftAxes() {
        assertVec(new Vec3d(0, 1, 0), Quat.IDENTITY.bodyUp(), 1e-12);
        assertVec(new Vec3d(0, 0, 1), Quat.IDENTITY.bodyForward(), 1e-12);
        assertVec(new Vec3d(-1, 0, 0), Quat.IDENTITY.bodyRight(), 1e-12);
        assertEquals(0, Quat.IDENTITY.minecraftYawDeg(), 1e-9);
    }

    @Test
    void minecraftYawRoundTrips() {
        for (double yaw : new double[] {-170, -90, -30, 0, 45, 90, 135, 179}) {
            assertEquals(yaw, Quat.fromMinecraftYaw(yaw).minecraftYawDeg(), 1e-6, "yaw " + yaw);
        }
        // yaw 90 = west (-X) in Minecraft
        assertVec(new Vec3d(-1, 0, 0), Quat.fromMinecraftYaw(90).bodyForward(), 1e-9);
        // yaw -90 = east (+X)
        assertVec(new Vec3d(1, 0, 0), Quat.fromMinecraftYaw(-90).bodyForward(), 1e-9);
    }

    @Test
    void rotateAndUnrotateAreInverse() {
        Quat q = Quat.fromAxisAngle(new Vec3d(1, 2, 3), 1.1);
        Vec3d v = new Vec3d(0.3, -4, 2.5);
        assertVec(v, q.unrotate(q.rotate(v)), 1e-12);
    }

    @Test
    void integrationAboutBodyAxisTurnsByExpectedAngle() {
        Quat q = Quat.IDENTITY;
        double dt = 0.001;
        for (int i = 0; i < 1000; i++) q = q.integrate(new Vec3d(0, 0, Math.PI / 2), dt); // 90 deg/s... 1 s
        // roll about +Z body by +90 deg: right side goes down
        assertEquals(90, q.rollDeg(), 0.1);
    }

    @Test
    void integrationKeepsUnitNorm() {
        Quat q = Quat.IDENTITY;
        for (int i = 0; i < 100_000; i++) q = q.integrate(new Vec3d(3, -2, 5), 1.0 / 80);
        assertEquals(1.0, Math.sqrt(q.w() * q.w() + q.x() * q.x() + q.y() * q.y() + q.z() * q.z()), 1e-9);
    }

    @Test
    void noseDownIsPositivePitch() {
        Quat q = Quat.IDENTITY.integrate(new Vec3d(1, 0, 0), 0.5); // +x body rate = nose down
        assertTrue(q.minecraftPitchDeg() > 0);
        assertTrue(q.bodyForward().y() < 0);
    }
}
