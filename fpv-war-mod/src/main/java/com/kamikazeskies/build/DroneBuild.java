package com.kamikazeskies.build;

import com.kamikazeskies.drone.Warhead;
import com.kamikazeskies.sim.Battery;
import com.kamikazeskies.sim.DroneParams;

/**
 * A drone assembled from parts: seven tiered parts (1 = basic, 3 = best), an optional warhead and an optional blackbox.
 * Everything the flight sim needs (mass, thrust, battery, response, range, toughness, camera) is derived from it, so
 * better parts are a real trade-off: heavier pack = longer flights but slower acceleration, bigger warhead = more weight.
 *
 * <p>Pure Java; the tables below are game-balance numbers meant to be tuned.
 */
public record DroneBuild(int frame, int motor, int prop, int battery, int flightController, int camera, int antenna,
                         Warhead warhead, boolean blackbox) {

    // ---- balance tables, index = tier - 1 -----------------------------------------------------------
    static final double BASE_MASS = 0.19;                                   // ESC, wiring, hardware (kg)
    static final double[] FRAME_MASS = {0.130, 0.115, 0.105};
    static final double[] CRASH_SPEED = {8.0, 10.5, 13.5};                  // impact speed the frame survives (m/s)
    static final double[] MOTOR_THRUST = {7.0, 9.0, 11.5};                  // per motor (N)
    static final double[] MOTOR_MASS = {0.105, 0.125, 0.150};               // all four (kg)
    static final double[] MOTOR_MAX_CURRENT = {85, 130, 175};               // whole drone at full thrust (A)
    static final double[] PROP_RESPONSE = {0.036, 0.030, 0.025};            // motor time constant (s)
    static final double[] PROP_EFFICIENCY = {1.00, 0.93, 0.86};             // current multiplier (lower = better)
    static final double[] PROP_MASS = {0.012, 0.011, 0.010};
    static final double[] BATTERY_MAH = {900, 1300, 1800};
    static final double[] BATTERY_RESISTANCE = {0.0046, 0.0035, 0.0028};    // ohms per cell
    static final double[] BATTERY_MASS = {0.085, 0.140, 0.200};
    static final double[] FC_GAIN = {0.85, 1.00, 1.18};
    static final double[] FC_MASS = {0.012, 0.011, 0.010};
    static final double[] CAMERA_SIGNAL_BONUS = {0.00, 0.07, 0.14};         // added to the signal shown on the video feed
    static final double[] CAMERA_FOV = {95, 105, 115};
    static final double[] CAMERA_MASS = {0.020, 0.018, 0.016};
    static final double[] ANTENNA_RANGE = {0.6, 1.0, 1.8};                  // multiplies the configured radio range
    static final double[] ANTENNA_MASS = {0.004, 0.005, 0.006};
    static final double WARHEAD_IMPACT_MASS = 0.16, WARHEAD_SHAPED_MASS = 0.20, BLACKBOX_MASS = 0.015;

    /** A balanced starter drone: tier 2 everywhere, no warhead, no blackbox. */
    public static final DroneBuild STANDARD = new DroneBuild(2, 2, 2, 2, 2, 2, 2, Warhead.NONE, false);

    public DroneBuild {
        frame = clampTier(frame); motor = clampTier(motor); prop = clampTier(prop); battery = clampTier(battery);
        flightController = clampTier(flightController); camera = clampTier(camera); antenna = clampTier(antenna);
        if (warhead == null) warhead = Warhead.NONE;
    }

    private static int clampTier(int t) { return Math.max(1, Math.min(PartKind.MAX_TIER, t)); }
    private static double at(double[] table, int tier) { return table[tier - 1]; }

    // ---- editing ------------------------------------------------------------------------------------
    public DroneBuild withTier(PartKind kind, int tier) {
        return switch (kind) {
            case FRAME -> new DroneBuild(tier, motor, prop, battery, flightController, camera, antenna, warhead, blackbox);
            case MOTOR -> new DroneBuild(frame, tier, prop, battery, flightController, camera, antenna, warhead, blackbox);
            case PROP -> new DroneBuild(frame, motor, tier, battery, flightController, camera, antenna, warhead, blackbox);
            case BATTERY -> new DroneBuild(frame, motor, prop, tier, flightController, camera, antenna, warhead, blackbox);
            case FLIGHT_CONTROLLER -> new DroneBuild(frame, motor, prop, battery, tier, camera, antenna, warhead, blackbox);
            case CAMERA -> new DroneBuild(frame, motor, prop, battery, flightController, tier, antenna, warhead, blackbox);
            case ANTENNA -> new DroneBuild(frame, motor, prop, battery, flightController, camera, tier, warhead, blackbox);
            default -> throw new IllegalArgumentException(kind + " is not a tiered part");
        };
    }

    public DroneBuild withWarhead(Warhead w) { return new DroneBuild(frame, motor, prop, battery, flightController, camera, antenna, w, blackbox); }
    public DroneBuild withBlackbox(boolean b) { return new DroneBuild(frame, motor, prop, battery, flightController, camera, antenna, warhead, b); }

    /** @return a reason this combination cannot be assembled, or {@code null} if it can */
    public String validate() {
        if (blackbox && flightController < 2) return "blackbox_needs_tier2_controller";
        return null;
    }

    // ---- derived stats ------------------------------------------------------------------------------
    public double massKg() {
        return BASE_MASS + at(FRAME_MASS, frame) + at(MOTOR_MASS, motor) + at(PROP_MASS, prop) + at(BATTERY_MASS, battery)
                + at(FC_MASS, flightController) + at(CAMERA_MASS, camera) + at(ANTENNA_MASS, antenna)
                + switch (warhead) { case NONE -> 0; case IMPACT -> WARHEAD_IMPACT_MASS; case SHAPED -> WARHEAD_SHAPED_MASS; }
                + (blackbox ? BLACKBOX_MASS : 0);
    }

    /** Flight-model parameters for this build. */
    public DroneParams toParams() {
        DroneParams base = DroneParams.fiveInch();
        double m = massKg();
        double gain = at(FC_GAIN, flightController);
        return new DroneParams(
                m, at(MOTOR_THRUST, motor), base.armHalfSpan(), base.yawTorqueRatio(),
                0.00385 * m, 0.00692 * m,                                   // inertia scales with mass
                at(PROP_RESPONSE, prop), base.dragCoefficient(), base.hoverStick(),
                base.rollPitchRates(), base.yawRates(), base.maxAngleDeg(), base.angleGain(), base.maxAngleRateDegS(),
                base.ratePGain() * gain, base.rateIGain() * gain, base.yawPGain() * gain, base.yawIGain() * gain);
    }

    public Battery.Spec batterySpec() { return new Battery.Spec(4, at(BATTERY_MAH, battery), at(BATTERY_RESISTANCE, battery)); }

    /** Current drawn by the whole drone at full thrust (A): stronger motors draw more, efficient props draw less. */
    public double maxCurrentA() { return at(MOTOR_MAX_CURRENT, motor) * at(PROP_EFFICIENCY, prop); }

    public double rangeMultiplier() { return at(ANTENNA_RANGE, antenna); }
    public double crashSpeed() { return at(CRASH_SPEED, frame); }
    public double fovDegrees() { return at(CAMERA_FOV, camera); }

    /** The signal quality shown on the video feed: a better camera/transmitter cleans up a weak link. */
    public double videoSignal(double rssi) {
        return rssi <= 0.0 ? 0.0 : Math.min(1.0, rssi + at(CAMERA_SIGNAL_BONUS, camera));
    }

    /** Summary numbers for tooltips and balance checks. */
    public record Stats(double massKg, double thrustToWeight, double hoverMinutes, double rangeMultiplier, double crashSpeed,
                        double packMah, double fov) {}

    public Stats stats() {
        DroneParams p = toParams();
        double tw = 4 * p.maxThrustPerMotor() / (p.mass() * DroneParams.GRAVITY);
        double hoverA = Battery.currentFor(p.hoverThrustFraction(), maxCurrentA());
        double minutes = at(BATTERY_MAH, battery) / (hoverA * 1000.0) * 60.0;
        return new Stats(p.mass(), tw, minutes, rangeMultiplier(), crashSpeed(), at(BATTERY_MAH, battery), fovDegrees());
    }

    // ---- per-part numbers for tooltips ---------------------------------------------------------------
    /** Mass of one part in kilograms. */
    public static double partMass(PartKind kind, int tier) {
        int t = clampTier(tier);
        return switch (kind) {
            case FRAME -> at(FRAME_MASS, t); case MOTOR -> at(MOTOR_MASS, t); case PROP -> at(PROP_MASS, t);
            case BATTERY -> at(BATTERY_MASS, t); case FLIGHT_CONTROLLER -> at(FC_MASS, t); case CAMERA -> at(CAMERA_MASS, t);
            case ANTENNA -> at(ANTENNA_MASS, t); case WARHEAD_IMPACT -> WARHEAD_IMPACT_MASS; case WARHEAD_SHAPED -> WARHEAD_SHAPED_MASS;
            case BLACKBOX -> BLACKBOX_MASS;
        };
    }

    /** The headline number of a part: crash speed (m/s), thrust per motor (N), response (ms), pack (mAh), gain, signal bonus, range x, 0 otherwise. */
    public static double partStat(PartKind kind, int tier) {
        int t = clampTier(tier);
        return switch (kind) {
            case FRAME -> at(CRASH_SPEED, t); case MOTOR -> at(MOTOR_THRUST, t); case PROP -> at(PROP_RESPONSE, t) * 1000.0;
            case BATTERY -> at(BATTERY_MAH, t); case FLIGHT_CONTROLLER -> at(FC_GAIN, t); case CAMERA -> at(CAMERA_FOV, t);
            case ANTENNA -> at(ANTENNA_RANGE, t); default -> 0.0;
        };
    }

    // ---- compact encoding (17 bits) for entity data and item components ----------------------------------
    public int encode() {
        int v = 0;
        int[] tiers = {frame, motor, prop, battery, flightController, camera, antenna};
        for (int i = 0; i < tiers.length; i++) v |= (tiers[i] - 1) << (2 * i);
        v |= warhead.ordinal() << 14;
        if (blackbox) v |= 1 << 16;
        return v;
    }

    public static DroneBuild decode(int v) {
        return new DroneBuild((v & 3) + 1, ((v >> 2) & 3) + 1, ((v >> 4) & 3) + 1, ((v >> 6) & 3) + 1, ((v >> 8) & 3) + 1,
                ((v >> 10) & 3) + 1, ((v >> 12) & 3) + 1, Warhead.byOrdinal((v >> 14) & 3), ((v >> 16) & 1) != 0);
    }
}
