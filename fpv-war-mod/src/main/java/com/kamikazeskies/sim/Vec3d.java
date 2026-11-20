package com.kamikazeskies.sim;

/** Immutable 3D vector (double precision). World axes match Minecraft: +X east, +Y up, +Z south. Units: SI. */
public record Vec3d(double x, double y, double z) {
    public static final Vec3d ZERO = new Vec3d(0, 0, 0);
    public static final Vec3d UP = new Vec3d(0, 1, 0);

    public Vec3d add(Vec3d o) { return new Vec3d(x + o.x, y + o.y, z + o.z); }
    public Vec3d sub(Vec3d o) { return new Vec3d(x - o.x, y - o.y, z - o.z); }
    public Vec3d scale(double s) { return new Vec3d(x * s, y * s, z * s); }
    public Vec3d neg() { return new Vec3d(-x, -y, -z); }
    public double dot(Vec3d o) { return x * o.x + y * o.y + z * o.z; }
    public Vec3d cross(Vec3d o) { return new Vec3d(y * o.z - z * o.y, z * o.x - x * o.z, x * o.y - y * o.x); }
    public double lengthSq() { return dot(this); }
    public double length() { return Math.sqrt(lengthSq()); }

    public Vec3d normalized() {
        double l = length();
        return l < 1e-12 ? ZERO : scale(1.0 / l);
    }
}
