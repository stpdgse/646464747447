package com.kamikazeskies.input;

/**
 * Arm and assist-mode state shared by all input sources. Like a real radio, arming is refused unless
 * the throttle is low.
 */
public final class PilotSwitches {
    public static final double ARM_THROTTLE_LIMIT = 0.05;

    private boolean armed;
    private AssistMode assist = AssistMode.ANGLE;
    private boolean armRefused;

    public boolean armed() { return armed; }
    public AssistMode assist() { return assist; }
    /** True for the call after an arm attempt with the throttle too high; the UI shows a warning. */
    public boolean armRefused() { return armRefused; }

    /** @return the new armed state */
    public boolean toggleArm(double throttle) {
        armRefused = false;
        if (armed) {
            armed = false;
        } else if (throttle <= ARM_THROTTLE_LIMIT) {
            armed = true;
        } else {
            armRefused = true;
        }
        return armed;
    }

    public void cycleAssist() { assist = assist.next(); }

    public void setAssist(AssistMode mode) { this.assist = mode; }

    public void disarm() { armed = false; }
}
