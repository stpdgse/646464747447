package com.kamikazeskies.drone;

/** Pure (Minecraft-free) animation selection for the rifleman, so it can be unit-tested. */
public final class SoldierAnim {
    private SoldierAnim() {}

    /** What the soldier is doing with his body, independent of walking. */
    public enum Stance {
        STAND, AIM, RELOAD, CROUCH, PANIC;
        public static Stance of(int ordinal) { Stance[] v = values(); return v[Math.floorMod(ordinal, v.length)]; }
    }

    /** Horizontal speed (blocks/tick) above which the run/flee cycle is used instead of walking. */
    public static final double RUN_SPEED = 0.16;
    public static final double MOVING_SPEED = 0.01;
    public static final String PREFIX = "animation.rifleman.";

    /** The steady-state animation for the current state (one-shots such as fire and flinch are triggered separately). */
    public static String steady(boolean dead, boolean fellForward, Stance stance, double speed) {
        if (dead) return fellForward ? "death_b" : "death_a";
        boolean running = speed > RUN_SPEED, moving = speed > MOVING_SPEED;
        return switch (stance) {
            case PANIC -> moving ? "flee" : "panic";
            case CROUCH -> "crouch";
            case RELOAD -> "reload";
            case AIM -> "aim";
            case STAND -> running ? "run" : moving ? "walk" : "idle";
        };
    }
}
