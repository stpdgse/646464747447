package com.kamikazeskies.input;

/** Small shared helpers for stick shaping. */
public final class StickMath {
    private StickMath() {}

    /** Zero inside the deadzone, rescaled so the output still reaches +-1 at full deflection. */
    public static double deadzone(double v, double dz) {
        double a = Math.abs(v);
        if (a <= dz) return 0;
        return Math.copySign(Math.min(1.0, (a - dz) / (1.0 - dz)), v);
    }

    /** Move {@code current} toward {@code target} by at most {@code maxDelta}. */
    public static double approach(double current, double target, double maxDelta) {
        if (current < target) return Math.min(target, current + maxDelta);
        return Math.max(target, current - maxDelta);
    }

    public static double clamp(double v, double lo, double hi) { return v < lo ? lo : Math.min(v, hi); }
}
