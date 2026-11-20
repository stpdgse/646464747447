package com.kamikazeskies.destruction.core;

import com.kamikazeskies.sim.Vec3d;

/**
 * Shape and strength of a warhead's block damage.
 *
 * @param name            display/debug name
 * @param power           initial ray energy; each block removes (resistance + 0.3) * STEP from a ray
 * @param airAttenuation  extra energy lost per block travelled (vanilla explosions: 0.75)
 * @param coneHalfAngle   degrees; 180 = omnidirectional, small = shaped charge jet along {@code axis}
 * @param maxRange        rays stop after this many blocks, however much energy is left (keeps jets from tunnelling forever)
 */
public record BlastProfile(String name, double power, double airAttenuation, double coneHalfAngle, double maxRange) {
    public static final double STEP = 0.3;

    /** General-purpose impact-fused warhead: a roughly spherical blast (about TNT strength). */
    public static final BlastProfile IMPACT = new BlastProfile("impact", 5.5, 0.75, 180, 9);
    /** Shaped charge: a narrow, deep jet. Little collateral damage, but punches through armor and bunker walls. */
    public static final BlastProfile SHAPED = new BlastProfile("shaped", 30.0, 0.35, 9, 8);

    public boolean isDirectional() { return coneHalfAngle < 179.9; }

    public BlastProfile withPower(double newPower) { return new BlastProfile(name, newPower, airAttenuation, coneHalfAngle, maxRange); }

    /** Convenience for tests and tools. */
    public static Vec3d axisOf(double x, double y, double z) { return new Vec3d(x, y, z).normalized(); }
}
