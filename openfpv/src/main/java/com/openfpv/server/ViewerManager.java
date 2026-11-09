package com.openfpv.server;

import com.openfpv.Config;
import com.openfpv.net.Net;
import com.openfpv.net.PacketAttitude;
import com.openfpv.net.PacketControl;
import com.openfpv.net.PacketViewEnd;
import com.openfpv.oc.CameraEnvironment;
import com.openfpv.oc.CameraRegistry;
import com.openfpv.oc.DroneAccess;
import com.openfpv.util.CameraMath;
import com.openfpv.util.QuadPhysics;
import java.util.HashMap;
import java.util.Iterator;
import java.util.Map;
import java.util.UUID;
import li.cil.oc.api.internal.Drone;
import net.minecraft.entity.Entity;
import net.minecraft.entity.player.EntityPlayerMP;
import net.minecraft.server.MinecraftServer;
import net.minecraft.util.EnumFacing;
import net.minecraftforge.fml.common.FMLCommonHandler;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import net.minecraftforge.fml.common.gameevent.PlayerEvent;
import net.minecraftforge.fml.common.gameevent.TickEvent;

/**
 * Server-side authority for FPV sessions: who is piloting which drone, range checks, the
 * quadcopter flight model, and the long-range chunk/entity link.
 *
 * <h3>Flight model (angle mode)</h3>
 * The pilot's sticks command a bank/pitch angle; the drone's attitude follows with a first-order lag.
 * The tilted thrust vector gives a horizontal acceleration of {@code accelScale * sin(tilt)}, the
 * throttle stick gives a vertical one. Each tick the drone's movement target is set to
 * {@code position + acceleration}; OpenComputers' own drone physics then adds exactly that to the
 * velocity (it adds {@code min(accel, distance)} towards the target) and applies drag. Let go of
 * the sticks and the drone levels out and coasts to a stop.
 */
public final class ViewerManager {
    private static final int CONTROL_TIMEOUT_TICKS = 20;

    private static final class Session {
        final int entityId;
        long lastControl;
        float sPitch, sRoll, sYaw, sThrottle;
        float yaw, pitch, roll;
        float[] savedLimits;
        LongRangeLink link;

        Session(int entityId, long now) {
            this.entityId = entityId;
            this.lastControl = now;
        }
    }

    private static final Map<UUID, Session> SESSIONS = new HashMap<>();
    private static long ticks;

    public ViewerManager() {}

    // ------------------------------------------------------------------ session lifecycle

    public static void begin(EntityPlayerMP player, int entityId) {
        Entity e = player.world.getEntityByID(entityId);
        CameraEnvironment cam = CameraRegistry.get(entityId);
        if (e == null || e.isDead || !(e instanceof Drone) || cam == null || !cam.isActive()
            || player.getDistance(e) > cam.range() || pilotedByOther(player.getUniqueID(), entityId)) {
            Net.sendViewEnd(player, PacketViewEnd.REFUSED);
            return;
        }
        Session s = new Session(entityId, ticks);
        EnumFacing f = ((Drone) e).facing();
        s.yaw = f != null && f.getAxis().isHorizontal() ? f.getHorizontalAngle() : 0f;
        s.savedLimits = DroneAccess.limits(e);
        if (s.savedLimits != null) {
            DroneAccess.setLimits(e, (float) (s.savedLimits[0] * Config.speedMultiplier), (float) Config.fpvDrag);
        }
        s.link = new LongRangeLink(player, entityId);
        SESSIONS.put(player.getUniqueID(), s);
        Net.sendAttitude(player, new PacketAttitude(entityId, s.yaw, 0, 0));
    }

    private static boolean pilotedByOther(UUID me, int entityId) {
        for (Map.Entry<UUID, Session> en : SESSIONS.entrySet()) {
            if (en.getValue().entityId == entityId && !en.getKey().equals(me)) return true;
        }
        return false;
    }

    /** Ends the player's session. A reason of -1 means the client asked for it (no reply needed). */
    public static void end(EntityPlayerMP player, int reason) {
        Session s = SESSIONS.remove(player.getUniqueID());
        if (s == null) return;
        finish(player, s);
        if (reason >= 0) Net.sendViewEnd(player, reason);
    }

    private static void finish(EntityPlayerMP player, Session s) {
        Entity e = player.world.getEntityByID(s.entityId);
        if (e != null && !e.isDead) {
            if (s.savedLimits != null) DroneAccess.setLimits(e, s.savedLimits[0], s.savedLimits[1]);
            hold(e);
        }
        if (s.link != null) s.link.release();
    }

