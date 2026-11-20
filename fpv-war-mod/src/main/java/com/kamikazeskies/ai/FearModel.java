package com.kamikazeskies.ai;

/**
 * Fear of a soldier, 0 (calm) to 1 (terror). A drone coming closer and faster raises it; blasts and seeing friends die add sharp spikes;
 * it decays slowly when nothing threatens him. Courage damps every source. Pure Java, advanced once per game tick.
 */
public final class FearModel {
    public static final double SIGHT_RANGE = 48.0;
    private double fear;
    private final double courage;

    public FearModel(double courage) { this.courage = courage; }

    public double fear() { return fear; }
    public void set(double f) { fear = clamp(f); }

    /** @param distance metres to the nearest visible drone, or NaN / infinity if none is seen
     *  @param closingSpeed metres per second the drone is approaching (negative = moving away) */
    public void tick(double distance, double closingSpeed) {
        boolean seen = !Double.isNaN(distance) && distance < SIGHT_RANGE;
        if (seen) {
            double near = Math.pow(1.0 - distance / SIGHT_RANGE, 1.4);
            double rush = Math.max(0, closingSpeed) / 25.0;                  // a drone diving at 25 m/s is as scary as it gets
            double target = clamp(near * (0.55 + 0.9 * rush)) * (1.0 - 0.45 * courage);
            if (target > fear) fear += (target - fear) * 0.08;               // fear builds over about a second
            else fear -= 0.004;
        } else {
            fear -= 0.0035 * (0.6 + courage);                                // brave men calm down faster
        }
        fear = clamp(fear);
    }

    /** A blast of strength 0..1 (already scaled by distance) goes off nearby. */
    public void blast(double strength) { fear = clamp(fear + (0.25 + 0.55 * strength) * strength * (1.0 - 0.5 * courage) * 1.6); }

    /** A friend was killed within sight. */
    public void allyDied() { fear = clamp(fear + 0.28 * (1.0 - 0.6 * courage)); }

    private static double clamp(double v) { return Math.max(0, Math.min(1, v)); }
}
