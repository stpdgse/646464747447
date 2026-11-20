package com.kamikazeskies.sim;

/**
 * How the video feed looks at a given signal quality. The client maps these to analog-video effect strengths
 * (more noise, tearing, snow), and level 5 is a lost link.
 */
public enum SignalLevel {
    CLEAN(0.60), LIGHT_NOISE(0.45), NOISY(0.30), BREAKUP(0.18), HEAVY_BREAKUP(0.08), LOST(0.0);

    /** Enter this level when the signal rises to at least this value. */
    public final double enterAt;

    SignalLevel(double enterAt) { this.enterAt = enterAt; }

    public static SignalLevel of(double rssi) {
        for (SignalLevel l : values()) if (rssi >= l.enterAt && l != LOST) return l;
        return LOST;
    }

    /** Level with hysteresis: only move to a better level once the signal clears its threshold by {@code margin}. */
    public static SignalLevel withHysteresis(SignalLevel current, double rssi, double margin) {
        SignalLevel target = of(rssi);
        if (target.ordinal() < current.ordinal()) {                // improving
            SignalLevel better = of(rssi - margin);
            return better.ordinal() < current.ordinal() ? better : current;
        }
        return target;
    }
}
