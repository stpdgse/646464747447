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
 *   <li>{@code soldiers}: a squad of riflemen stands ahead and plays every animation in turn: idle, walking, aiming and firing, reloading,
 *       crouching, panicking and fleeing, then a death from the front and one from behind.</li>
 *   <li>{@code mission}: starts Outpost Alpha in the warzone dimension; the camera rides an armed drone that cruises toward the tank at 3.5 m.</li>
 *   <li>{@code ai}: a mixed squad (conscripts, riflemen, a veteran) stands in a field; an armed drone dives on them from behind the player.
 *       Watch who runs, who ducks, who shoots back, and who is shot out of the sky.</li>
 *   <li>{@code tank}: a tank scans with its turret, fires twice, drives off, then is destroyed (turret thrown, wreck texture).</li>
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
        if (MODE.equalsIgnoreCase("mission")) { startMission(player); return; }
        if (MODE.equalsIgnoreCase("ai")) { startAi(player, level); return; }
        if (MODE.equalsIgnoreCase("tank")) { startTank(player, level); return; }
        if (MODE.equalsIgnoreCase("soldiers")) { startSoldiers(player, level); return; }
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

    // ---- mission demo ---------------------------------------------------------------------------------
    private static void startMission(ServerPlayer player) {
        com.kamikazeskies.mission.MissionManager.start(player, com.kamikazeskies.mission.MissionRegistry.all().get("outpost_alpha"));
        ServerLevel level = (ServerLevel) player.level();
        DroneEntity drone = KsRegistries.DRONE.get().create(level, EntitySpawnReason.COMMAND);
        drone.snapTo(player.getX(), player.getY() + 0.1, player.getZ() + 1.5);
        drone.resetOrientation(0f);
        level.addFreshEntity(drone);
        drone.startPilot(player);
        double gy = player.getY();
        drone.debugScript(t -> t < 20 ? new FlightInput(0, 0, 0, 0, false, 1) : new FlightInput(altHold(drone, gy + 3.5, 0.3 * P.maxAngleDeg()), 0, t < 40 ? 0 : 0.3, 0, true, 1));
        KamikazeSkies.LOGGER.info("KS_DEMO started (mission)");
    }

    // ---- ai demo --------------------------------------------------------------------------------------
    private static void startAi(ServerPlayer player, ServerLevel level) {
        Vec3 look = player.getLookAngle();
        Vec3 flat = new Vec3(look.x, 0, look.z).normalize();
        Vec3 right = new Vec3(-flat.z, 0, flat.x);
        com.kamikazeskies.ai.Role[] roles = {com.kamikazeskies.ai.Role.CONSCRIPT, com.kamikazeskies.ai.Role.RIFLEMAN, com.kamikazeskies.ai.Role.VETERAN,
                com.kamikazeskies.ai.Role.CONSCRIPT, com.kamikazeskies.ai.Role.RIFLEMAN, com.kamikazeskies.ai.Role.CONSCRIPT};
        for (int i = 0; i < roles.length; i++) {
            SoldierEntity s = KsRegistries.SOLDIER.get().create(level, EntitySpawnReason.COMMAND);
            Vec3 pos = player.position().add(flat.scale(16)).add(right.scale((i - 2.5) * 2.2));
            s.setRole(roles[i]);
            s.snapTo(pos.x, player.getY(), pos.z, player.getYRot() + 180f, 0f);
            s.setPersistenceRequired();
            level.addFreshEntity(s);
        }
        DroneEntity drone = KsRegistries.DRONE.get().create(level, EntitySpawnReason.COMMAND);
        drone.snapTo(player.position().subtract(flat.scale(4)).add(0, 0.2, 0));
        drone.resetOrientation(player.getYRot());
        level.addFreshEntity(drone);
        double gy = player.getY();
        drone.debugScript(t -> t < 20 ? new FlightInput(0, 0, 0, 0, false, 1) : new FlightInput(altHold(drone, gy + 3.5, 0.3 * P.maxAngleDeg()), 0, t < 40 ? 0 : 0.3, 0, true, 1));
        KamikazeSkies.LOGGER.info("KS_DEMO started (ai)");
    }

    // ---- tank demo ------------------------------------------------------------------------------------
    private static TankEntity demoTank;
    private static ServerPlayer tankPlayer;
    private static long tankStart;

    private static void startTank(ServerPlayer player, ServerLevel level) {
        Vec3 look = player.getLookAngle();
        Vec3 flat = new Vec3(look.x, 0, look.z).normalize();
        demoTank = KsRegistries.TANK.get().create(level, EntitySpawnReason.COMMAND);
        Vec3 pos = player.position().add(flat.scale(10));
        float yaw = player.getYRot() + 125f;
        demoTank.snapTo(pos.x, player.getY(), pos.z, yaw, 0f);
        demoTank.setYBodyRot(yaw); demoTank.setYHeadRot(yaw);
        demoTank.setPersistenceRequired();
        demoTank.setScanning(true);
        level.addFreshEntity(demoTank);
        tankPlayer = player;
        tankStart = level.getGameTime();
        KamikazeSkies.LOGGER.info("KS_DEMO started (tank)");
    }

    private static void tickTank(ServerLevel level) {
        long t = level.getGameTime() - tankStart;
        if (t == 110) demoTank.setScanning(false);
        if (t == 140 || t == 200) demoTank.fireCannon();
        if (t >= 250 && t < 330) demoTank.setZza(1.0f);
        if (t == 520 && !demoTank.isDeadOrDying()) demoTank.hurtServer(level, level.damageSources().generic(), 500f);
    }

    // ---- soldiers demo --------------------------------------------------------------------------------
    private static final java.util.List<SoldierEntity> squad = new java.util.ArrayList<>();
    private static ServerPlayer squadPlayer;
    private static long squadStart;

    private static void startSoldiers(ServerPlayer player, ServerLevel level) {
        Vec3 look = player.getLookAngle();
        Vec3 flat = new Vec3(look.x, 0, look.z).normalize();
        Vec3 right = new Vec3(-flat.z, 0, flat.x);
        float yaw = player.getYRot() + 180f;
        squad.clear();
        for (int i = 0; i < 7; i++) {
            SoldierEntity s = KsRegistries.SOLDIER.get().create(level, EntitySpawnReason.COMMAND);
            Vec3 pos = i < 6 ? player.position().add(flat.scale(6.5)).add(right.scale((i - 2.5) * 1.35)) : player.position().add(flat.scale(10)).add(right.scale(2.5 * 1.35));
            s.snapTo(pos.x, player.getY(), pos.z, yaw, 0f);
            s.setYHeadRot(yaw); s.setYBodyRot(yaw);
            s.setPersistenceRequired();
            level.addFreshEntity(s);
            squad.add(s);
        }
        squad.get(2).setStance(SoldierAnim.Stance.AIM);
        squad.get(4).setStance(SoldierAnim.Stance.CROUCH);
        squad.get(5).setStance(SoldierAnim.Stance.PANIC);
        squad.get(6).setStance(SoldierAnim.Stance.AIM);
        squadPlayer = player;
        squadStart = level.getGameTime();
        KamikazeSkies.LOGGER.info("KS_DEMO started (soldiers)");
    }

    private static void tickSoldiers(ServerLevel level) {
        long t = level.getGameTime() - squadStart;
        SoldierEntity walker = squad.get(1);
        walker.setZza(1.0f);
        if (t % 70 == 35) { float y = walker.getYRot() + 180f; walker.setYRot(y); walker.setYHeadRot(y); walker.setYBodyRot(y); }
        if (t % 24 == 12) squad.get(2).fireShot();
        SoldierEntity reloader = squad.get(3);
        if (t % 80 == 0) reloader.setStance(SoldierAnim.Stance.STAND);
        if (t % 80 == 2) reloader.setStance(SoldierAnim.Stance.RELOAD);
        SoldierEntity runner = squad.get(5);
        if (t >= 120 && t < 170) { runner.setZza(1.0f); runner.setYRot(squadPlayer.getYRot()); runner.setYBodyRot(squadPlayer.getYRot()); } // flee away from the camera
        else if (t >= 170 && !runner.isDeadOrDying()) { runner.setYRot(squadPlayer.getYRot() + 180f); runner.setYBodyRot(squadPlayer.getYRot() + 180f); }
        if (t == 300 && !squad.get(4).isDeadOrDying()) squad.get(4).hurtServer(level, level.damageSources().playerAttack(squadPlayer), 100f);
        if (t == 330) squad.get(6).fireShot();
        if (t == 332 && !runner.isDeadOrDying()) runner.hurtServer(level, level.damageSources().mobAttack(squad.get(6)), 100f);
        if (t == 340 && !squad.get(0).isDeadOrDying()) squad.get(0).hurtServer(level, level.damageSources().generic(), 3f);   // a flinch
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
        if (demoTank != null && tankPlayer != null && tankPlayer.level() instanceof ServerLevel tl) tickTank(tl);
        if (!squad.isEmpty() && squadPlayer != null && squadPlayer.level() instanceof ServerLevel sl) tickSoldiers(sl);
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
