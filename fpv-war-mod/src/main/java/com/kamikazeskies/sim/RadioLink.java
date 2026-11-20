package com.kamikazeskies.sim;

/**
 * Radio link quality between the operator and the drone: falls off with distance, is absorbed by solid blocks in
 * the way, and is knocked down by jammers. All values are 0..1 (1 = perfect). Pure functions so they are easy to test
 * and tune; the world sampling (counting blocks on the line, finding jammers) happens in the game layer.
 */
public final class RadioLink {
    /** How quickly blocks in the way kill the signal: each metre of solid material multiplies the signal by exp(-k). */
    public static final double DEFAULT_OCCLUSION_K = 0.25;

    private RadioLink() {}

    /** Free-space part: 1 close in, falling smoothly to 0 at the antenna's range. */
    public static double distanceFactor(double distance, double range) {
        if (range <= 0) return 0;
        double x = distance / range;
        if (x <= 0.30) return 1.0;                       // generous plateau near the operator
        if (x >= 1.0) return 0.0;
        double t = (x - 0.30) / 0.70;
        return 1.0 - t * t;
    }

    /** Blocks in the way: {@code blockedMetres} of solid material along the line of sight. */
    public static double occlusionFactor(double blockedMetres, double k) { return Math.exp(-k * Math.max(0, blockedMetres)); }

    /**
     * @param jamming 0..1, the combined strength of all jammers at the drone (see {@link #jammingAt})
     */
    public static double rssi(double distance, double range, double blockedMetres, double jamming) {
        double j = Math.max(0, Math.min(1, jamming));
        return distanceFactor(distance, range) * occlusionFactor(blockedMetres, DEFAULT_OCCLUSION_K) * (1.0 - j);
    }

    /** One jammer's strength at a distance: full inside the inner third of its radius, fading to zero at the edge. */
    public static double jammingAt(double distance, double radius, double strength) {
        if (radius <= 0 || distance >= radius) return 0;
        double x = distance / radius;
        double falloff = x <= 1.0 / 3.0 ? 1.0 : 1.0 - (x - 1.0 / 3.0) / (2.0 / 3.0);
        return Math.max(0, Math.min(1, strength)) * falloff;
    }

    /** Combine several jammers: strengths compound like independent noise sources (never above 1). */
    public static double combine(double a, double b) { return 1.0 - (1.0 - a) * (1.0 - b); }
}
