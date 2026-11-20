package com.kamikazeskies.drone;

import com.kamikazeskies.core.KamikazeSkies;
import com.kamikazeskies.core.KsRegistries;
import com.kamikazeskies.sim.DroneParams;
import com.kamikazeskies.sim.FlightInput;

import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.phys.Vec3;

/**
 * Developer demos, enabled with {@code -Dkamikaze_skies.demo=<mode>} (used by {@code scripts/demo-screenshots.sh}):
 * <ul>
 *   <li>{@code fly}: the joining player's camera rides a drone flying a fixed script (FPV view, OSD, camera roll).</li>
 *   <li>{@code model}: a hovering drone floats in front of the player (checks the 3D model and prop animation).</li>
 * </ul>
 */
public final class DemoMode {
    private static final String MODE = System.getProperty("kamikaze_skies.demo", "");
    public static final boolean ENABLED = !MODE.isEmpty() && !MODE.equalsIgnoreCase("false");
    private static final DroneParams P = DroneParams.fiveInch();
    private static final double HOVER = P.hoverStick();

    private DemoMode() {}

    public static void start(ServerPlayer player) {
        if (!(player.level() instanceof ServerLevel level)) return;
        DroneEntity drone = KsRegistries.DRONE.get().create(level, EntitySpawnReason.COMMAND);
        if (drone == null) return;
        Vec3 look = player.getLookAngle();
        Vec3 flat = new Vec3(look.x, 0, look.z).normalize();
        boolean model = MODE.equalsIgnoreCase("model");
        Vec3 pos = player.position().add(flat.scale(model ? 1.6 : 2.5)).add(0, model ? 1.5 : 0.1, 0);
        drone.snapTo(pos);
        drone.resetOrientation(model ? player.getYRot() + 160f : player.getYRot());   // model: nose toward the viewer, a bit off-axis
        level.addFreshEntity(drone);
        if (model) {
            drone.debugSpinUpToHover();
            drone.debugScript(t -> new FlightInput(HOVER, 0, 0, 0, true, 1));
        } else {
            drone.startPilot(player);
            double groundY = drone.getY();
            drone.debugScript(t -> script(t, drone, groundY));
        }
        KamikazeSkies.LOGGER.info("KS_DEMO started ({})", MODE);
    }

    /**
     * Throttle stick for a commanded vertical acceleration (m/s^2) while tilted by {@code tiltDeg}: the vertical
     * component of thrust must carry the weight plus the commanded acceleration.
     */
    private static double stickFor(double tiltDeg, double verticalAccel) {
        double fraction = P.hoverThrustFraction() * (1 + verticalAccel / DroneParams.GRAVITY) / Math.cos(Math.toRadians(tiltDeg));
        return P.stickForThrustFraction(fraction);
    }

    /** Simple PD altitude hold: climb/descend toward {@code targetY} at the given tilt. */
    private static double altHold(DroneEntity d, double targetY, double tiltDeg) {
        double err = targetY - d.sim().position().y(), vy = d.sim().velocity().y();
        double accel = Math.max(-4, Math.min(6, 2.0 * err - 2.6 * vy));
        return stickFor(tiltDeg, accel);
    }

    /** t = ticks since the demo began (20 per second). */
    static FlightInput script(int t, DroneEntity d, double groundY) {
        double target = groundY + 8;
        double fwdTilt = 0.45 * P.maxAngleDeg(), bankTilt = Math.hypot(0.5, 0.3) * P.maxAngleDeg();
        if (t < 10) return new FlightInput(0, 0, 0, 0, false, 1);                                      // still disarmed
        if (t < 120) return new FlightInput(altHold(d, target, 0), 0, 0, 0, true, 1);                  // climb and settle at 8 m
        if (t < 230) return new FlightInput(altHold(d, target, fwdTilt), 0, 0.45, 0.12, true, 1);      // forward flight, gentle yaw
        if (t < 330) return new FlightInput(altHold(d, target, bankTilt), 0.5, 0.3, 0.35, true, 1);    // banked right turn
        if (t < 343) return new FlightInput(altHold(d, target, 0) + 0.04, 0.95, 0, 0, true, 0);        // acro roll (~0.65 s, one flip)
        return new FlightInput(altHold(d, target, 0), 0, 0, 0, true, 1);                               // recover and hover
    }
}
