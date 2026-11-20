package com.kamikazeskies.sim;

/**
 * LiPo pack model: open-circuit voltage from state of charge, internal resistance (voltage sag under load),
 * capacity drain from current, and a power scale that weakens the motors as the loaded voltage collapses.
 * Pure Java; SI plus mAh and amps.
 */
public final class Battery {
    /** @param cells series cells, @param capacityMah pack capacity, @param resistanceOhmPerCell internal resistance per cell */
    public record Spec(int cells, double capacityMah, double resistanceOhmPerCell) {
        /** 4S 1300 mAh: a typical 5" freestyle pack. */
        public static Spec fourS1300() { return new Spec(4, 1300, 0.0035); }
        /** Bigger pack: longer flights, more weight (weight is handled by the drone build, not here). */
        public static Spec fourS2200() { return new Spec(4, 2200, 0.0030); }
    }

    // resting cell voltage vs state of charge (1 = full): typical LiPo curve
    private static final double[] SOC = {0.00, 0.05, 0.10, 0.20, 0.30, 0.40, 0.50, 0.60, 0.70, 0.80, 0.90, 1.00};
    private static final double[] OCV = {3.00, 3.40, 3.55, 3.68, 3.74, 3.78, 3.82, 3.87, 3.93, 4.00, 4.10, 4.20};

    /** Current drawn by the whole drone at full thrust and when the motors idle (amps). */
    public static final double MAX_CURRENT_A = 130.0, IDLE_CURRENT_A = 1.5;

    private final Spec spec;
    private double usedMah;

    public Battery(Spec spec) { this.spec = spec; }

    public Spec spec() { return spec; }

    /** State of charge in [0, 1]. */
    public double soc() { return Math.max(0, 1 - usedMah / spec.capacityMah()); }

    public double usedMah() { return usedMah; }

    /** Test/tuning hook. */
    public void setSoc(double soc) { usedMah = (1 - Math.max(0, Math.min(1, soc))) * spec.capacityMah(); }

    /** Resting voltage of one cell at the current state of charge. */
    public double restingCellVoltage() {
        double s = soc();
        for (int i = 1; i < SOC.length; i++) {
            if (s <= SOC[i]) {
                double t = (s - SOC[i - 1]) / (SOC[i] - SOC[i - 1]);
                return OCV[i - 1] + t * (OCV[i] - OCV[i - 1]);
            }
        }
        return OCV[OCV.length - 1];
    }

    /** Pack voltage while delivering {@code currentA} (sags with load). */
    public double voltage(double currentA) {
        return Math.max(0, spec.cells() * restingCellVoltage() - currentA * spec.cells() * spec.resistanceOhmPerCell());
    }

    public double cellVoltage(double currentA) { return voltage(currentA) / spec.cells(); }

    /** Current drawn for an average per-motor thrust fraction in [0, 1] (power grows roughly with thrust^1.5). */
    public static double currentFor(double averageThrustFraction) { return currentFor(averageThrustFraction, MAX_CURRENT_A); }

    /** As above for a drone whose full-thrust draw is {@code maxCurrentA} (set by its motors and props). */
    public static double currentFor(double averageThrustFraction, double maxCurrentA) {
        double f = Math.max(0, Math.min(1, averageThrustFraction));
        return IDLE_CURRENT_A + (maxCurrentA - IDLE_CURRENT_A) * Math.pow(f, 1.5);
    }

    public void drain(double currentA, double dt) { usedMah = Math.min(spec.capacityMah(), usedMah + currentA * dt / 3.6); }

    public boolean empty() { return soc() <= 0.0; }

    /**
     * Motor power available, 1.0 with a healthy pack down to ~0.1 as the loaded cell voltage falls: full above
     * 3.7 V, 0.7 at 3.3 V, 0.35 at 3.0 V, 0.1 at 2.9 V or below.
     */
    public double powerScale(double currentA) {
        double v = cellVoltage(currentA);
        if (v >= 3.7) return 1.0;
        if (v >= 3.3) return 0.7 + 0.3 * (v - 3.3) / 0.4;
        if (v >= 3.0) return 0.35 + 0.35 * (v - 3.0) / 0.3;
        if (v >= 2.9) return 0.1 + 0.25 * (v - 2.9) / 0.1;
        return 0.1;
    }

    /** Warning thresholds on the resting cell voltage, as on a real OSD. */
    public boolean low() { return restingCellVoltage() < 3.55; }
    public boolean critical() { return restingCellVoltage() < 3.35; }
}
