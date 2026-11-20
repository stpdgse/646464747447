package com.kamikazeskies.ai;

/** Chance that one rifle round hits. A 5-inch FPV at 20 m/s is almost impossible to hit; one hovering at 30 m is not. */
public final class Marksmanship {
    private Marksmanship() {}

    /** @param skill 0..1, @param fear 0..1 (shaking hands), @param targetSpeed m/s across the line of fire (use total speed as an upper bound) */
    public static double hitChance(double distance, double targetSpeed, double skill, double fear) {
        double range = Math.max(0, Math.min(1, 1.0 - distance / 70.0));
        double motion = 1.0 / (1.0 + targetSpeed * targetSpeed / 12.0);
        double steady = 1.0 - 0.6 * fear;
        return Math.max(0, Math.min(0.9, 0.9 * skill * range * motion * steady + 0.003));
    }

    /** Hit chance of a tank cannon: a shell is slow to traverse and the target is tiny, so only a hovering drone is at risk. */
    public static double cannonChance(double distance, double targetSpeed) {
        return Math.max(0, Math.min(0.6, 0.5 * (1.0 - distance / 80.0) / (1.0 + targetSpeed * targetSpeed / 3.0) + 0.002));
    }
}
