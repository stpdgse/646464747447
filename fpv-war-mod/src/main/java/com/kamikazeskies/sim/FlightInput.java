package com.kamikazeskies.sim;

/**
 * Normalized pilot input, identical for keyboard, gamepad and touch.
 *
 * @param throttle 0..1
 * @param roll     -1..1, positive = right
 * @param pitch    -1..1, positive = nose down / forward
 * @param yaw      -1..1, positive = nose right
 * @param armed    motors enabled
 * @param assist   0 = acro (rate mode) .. 1 = angle mode (self-leveling); values between blend
 */
public record FlightInput(double throttle, double roll, double pitch, double yaw, boolean armed, double assist) {
    public static final FlightInput DISARMED = new FlightInput(0, 0, 0, 0, false, 1);

    public FlightInput {
        throttle = clamp(throttle, 0, 1);
        roll = clamp(roll, -1, 1);
        pitch = clamp(pitch, -1, 1);
        yaw = clamp(yaw, -1, 1);
        assist = clamp(assist, 0, 1);
    }

    private static double clamp(double v, double lo, double hi) { return v < lo ? lo : Math.min(v, hi); }
}
