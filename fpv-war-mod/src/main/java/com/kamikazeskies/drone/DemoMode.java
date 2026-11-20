package com.kamikazeskies.drone;

import com.kamikazeskies.core.KamikazeSkies;
import com.kamikazeskies.core.KsRegistries;
import com.kamikazeskies.destruction.CollapseManager;
import com.kamikazeskies.sim.DroneParams;
import com.kamikazeskies.sim.FlightInput;
import com.kamikazeskies.sim.Vec3d;

import net.minecraft.core.BlockPos;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.phys.Vec3;

/**
 * Developer demos, enabled with {@code -Dkamikaze_skies.demo=<mode>} (used by {@code scripts/demo-screenshots.sh}):
 * <ul>
 *   <li>{@code fly}: the joining player's camera rides a drone flying a fixed script (FPV view, OSD, camera roll).</li>
 *   <li>{@code model}: a hovering drone floats in front of the player (checks the 3D model and prop animation).</li>
 *   <li>{@code link}: the drone flies straight at a jammer; the picture degrades through every signal level until the link is lost.</li>
 *   <li>{@code battery}: a nearly empty pack: voltage sags, warnings flash, and the drone runs out of power.</li>
 *   <li>{@code parts}: a Drone Workbench stands ahead and the hotbar cycles through every part, drone preset and the jammer, so
 *       the real game renders each item model and texture.</li>
 *   <li>{@code strike}: a cottage stands ahead; the player's camera rides an armed drone that homes in on it. At the
 *       blast the game drops into slow motion while the structure collapses.</li>
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
        if (MODE.equalsIgnoreCase("strike")) { startStrike(player, level); return; }
        if (MODE.equalsIgnoreCase("parts")) { startParts(player, level); return; }
        if (MODE.equalsIgnoreCase("link") || MODE.equalsIgnoreCase("battery")) { startLinkOrBattery(player, level, MODE.equalsIgnoreCase("link")); return; }
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

    // ---- parts demo ----------------------------------------------------------------------------------
    private static java.util.List<net.minecraft.world.item.ItemStack> partsShowcase;
    private static ServerPlayer partsPlayer;
    private static long partsStart;

    private static void startParts(ServerPlayer player, ServerLevel level) {
        Vec3 look = player.getLookAngle();
        Vec3 flat = new Vec3(look.x, 0, look.z).normalize();
        BlockPos at = BlockPos.containing(player.position().add(flat.scale(2.2)));
        level.setBlock(at, KsRegistries.WORKBENCH.get().defaultBlockState(), 3);
        java.util.List<net.minecraft.world.item.ItemStack> list = new java.util.ArrayList<>();
        for (var part : KsRegistries.PARTS.values()) list.add(new net.minecraft.world.item.ItemStack(part.get()));
        list.add(KsRegistries.droneStack(com.kamikazeskies.build.DroneBuild.STANDARD));
        list.add(KsRegistries.droneStack(com.kamikazeskies.build.DroneBuild.STANDARD.withWarhead(Warhead.IMPACT)));
        list.add(KsRegistries.droneStack(new com.kamikazeskies.build.DroneBuild(3, 3, 3, 3, 3, 3, 3, Warhead.SHAPED, true)));
        list.add(new net.minecraft.world.item.ItemStack(KsRegistries.JAMMER_ITEM.get()));
        list.add(new net.minecraft.world.item.ItemStack(KsRegistries.WORKBENCH_ITEM.get()));
        partsShowcase = list;
        partsPlayer = player;
        partsStart = level.getGameTime();
        showPartsPage(0);
        KamikazeSkies.LOGGER.info("KS_DEMO started (parts)");
    }

    private static void showPartsPage(int page) {
        if (partsPlayer == null || partsShowcase == null) return;
        for (int i = 0; i < 9; i++) {
            int idx = page * 9 + i;
            partsPlayer.getInventory().setItem(i, idx < partsShowcase.size() ? partsShowcase.get(idx).copy() : net.minecraft.world.item.ItemStack.EMPTY);
        }
        partsPlayer.inventoryMenu.broadcastChanges();
        KamikazeSkies.LOGGER.info("KS_DEMO page {}", page);
    }

    // ---- link / battery demos -------------------------------------------------------------------------
    private static void startLinkOrBattery(ServerPlayer player, ServerLevel level, boolean link) {
        Vec3 look = player.getLookAngle();
        Vec3 flat = new Vec3(look.x, 0, look.z).normalize();
        double gy = player.getY();
        DroneEntity drone = KsRegistries.DRONE.get().create(level, EntitySpawnReason.COMMAND);
        if (drone == null) return;
        drone.snapTo(player.position().add(flat.scale(2.5)).add(0, 0.1, 0));
        drone.resetOrientation(player.getYRot());
        level.addFreshEntity(drone);
        drone.startPilot(player);
        if (link) {
            JammerEntity jammer = KsRegistries.JAMMER.get().create(level, EntitySpawnReason.COMMAND);
            if (jammer != null) {
                jammer.snapTo(player.position().add(flat.scale(95)));
                level.addFreshEntity(jammer);
            }
            double tilt = 0.2 * P.maxAngleDeg();                      // a gentle cruise so every signal level is visible
            drone.debugScript(t -> t < 12 ? new FlightInput(0, 0, 0, 0, false, 1)
                    : new FlightInput(altHold(drone, gy + 6, tilt), 0, t < 40 ? 0 : 0.2, 0, true, 1));
        } else {
            drone.debugBatterySoc(0.045);
            drone.debugScript(t -> t < 12 ? new FlightInput(0, 0, 0, 0, false, 1)
                    : new FlightInput(altHold(drone, gy + 8, 0), 0, 0, 0, true, 1));
        }
        // slow motion so each signal level / warning lasts several captured frames (software rendering is slow)
        level.getServer().tickRateManager().setTickRate(link ? 5.0f : 10.0f);
        KamikazeSkies.LOGGER.info("KS_DEMO started ({})", MODE);
    }

    // ---- strike demo ----------------------------------------------------------------------------------
    private static boolean slowMo;
    private static long slowMoStart;

    private static void startStrike(ServerPlayer player, ServerLevel level) {
        Vec3 look = player.getLookAngle();
        int fx = Math.abs(look.x) > Math.abs(look.z) ? (int) Math.signum(look.x) : 0;
        int fz = fx == 0 ? (look.z >= 0 ? 1 : -1) : 0;                   // horizontal facing axis
        int rx = -fz, rz = fx;                                           // the player's right-hand axis
        BlockPos feet = player.blockPosition();
        int gy = feet.getY();
        int dist = 34;                                                   // metres from the operator to the cottage front wall
        double yaw = player.getYRot();

        // cottage: 7 wide (u) x 5 deep (v) planks walls 4 high, glass windows, spruce roof. u is to the right, v away from the operator.
        for (int u = -3; u <= 3; u++) {
            for (int v = 0; v <= 4; v++) {
                boolean wall = u == -3 || u == 3 || v == 0 || v == 4;
                for (int y = 0; y < 4; y++) {
                    Block b = Blocks.AIR;
                    if (wall) {
                        b = Blocks.OAK_PLANKS;
                        boolean window = y == 2 && ((v == 0 || v == 4) && (u == -1 || u == 1)) || (y == 2 && (u == -3 || u == 3) && v == 2);
                        boolean door = v == 4 && u == 0 && y < 2;
                        if (window) b = Blocks.GLASS;
                        if (door) b = Blocks.AIR;
                    }
                    set(level, gy + y, feet, fx, fz, rx, rz, dist, u, v, b);
                }
                set(level, gy + 4, feet, fx, fz, rx, rz, dist, u, v, Blocks.SPRUCE_PLANKS);       // roof
            }
        }
        Vec3 target = new Vec3(feet.getX() + 0.5 + fx * dist + rx * 0.0, gy + 2.2, feet.getZ() + 0.5 + fz * dist);

        DroneEntity drone = KsRegistries.DRONE.get().create(level, EntitySpawnReason.COMMAND);
        if (drone == null) return;
        drone.snapTo(new Vec3(feet.getX() + 0.5 + fx * 2.5, gy + 0.1, feet.getZ() + 0.5 + fz * 2.5));
        drone.resetOrientation((float) yaw);
        drone.setWarhead(Warhead.IMPACT);
        level.addFreshEntity(drone);
        drone.startPilot(player);
        double tilt = 0.55 * P.maxAngleDeg();
        drone.debugScript(t -> {
            if (t < 12) return new FlightInput(0, 0, 0, 0, false, 1);
            Vec3d pos = drone.sim().position();
            double wantYaw = Math.toDegrees(Math.atan2(-(target.x - pos.x()), target.z - pos.z()));
            double err = ((wantYaw - drone.sim().orientation().minecraftYawDeg() + 540) % 360) - 180;
            double alt = altHold(drone, target.y, tilt);
            return new FlightInput(alt, 0, t < 30 ? 0 : 0.55, Math.max(-1, Math.min(1, err / 40.0)), true, 1);
        });
        KamikazeSkies.LOGGER.info("KS_DEMO started (strike)");
    }

    private static void set(ServerLevel level, int y, BlockPos feet, int fx, int fz, int rx, int rz, int dist, int u, int v, Block b) {
        level.setBlock(new BlockPos(feet.getX() + fx * (dist + v) + rx * u, y, feet.getZ() + fz * (dist + v) + rz * u), b.defaultBlockState(), 2);
    }

    /** Called by the blast executor: slow the game down so the collapse is easy to capture. */
    public static void onBlast(ServerLevel level) {
        if (!MODE.equalsIgnoreCase("strike") || slowMo) return;
        slowMo = true;
        slowMoStart = level.getGameTime();
        level.getServer().tickRateManager().setTickRate(4.0f);
        KamikazeSkies.LOGGER.info("KS_DEMO impact: slow motion");
    }

    /** Restore normal speed once the collapse has played out. */
    public static void tick(MinecraftServer server) {
        if (partsShowcase != null && partsPlayer != null) {
            long elapsed = server.overworld().getGameTime() - partsStart;
            if (elapsed > 0 && elapsed % 50 == 0) showPartsPage((int) ((elapsed / 50) % ((partsShowcase.size() + 8) / 9)));
        }
        if (!slowMo) return;
        long elapsed = server.overworld().getGameTime() - slowMoStart;
        if (elapsed > 30 && !CollapseManager.busy()) {
            server.tickRateManager().setTickRate(20.0f);
            slowMo = false;
            KamikazeSkies.LOGGER.info("KS_DEMO slow motion over");
        }
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
