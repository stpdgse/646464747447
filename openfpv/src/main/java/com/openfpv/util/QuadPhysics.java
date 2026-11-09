package com.openfpv.util;

/**
 * Pure maths of the angle-mode quadcopter model (no Minecraft classes, unit tested).
 *
 * Conventions follow Minecraft: yaw 0 = facing +Z (south), yaw 90 = facing -X (west). Pitch is
 * positive when the nose points DOWN (leaning forward, i.e. flying forward). Roll is positive when
 * banking to the RIGHT. The thrust vector tilts with the body, so horizontal acceleration is
 * proportional to sin(tilt).
 */
public final class QuadPhysics {
    private QuadPhysics() {}

    /** One step of a first-order lag: moves cur towards target, closing 'response' of the gap. */
    public static float approach(float cur, float target, float response) {
        float r = CameraMath.clamp(response, 0f, 1f);
        return cur + (target - cur) * r;
    }

    public static float wrapDegrees(float deg) {
        float d = deg % 360f;
        if (d >= 180f) d -= 360f;
        if (d < -180f) d += 360f;
        return d;
    }

    /** Interpolates two angles along the shortest arc. */
    public static float lerpAngle(float from, float to, float t) {
        return from + wrapDegrees(to - from) * t;
    }

    /**
     * World-space acceleration {x, y, z} in blocks/tick^2.
     *
     * @param yawDeg   body heading
     * @param pitchDeg nose-down tilt (forward acceleration)
     * @param rollDeg  right bank (sideways acceleration to the right)
     * @param throttle -1..1 vertical stick
     */
    public static double[] acceleration(double yawDeg, double pitchDeg, double rollDeg, double throttle,
                                        double accelScale, double throttleAccel) {
        double aForward = Math.sin(Math.toRadians(pitchDeg)) * accelScale;
        double aRight = Math.sin(Math.toRadians(rollDeg)) * accelScale;
        double[] f = CameraMath.forward(yawDeg, 0);
        double[] r = CameraMath.right(yawDeg);
        return new double[]{
            f[0] * aForward + r[0] * aRight,
            CameraMath.clamp(throttle, -1.0, 1.0) * throttleAccel,
            f[2] * aForward + r[2] * aRight};
    }

    /**
     * Acro (rate) mode acceleration for ANY attitude, including inverted. Thrust always points along the
     * body's up axis; level hover (throttle 0) cancels out exactly, so at small angles this equals
     * the angle-mode result. Tilted past 90 degrees the drone loses height; inverted it is pushed down.
     */
    public static double[] acroAcceleration(double yawDeg, double pitchDeg, double rollDeg, double throttle,
                                            double accelScale, double throttleAccel) {
        double p = Math.toRadians(pitchDeg), r = Math.toRadians(rollDeg);
        double uF = Math.sin(p) * Math.cos(r);
        double uR = Math.sin(r);
        double uY = Math.cos(p) * Math.cos(r);
        double thrust = accelScale + CameraMath.clamp(throttle, -1.0, 1.0) * throttleAccel;
        double[] f = CameraMath.forward(yawDeg, 0);
        double[] rt = CameraMath.right(yawDeg);
        return new double[]{
            (f[0] * uF + rt[0] * uR) * thrust,
            uY * thrust - accelScale,
            (f[2] * uF + rt[2] * uR) * thrust};
    }

    /**
     * Converts a body attitude (any pitch/roll) into camera angles Minecraft can render: pitch within
     * [-90, 90], flipping yaw and roll by 180 degrees when the drone is over the top of a loop.
     * Returns {yaw, pitch, roll}.
     */
    public static float[] normalizeView(float yaw, float pitch, float roll) {
        float p = wrapDegrees(pitch);
        if (p > 90f) {
            p = 180f - p;
            yaw += 180f;
            roll += 180f;
        } else if (p < -90f) {
            p = -180f - p;
            yaw += 180f;
            roll += 180f;
        }
        return new float[]{wrapDegrees(yaw), p, wrapDegrees(roll)};
    }

    /** Scales the vector down if it is longer than max (OC drones cap their acceleration). */
    public static double[] clampLength(double[] v, double max) {
        double len = Math.sqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]);
        if (len <= max || len == 0) return v;
        double k = max / len;
        return new double[]{v[0] * k, v[1] * k, v[2] * k};
    }

    /** Speed (blocks/tick) a constant acceleration settles at against per-tick drag. */
    public static double terminalSpeed(double accel, double drag) {
        return drag >= 1.0 ? Double.POSITIVE_INFINITY : accel / (1.0 - drag);
    }
}
