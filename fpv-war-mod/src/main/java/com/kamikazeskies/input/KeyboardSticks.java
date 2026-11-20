package com.kamikazeskies.input;

import com.kamikazeskies.sim.FlightInput;

/**
 * Keyboard emulation of a radio. Keys are binary, so pitch/roll/yaw ramp toward full deflection while held
 * and spring back when released. Throttle behaves like a real (non-spring) throttle stick: it stays where
 * you leave it and moves while up/down is held.
 */
public final class KeyboardSticks implements InputSource {
    /** Raw key state for one sample. */
    public record Keys(boolean pitchForward, boolean pitchBack, boolean rollLeft, boolean rollRight,
                       boolean yawLeft, boolean yawRight, boolean throttleUp, boolean throttleDown) {
        public static final Keys NONE = new Keys(false, false, false, false, false, false, false, false);

        public boolean any() {
            return pitchForward || pitchBack || rollLeft || rollRight || yawLeft || yawRight || throttleUp || throttleDown;
        }
    }

    public static final double STICK_RAMP_UP = 6.0;      // full deflection in ~0.17 s
    public static final double STICK_RAMP_DOWN = 10.0;   // recentre in 0.1 s
    public static final double THROTTLE_UP_RATE = 0.7;   // per second
    public static final double THROTTLE_DOWN_RATE = 1.2;

    private final PilotSwitches switches;
    private double throttle, roll, pitch, yaw;
    private Keys keys = Keys.NONE;

    public KeyboardSticks(PilotSwitches switches) { this.switches = switches; }

    public void setKeys(Keys keys) { this.keys = keys; }
    public double throttle() { return throttle; }

    @Override
    public FlightInput sample(double dt) {
        pitch = ramp(pitch, axis(keys.pitchForward(), keys.pitchBack()), dt);
        roll = ramp(roll, axis(keys.rollRight(), keys.rollLeft()), dt);
        yaw = ramp(yaw, axis(keys.yawRight(), keys.yawLeft()), dt);
        if (keys.throttleUp()) throttle += THROTTLE_UP_RATE * dt;
        if (keys.throttleDown()) throttle -= THROTTLE_DOWN_RATE * dt;
        throttle = StickMath.clamp(throttle, 0, 1);
        if (!switches.armed() && throttle > 0 && !keys.throttleUp()) throttle = 0;   // disarmed: stick rests low
        return new FlightInput(throttle, roll, pitch, yaw, switches.armed(), switches.assist().level);
    }

    @Override
    public boolean isActive() { return keys.any(); }

    private static double axis(boolean positive, boolean negative) { return (positive ? 1.0 : 0.0) - (negative ? 1.0 : 0.0); }

    private static double ramp(double current, double target, double dt) {
        double rate = (target == 0 || Math.signum(target) != Math.signum(current)) && current != 0 ? STICK_RAMP_DOWN : STICK_RAMP_UP;
        return StickMath.approach(current, target, rate * dt);
    }
}
