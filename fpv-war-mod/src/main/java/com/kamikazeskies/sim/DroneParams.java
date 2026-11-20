package com.kamikazeskies.sim;

/** Physical and tuning parameters of one drone build. SI units. Defaults describe a 5" freestyle quad. */
public record DroneParams(
        double mass,                // kg
        double maxThrustPerMotor,   // N
        double armHalfSpan,         // m, motor offset on each axis from center (X layout)
        double yawTorqueRatio,      // N*m of reaction torque per N of thrust
        double inertiaPitchRoll,    // kg*m^2 (Ix, Iz)
        double inertiaYaw,          // kg*m^2 (Iy)
        double motorTimeConstant,   // s, first-order motor response
        double dragCoefficient,     // N/(m/s)^2, quadratic body drag
        double hoverStick,          // throttle stick position (0..1) that hovers
        RateCurve rollPitchRates,   // acro rates, deg/s
        RateCurve yawRates,         // acro yaw rates, deg/s
        double maxAngleDeg,         // angle-mode tilt limit
        double angleGain,           // 1/s, angle-mode attitude P gain
        double maxAngleRateDegS,    // angle-mode rate ceiling
        double ratePGain,           // 1/s
        double rateIGain,           // 1/s^2
        double yawPGain,            // 1/s
        double yawIGain             // 1/s^2
) {
    public static final double GRAVITY = 9.81;

    /** Arcade-leaning realistic 5" quad: ~650 g, ~5.5:1 thrust-to-weight. */
    public static DroneParams fiveInch() {
        return new DroneParams(
                0.65, 9.0, 0.08, 0.016,
                0.0025, 0.0045,
                0.03, 0.0035, 0.36,
                new RateCurve(120, 620, 0.35),
                new RateCurve(90, 420, 0.25),
                55, 7.0, 360,
                22, 14, 12, 8);
    }

    public double hoverThrustFraction() { return mass * GRAVITY / (4 * maxThrustPerMotor); }

    /** Inverse of {@link #throttleToThrustFraction}: the stick position that yields the given collective thrust fraction. */
    public double stickForThrustFraction(double fraction) {
        double f = Math.max(1e-6, Math.min(1, fraction));
        double p = Math.log(hoverThrustFraction()) / Math.log(hoverStick);
        return Math.min(1, Math.pow(f, 1.0 / p));
    }

    /** Maps the throttle stick to a collective thrust fraction in 0..1 (power law through the hover point). */
    public double throttleToThrustFraction(double stick) {
        double s = Math.max(0, Math.min(1, stick));
        double p = Math.log(hoverThrustFraction()) / Math.log(hoverStick);
        return Math.min(1, Math.pow(s, p));
    }
}
