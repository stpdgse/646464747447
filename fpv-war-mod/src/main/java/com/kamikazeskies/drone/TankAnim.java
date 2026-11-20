package com.kamikazeskies.drone;

/** Pure animation selection for the tank. */
public final class TankAnim {
    private TankAnim() {}
    public static final String PREFIX = "animation.tank.";
    public static final double MOVING_SPEED = 0.01;

    /** Hull animation: wreck, rolling or idling. The turret scan and the gun recoil run on their own controller. */
    public static String hull(boolean dead, double speed) { return dead ? "destroyed" : speed > MOVING_SPEED ? "drive" : "idle"; }
}
