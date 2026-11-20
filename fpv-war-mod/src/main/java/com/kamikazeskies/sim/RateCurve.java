package com.kamikazeskies.sim;

/**
 * Betaflight-style "actual rates" curve: maps a normalized stick deflection in [-1, 1] to a
 * target angular rate in degrees/second.
 *
 * <p>Pure Java (no Minecraft dependency) so it can be unit tested and reused by the flight model.
 * {@code rate(x) = sign(x) * ( |x|*centerSensitivity + |x|^5*expo-shaped extra + ... )}; here we use the
 * common formulation: center sensitivity near zero, max rate at full stick, expo blends in a cubic term.
 */
public record RateCurve(double centerSensitivity, double maxRate, double expo) {

    public RateCurve {
        if (centerSensitivity < 0 || maxRate < centerSensitivity) {
            throw new IllegalArgumentException("require 0 <= centerSensitivity <= maxRate");
        }
        if (expo < 0 || expo > 1) {
            throw new IllegalArgumentException("expo must be in [0, 1]");
        }
    }

    /** Arcade-leaning default: gentle center, 600 deg/s at full stick. */
    public static RateCurve defaults() {
        return new RateCurve(120, 600, 0.35);
    }

    /** @param stick deflection in [-1, 1] (values outside are clamped) @return rate in deg/s */
    public double rate(double stick) {
        double x = Math.max(-1.0, Math.min(1.0, stick));
        double a = Math.abs(x);
        // blend linear and cubic response by expo, then interpolate center -> max
        double shaped = (1.0 - expo) * a + expo * a * a * a;
        double rate = centerSensitivity * a + (maxRate - centerSensitivity) * shaped;
        return Math.copySign(rate, x);
    }
}
