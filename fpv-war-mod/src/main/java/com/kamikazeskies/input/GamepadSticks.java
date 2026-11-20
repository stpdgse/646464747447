package com.kamikazeskies.input;

import com.kamikazeskies.sim.FlightInput;

/**
 * Gamepad mapping in "Mode 2" radio layout: left stick = throttle (up/down) and yaw (left/right),
 * right stick = pitch (up/down) and roll (left/right).
 *
 * <p>Axis inputs use the usual gamepad convention: x right = +1, y DOWN = +1.
 */
public final class GamepadSticks implements InputSource {
    public enum ThrottleMode {
        /** Stick position is the throttle (stick up = 100%). Authentic but the spring-centered stick idles at 50%. */
        DIRECT,
        /** Stick up/down changes the throttle, which then stays put (like a non-spring radio stick). */
        INCREMENTAL
    }

    /** Raw axes from the device. */
    public record Axes(double leftX, double leftY, double rightX, double rightY) {
        public static final Axes CENTERED = new Axes(0, 0, 0, 0);
    }

    public static final double DEFAULT_DEADZONE = 0.10;
    public static final double INCREMENTAL_RATE = 0.9;   // per second at full deflection

    private final PilotSwitches switches;
    private final ThrottleMode mode;
    private final double deadzone;
    private Axes axes = Axes.CENTERED;
    private double throttle;

    public GamepadSticks(PilotSwitches switches, ThrottleMode mode, double deadzone) {
        this.switches = switches;
        this.mode = mode;
        this.deadzone = deadzone;
    }

    public GamepadSticks(PilotSwitches switches) { this(switches, ThrottleMode.DIRECT, DEFAULT_DEADZONE); }

    public void setAxes(Axes axes) { this.axes = axes; }

    @Override
    public FlightInput sample(double dt) {
        double lx = StickMath.deadzone(axes.leftX(), deadzone);
        double ly = -StickMath.deadzone(axes.leftY(), deadzone);     // up = +1
        double rx = StickMath.deadzone(axes.rightX(), deadzone);
        double ry = -StickMath.deadzone(axes.rightY(), deadzone);    // up = +1

        if (mode == ThrottleMode.DIRECT) {
            throttle = (ly + 1.0) / 2.0;
        } else {
            throttle = StickMath.clamp(throttle + ly * INCREMENTAL_RATE * dt, 0, 1);
        }
        if (!switches.armed() && mode == ThrottleMode.INCREMENTAL) throttle = Math.min(throttle, 0.0);
        // right stick up = nose down (pitch forward), right = roll right, left stick right = yaw right
        return new FlightInput(throttle, rx, ry, lx, switches.armed(), switches.assist().level);
    }

    public double throttle() { return throttle; }

    @Override
    public boolean isActive() {
        return Math.abs(axes.leftX()) > deadzone || Math.abs(axes.leftY()) > deadzone
                || Math.abs(axes.rightX()) > deadzone || Math.abs(axes.rightY()) > deadzone;
    }
}
