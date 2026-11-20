package com.kamikazeskies.sim;

/**
 * Tracks the link over time: smooths the raw signal, derives the video {@link SignalLevel}, decides when the control
 * link is lost (failsafe) or degraded (dropped control packets). Deterministic: randomness is passed in by the caller.
 */
public final class LinkMonitor {
    public static final double FAILSAFE_RSSI = 0.04, RECOVER_RSSI = 0.10;
    public static final int FAILSAFE_TICKS = 10, RECOVER_TICKS = 5;

    private double smoothed = 1.0;
    private SignalLevel level = SignalLevel.CLEAN;
    private boolean controlLost;
    private int lowTicks, highTicks;

    public double rssi() { return smoothed; }
    public SignalLevel level() { return level; }
    public boolean controlLost() { return controlLost; }

    /** Advance one tick with the raw (unsmoothed) signal. */
    public void update(double rawRssi) {
        double raw = Math.max(0, Math.min(1, rawRssi));
        smoothed += (raw - smoothed) * (raw < smoothed ? 0.5 : 0.25);   // drops fast, recovers slowly
        level = SignalLevel.withHysteresis(level, smoothed, 0.03);

        if (smoothed < FAILSAFE_RSSI) { lowTicks++; highTicks = 0; }
        else if (smoothed > RECOVER_RSSI) { highTicks++; lowTicks = 0; }
        else { lowTicks = 0; highTicks = 0; }
        if (!controlLost && lowTicks >= FAILSAFE_TICKS) controlLost = true;
        if (controlLost && highTicks >= RECOVER_TICKS) controlLost = false;
    }

    /** Probability that a control packet is dropped at the current quality (0 when the signal is good). */
    public double dropProbability() {
        double r = smoothed;
        return r >= 0.30 ? 0.0 : Math.min(0.6, (0.30 - r) / 0.30 * 0.6);
    }

    /** @param uniform a random number in [0, 1) supplied by the caller */
    public boolean dropPacket(double uniform) { return uniform < dropProbability(); }
}
