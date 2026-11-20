package com.kamikazeskies.ai;

/** Turns fear and what a soldier can see into a behaviour. Thresholds depend on courage, so conscripts break first. */
public final class Brain {
    private Brain() {}

    public static double coverThreshold(double courage) { return 0.28 + 0.30 * courage; }
    public static double panicThreshold(double courage) { return 0.55 + 0.35 * courage; }

    /** @param threatVisible a drone or an enemy operator is in view */
    public static Behaviour decide(double fear, double courage, boolean threatVisible) {
        if (fear >= panicThreshold(courage)) return Behaviour.FLEE;
        if (!threatVisible && fear < coverThreshold(courage) * 0.6) return Behaviour.IDLE;
        if (fear >= coverThreshold(courage)) return courage >= 0.5 && threatVisible ? Behaviour.ENGAGE : Behaviour.COVER;   // brave men shoot back from where they are
        return threatVisible ? Behaviour.ENGAGE : Behaviour.IDLE;
    }

    /** Seconds a behaviour is held before another may replace it (stops flicker); fleeing is held longest. */
    public static int holdTicks(Behaviour b) { return switch (b) { case FLEE -> 60; case COVER -> 40; case ENGAGE -> 20; case IDLE -> 10; }; }
}
