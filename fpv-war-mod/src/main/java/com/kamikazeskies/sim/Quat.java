package com.kamikazeskies.sim;

/**
 * Unit quaternion (w, x, y, z) describing body-to-world rotation.
 * Body axes: +Y up (thrust), +Z forward (nose), -X right. Identity = level, facing +Z (Minecraft yaw 0).
 */
public record Quat(double w, double x, double y, double z) {
    public static final Quat IDENTITY = new Quat(1, 0, 0, 0);

    public static Quat fromAxisAngle(Vec3d axis, double angle) {
        Vec3d a = axis.normalized();
        double s = Math.sin(angle / 2);
        return new Quat(Math.cos(angle / 2), a.x() * s, a.y() * s, a.z() * s);
    }

    /** Minecraft-style yaw (degrees, 0 = +Z, increasing clockwise seen from above) with level attitude. */
    public static Quat fromMinecraftYaw(double yawDeg) {
        // yaw increases turning from +Z toward -X, i.e. a rotation about +Y by -yaw.
        return fromAxisAngle(Vec3d.UP, Math.toRadians(-yawDeg));
    }

    public Quat multiply(Quat o) {
        return new Quat(
                w * o.w - x * o.x - y * o.y - z * o.z,
                w * o.x + x * o.w + y * o.z - z * o.y,
                w * o.y - x * o.z + y * o.w + z * o.x,
                w * o.z + x * o.y - y * o.x + z * o.w);
    }

    public double dot(Quat o) { return w * o.w + x * o.x + y * o.y + z * o.z; }

    /** Normalized linear interpolation along the shortest arc (cheap; fine for small steps). */
    public static Quat nlerp(Quat a, Quat b, double t) {
        double sign = a.dot(b) < 0 ? -1 : 1;
        return new Quat(
                a.w + (sign * b.w - a.w) * t, a.x + (sign * b.x - a.x) * t,
                a.y + (sign * b.y - a.y) * t, a.z + (sign * b.z - a.z) * t).normalized();
    }

    /** Spherical interpolation along the shortest arc (used for render interpolation). */
    public static Quat slerp(Quat a, Quat b, double t) {
        double d = a.dot(b);
        Quat bb = b;
        if (d < 0) { d = -d; bb = new Quat(-b.w, -b.x, -b.y, -b.z); }
        if (d > 0.9995) return nlerp(a, bb, t);
        double theta = Math.acos(d), sin = Math.sin(theta);
        double wa = Math.sin((1 - t) * theta) / sin, wb = Math.sin(t * theta) / sin;
        return new Quat(wa * a.w + wb * bb.w, wa * a.x + wb * bb.x, wa * a.y + wb * bb.y, wa * a.z + wb * bb.z).normalized();
    }

    /** Level attitude (no pitch/roll) with the same Minecraft heading as this orientation. */
    public Quat leveled() { return fromMinecraftYaw(minecraftYawDeg()); }

    public Quat conjugate() { return new Quat(w, -x, -y, -z); }

    public Quat normalized() {
        double n = Math.sqrt(w * w + x * x + y * y + z * z);
        return n < 1e-12 ? IDENTITY : new Quat(w / n, x / n, y / n, z / n);
    }

    /** Rotate a body-frame vector into the world frame. */
    public Vec3d rotate(Vec3d v) {
        Quat r = multiply(new Quat(0, v.x(), v.y(), v.z())).multiply(conjugate());
        return new Vec3d(r.x, r.y, r.z);
    }

    /** Rotate a world-frame vector into the body frame. */
    public Vec3d unrotate(Vec3d v) { return conjugate().rotate(v); }

    /** Integrate a body-frame angular velocity (rad/s) over dt. */
    public Quat integrate(Vec3d omegaBody, double dt) {
        Quat dq = new Quat(0, omegaBody.x(), omegaBody.y(), omegaBody.z());
        Quat d = multiply(dq);
        return new Quat(w + 0.5 * dt * d.w, x + 0.5 * dt * d.x, y + 0.5 * dt * d.y, z + 0.5 * dt * d.z).normalized();
    }

    public Vec3d bodyUp() { return rotate(new Vec3d(0, 1, 0)); }
    public Vec3d bodyForward() { return rotate(new Vec3d(0, 0, 1)); }
    /** Pilot's right-hand direction (body -X). */
    public Vec3d bodyRight() { return rotate(new Vec3d(-1, 0, 0)); }

    /** Minecraft yaw in degrees (0 = +Z) of the nose projected on the horizontal plane. */
    public double minecraftYawDeg() {
        Vec3d f = bodyForward();
        return Math.toDegrees(Math.atan2(-f.x(), f.z()));
    }

    /** Pitch in degrees, positive = nose down (Minecraft convention). */
    public double minecraftPitchDeg() {
        Vec3d f = bodyForward();
        return -Math.toDegrees(Math.asin(Math.max(-1, Math.min(1, f.y()))));
    }

    /** Roll in degrees, positive = right side down, measured against the horizon. */
    public double rollDeg() {
        Vec3d r = bodyRight();
        return Math.toDegrees(Math.asin(Math.max(-1, Math.min(1, -r.y()))));
    }

    /** Angle in degrees between body up and world up. */
    public double tiltDeg() {
        return Math.toDegrees(Math.acos(Math.max(-1, Math.min(1, bodyUp().y()))));
    }
}
