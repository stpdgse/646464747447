package com.openfpv.util;

/**
 * Pure math helpers (no Minecraft classes) so they can be unit tested without a game.
 * Angle convention follows Minecraft: yaw 0 = facing +Z (south), yaw 90 = facing -X (west);
 * pitch is positive when looking DOWN. Lua-facing gimbal tilt is positive when looking UP.
 */
public final class CameraMath {
    private CameraMath() {}

    public static final float MIN_FOV = 10f;
    public static final float MAX_FOV = 110f;
    public static final float MAX_ZOOM = 8f;
    public static final float BASE_FOV = 70f;

    public static float clamp(float v, float lo, float hi) {
        return v < lo ? lo : (v > hi ? hi : v);
    }

    public static double clamp(double v, double lo, double hi) {
        return v < lo ? lo : (v > hi ? hi : v);
    }

    public static float clampFov(float fov) {
        if (Float.isNaN(fov)) return BASE_FOV;
        return clamp(fov, MIN_FOV, MAX_FOV);
    }

    /** Zoom factor (1 = no zoom) to field of view, relative to {@link #BASE_FOV}. */
    public static float zoomToFov(float zoom) {
        if (Float.isNaN(zoom) || zoom <= 0f) return BASE_FOV;
        return clampFov(BASE_FOV / clamp(zoom, 1f / 8f, MAX_ZOOM));
    }

    public static float fovToZoom(float fov) {
        return BASE_FOV / clampFov(fov);
    }

    /** Gimbal tilt in degrees, positive = up. */
    public static float clampTilt(float tilt) {
        if (Float.isNaN(tilt)) return 0f;
        return clamp(tilt, -90f, 90f);
    }

    /** Gimbal pan in degrees, positive = right, wrapped to (-180, 180]. */
    public static float wrapPan(float pan) {
        if (Float.isNaN(pan) || Float.isInfinite(pan)) return 0f;
        float p = pan % 360f;
        if (p > 180f) p -= 360f;
        if (p <= -180f) p += 360f;
        return p;
    }

    /**
     * Signal quality in [0,1]. Full quality inside the first half of the range, then falling off
     * linearly to 0 at the range limit.
     */
    public static float signalQuality(double distance, double range) {
        if (range <= 0) return 0f;
        double ratio = distance / range;
        if (ratio <= 0.5) return 1f;
        if (ratio >= 1.0) return 0f;
        return (float) (1.0 - (ratio - 0.5) / 0.5);
    }

    public static double energyFraction(double buffer, double size) {
        if (size <= 0) return 0;
        return clamp(buffer / size, 0.0, 1.0);
    }

    /** Unit forward vector for the given Minecraft yaw/pitch (degrees). */
    public static double[] forward(double yawDeg, double pitchDeg) {
        double y = Math.toRadians(yawDeg), p = Math.toRadians(pitchDeg);
        double cp = Math.cos(p);
        return new double[]{-Math.sin(y) * cp, -Math.sin(p), Math.cos(y) * cp};
    }

    /** Unit right vector (horizontal) for the given yaw. */
    public static double[] right(double yawDeg) {
        double y = Math.toRadians(yawDeg);
        return new double[]{-Math.cos(y), 0, -Math.sin(y)};
    }

    /** Unit up vector of the camera frame for the given yaw/pitch. */
    public static double[] up(double yawDeg, double pitchDeg) {
        double y = Math.toRadians(yawDeg), p = Math.toRadians(pitchDeg);
        return new double[]{-Math.sin(y) * Math.sin(p), Math.cos(p), Math.cos(y) * Math.sin(p)};
    }

    /**
     * Direction (normalized) of the ray through cell (col,row) of a w*h grid for a pinhole camera.
     * hFovDeg is the horizontal field of view; the vertical one follows from the grid aspect.
     * Row 0 is the top row, column 0 is the left column.
     */
    public static double[] gridRay(double yawDeg, double pitchDeg, double hFovDeg,
                                   int w, int h, int col, int row) {
        double[] f = forward(yawDeg, pitchDeg);
        double[] r = right(yawDeg);
        double[] u = up(yawDeg, pitchDeg);
        double tanH = Math.tan(Math.toRadians(hFovDeg) / 2.0);
        double tanV = tanH * h / (double) w;
        double sx = ((col + 0.5) / w - 0.5) * 2.0 * tanH;
        double sy = (0.5 - (row + 0.5) / h) * 2.0 * tanV;
        double dx = f[0] + r[0] * sx + u[0] * sy;
        double dy = f[1] + r[1] * sx + u[1] * sy;
        double dz = f[2] + r[2] * sx + u[2] * sy;
        double len = Math.sqrt(dx * dx + dy * dy + dz * dz);
        return new double[]{dx / len, dy / len, dz / len};
    }

    /**
     * Desired flight direction in world space from pilot input. forward/strafe/up are in [-1,1];
     * strafe is positive to the RIGHT. Result length is clamped to 1 so diagonals are not faster.
     */
    public static double[] flightVector(double yawDeg, double forward, double strafe, double up) {
        double y = Math.toRadians(yawDeg);
        double fx = -Math.sin(y), fz = Math.cos(y);
        double rx = -Math.cos(y), rz = -Math.sin(y);
        double dx = fx * forward + rx * strafe;
        double dy = up;
        double dz = fz * forward + rz * strafe;
        double len = Math.sqrt(dx * dx + dy * dy + dz * dz);
        if (len > 1.0) {
            dx /= len; dy /= len; dz /= len;
        }
        return new double[]{dx, dy, dz};
    }

    /** Heading in degrees [0,360) where 0 = north, 90 = east (compass convention) from MC yaw. */
    public static float compassHeading(float mcYaw) {
        float h = (mcYaw + 180f) % 360f;
        if (h < 0) h += 360f;
        return h;
    }
}
