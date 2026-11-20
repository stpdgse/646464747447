package com.kamikazeskies.sim;

import static org.junit.jupiter.api.Assertions.*;

import org.junit.jupiter.api.Test;

class CameraMathTest {
    @Test
    void levelDroneWithTiltedCameraLooksUpByThatAngle() {
        CameraMath.View v = CameraMath.view(Quat.IDENTITY, 25);
        assertEquals(0, v.yawDeg(), 1e-9);
        assertEquals(-25, v.pitchDeg(), 1e-9);       // negative pitch = looking up
        assertEquals(0, v.rollDeg(), 1e-9);
    }

    @Test
    void headingFollowsMinecraftYaw() {
        assertEquals(90, CameraMath.view(Quat.fromMinecraftYaw(90), 0).yawDeg(), 1e-9);
        assertEquals(-90, CameraMath.view(Quat.fromMinecraftYaw(-90), 0).yawDeg(), 1e-9);
        assertEquals(180, Math.abs(CameraMath.view(Quat.fromMinecraftYaw(180), 0).yawDeg()), 1e-9);
    }

    @Test
    void noseDownPitchesTheViewDown() {
        Quat noseDown = Quat.fromAxisAngle(new Vec3d(1, 0, 0), Math.toRadians(30));   // +x body axis = nose down
        assertEquals(30, CameraMath.view(noseDown, 0).pitchDeg(), 0.05);
        assertEquals(0, CameraMath.view(noseDown, 0).rollDeg(), 0.05);
    }

    @Test
    void rollRightGivesPositiveRollAndMatchesBodyRoll() {
        Quat rolled = Quat.fromAxisAngle(new Vec3d(0, 0, 1), Math.toRadians(30));      // +z body axis = roll right
        CameraMath.View v = CameraMath.view(rolled, 0);
        assertEquals(30, rolled.rollDeg(), 0.05);
        assertEquals(30, Math.abs(v.rollDeg()), 0.05);
        assertEquals(Math.signum(rolled.rollDeg()), Math.signum(v.rollDeg()), 0);
        assertEquals(0, v.pitchDeg(), 0.05);
    }

    @Test
    void rollIsContinuousThroughAFullBarrelRoll() {
        double prev = 0;
        Quat q = Quat.IDENTITY;
        for (int i = 1; i <= 360; i++) {
            q = q.integrate(new Vec3d(0, 0, Math.toRadians(1)), 1.0);
            double roll = CameraMath.view(q, 25).rollDeg();
            double delta = Math.abs(roll - prev);
            assertTrue(delta < 3.0 || Math.abs(delta - 360) < 3.0, "jump of " + delta + " at " + i + " deg");
            prev = roll;
        }
    }

    @Test
    void invertedFlightGivesHalfTurnRoll() {
        Quat inverted = Quat.fromAxisAngle(new Vec3d(0, 0, 1), Math.PI);
        assertEquals(180, Math.abs(CameraMath.view(inverted, 0).rollDeg()), 0.05);
    }

    @Test
    void straightDownDoesNotProduceNaN() {
        Quat down = Quat.fromAxisAngle(new Vec3d(1, 0, 0), Math.toRadians(90));
        CameraMath.View v = CameraMath.view(down, 0);
        assertTrue(Double.isFinite(v.yawDeg()) && Double.isFinite(v.pitchDeg()) && Double.isFinite(v.rollDeg()));
    }
}