    private static void hold(Entity e) {
        DroneAccess.setTarget(e, e.posX, e.posY, e.posZ);
    }

    public static boolean isViewing(UUID player) {
        return SESSIONS.containsKey(player);
    }

    /** Test/diagnostic hook: {yaw, pitch, roll} of the player's session, or null. */
    public static float[] attitude(UUID player) {
        Session s = SESSIONS.get(player);
        return s == null ? null : new float[]{s.yaw, s.pitch, s.roll};
    }

    /** Test/diagnostic hook: {watched chunks, boosted entities} of the player's long-range link. */
    public static int[] linkStats(UUID player) {
        Session s = SESSIONS.get(player);
        return s == null || s.link == null ? null : new int[]{s.link.watchedChunks(), s.link.boostedEntities()};
    }

    public static void control(EntityPlayerMP player, PacketControl msg) {
        Session s = SESSIONS.get(player.getUniqueID());
        if (s == null || s.entityId != msg.entityId) return;
        s.sPitch = stick(msg.pitch);
        s.sRoll = stick(msg.roll);
        s.sYaw = stick(msg.yaw);
        s.sThrottle = stick(msg.throttle);
        s.lastControl = ticks;
    }

    private static float stick(float v) {
        return Float.isNaN(v) ? 0f : CameraMath.clamp(v, -1f, 1f);
    }

    // ------------------------------------------------------------------ per-tick simulation

    @SubscribeEvent
    public void onServerTick(TickEvent.ServerTickEvent event) {
        if (event.phase != TickEvent.Phase.END) return;
        ticks++;
        if (SESSIONS.isEmpty()) return;
        MinecraftServer server = FMLCommonHandler.instance().getMinecraftServerInstance();
        if (server == null) return;
        Iterator<Map.Entry<UUID, Session>> it = SESSIONS.entrySet().iterator();
        while (it.hasNext()) {
            Map.Entry<UUID, Session> entry = it.next();
            Session s = entry.getValue();
            EntityPlayerMP player = server.getPlayerList().getPlayerByUUID(entry.getKey());
            if (player == null) {
                it.remove();
                continue;
            }
            Entity e = player.world.getEntityByID(s.entityId);
            CameraEnvironment cam = CameraRegistry.get(s.entityId);
            int reason = -1;
            if (player.isDead) {
                reason = PacketViewEnd.REFUSED;
            } else if (e == null || e.isDead || !(e instanceof Drone)) {
                reason = PacketViewEnd.DRONE_GONE;
            } else if (cam == null || !cam.isActive()) {
                reason = PacketViewEnd.CAMERA_OFF;
            } else if (player.getDistance(e) > cam.range()) {
                reason = PacketViewEnd.SIGNAL_LOST;
            }
            if (reason >= 0) {
                it.remove();
                finish(player, s);
                Net.sendViewEnd(player, reason);
                continue;
            }
            fly(player, s, e);
            s.link.tick(e);
        }
    }

    private static void fly(EntityPlayerMP player, Session s, Entity e) {
        if (ticks - s.lastControl > CONTROL_TIMEOUT_TICKS) {
            s.sPitch = s.sRoll = s.sYaw = s.sThrottle = 0f;   // link dead: level out and hover
        }
        float tilt = (float) Config.maxTilt;
        float response = (float) Config.attitudeResponse;
        s.pitch = QuadPhysics.approach(s.pitch, s.sPitch * tilt, response);
        s.roll = QuadPhysics.approach(s.roll, s.sRoll * tilt, response);
        s.yaw = QuadPhysics.wrapDegrees(s.yaw + s.sYaw * (float) Config.yawRate);

        double[] a = QuadPhysics.acceleration(s.yaw, s.pitch, s.roll, s.sThrottle, Config.accelScale, Config.throttleAccel);
        a = QuadPhysics.clampLength(a, 0.095);   // OC drones cap their acceleration at 0.1 blocks/tick^2
        DroneAccess.setTarget(e, e.posX + a[0], e.posY + a[1], e.posZ + a[2]);
        Net.sendAttitude(player, new PacketAttitude(s.entityId, s.yaw, s.pitch, s.roll));
    }

    @SubscribeEvent
    public void onLogout(PlayerEvent.PlayerLoggedOutEvent event) {
        if (event.player instanceof EntityPlayerMP) {
            end((EntityPlayerMP) event.player, -1);
        }
    }

    public static void reset() {
        SESSIONS.clear();
    }
}
