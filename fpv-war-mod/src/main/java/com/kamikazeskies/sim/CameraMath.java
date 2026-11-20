package com.kamikazeskies.sim;

/**
 * FPV camera orientation derived from the drone's body quaternion, in Minecraft view terms.
 * Works through flips and rolls (no Euler singularities except looking exactly straight up/down).
 *
 * <p>The camera is mounted on the drone tilted up from the nose by {@code tiltUpDeg} (real FPV quads tilt the
 * camera up 20-40 degrees so the view stays level when flying forward fast).
 */
public final class CameraMath {
    private CameraMath() {}

    /** @param yawDeg Minecraft yaw (0 = +Z), @param pitchDeg positive = looking down, @param rollDeg clockwise as seen by the pilot */
    public record View(double yawDeg, double pitchDeg, double rollDeg) {}

    public static View view(Quat bodyToWorld, double tiltUpDeg) {
        double t = Math.toRadians(tiltUpDeg);
        Vec3d forward = bodyToWorld.rotate(new Vec3d(0, Math.sin(t), Math.cos(t))).normalized();
        Vec3d up = bodyToWorld.rotate(new Vec3d(0, Math.cos(t), -Math.sin(t))).normalized();

        double yaw = Math.toDegrees(Math.atan2(-forward.x(), forward.z()));
        double pitch = -Math.toDegrees(Math.asin(Math.max(-1, Math.min(1, forward.y()))));

        // roll: angle from the horizon-up vector to the camera up vector, about the forward axis.
        // positive = the pilot's right side drops (clockwise as seen looking forward).
        Vec3d horizonUp = Vec3d.UP.sub(forward.scale(forward.y()));
        double roll;
        if (horizonUp.length() < 1e-6) {
            roll = 0; // looking straight up/down: roll is undefined, keep level
        } else {
            horizonUp = horizonUp.normalized();
            double s = horizonUp.cross(up).dot(forward);
            double c = horizonUp.dot(up);
            roll = Math.toDegrees(Math.atan2(s, c));
        }
        return new View(yaw, pitch, roll);
    }
}
