package com.openfpv.server;

import com.openfpv.Config;
import com.openfpv.net.Net;
import com.openfpv.net.PacketControl;
import com.openfpv.net.PacketViewEnd;
import com.openfpv.oc.CameraEnvironment;
import com.openfpv.oc.CameraRegistry;
import java.util.HashMap;
import java.util.Iterator;
import java.util.Map;
import java.util.UUID;
import li.cil.oc.api.internal.Drone;
import net.minecraft.entity.Entity;
import net.minecraft.entity.player.EntityPlayerMP;
import net.minecraft.server.MinecraftServer;
import net.minecraft.util.math.Vec3d;
import net.minecraftforge.fml.common.FMLCommonHandler;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import net.minecraftforge.fml.common.gameevent.PlayerEvent;
import net.minecraftforge.fml.common.gameevent.TickEvent;

/**
 * Server-side authority for FPV sessions: who is piloting which drone, range/energy checks, and
 * translating pilot input into drone movement targets.
 */
public final class ViewerManager {
    private static final int CONTROL_TIMEOUT_TICKS = 10;

    private static final class Session {
        final int entityId;
        long lastControl;
        boolean hovering = true;

        Session(int entityId, long now) {
            this.entityId = entityId;
            this.lastControl = now;
        }
    }

    private static final Map<UUID, Session> SESSIONS = new HashMap<>();
    private static long ticks;

    public ViewerManager() {}

    public static void begin(EntityPlayerMP player, int entityId) {
        Entity e = player.world.getEntityByID(entityId);
        CameraEnvironment cam = CameraRegistry.get(entityId);
        if (e == null || e.isDead || !(e instanceof Drone) || cam == null || !cam.isActive()
            || player.getDistance(e) > Config.maxRange) {
            Net.sendViewEnd(player, PacketViewEnd.REFUSED);
            return;
        }
        SESSIONS.put(player.getUniqueID(), new Session(entityId, ticks));
    }

    /** Ends the player's session. A reason of -1 means the client asked for it (no reply needed). */
    public static void end(EntityPlayerMP player, int reason) {
        Session s = SESSIONS.remove(player.getUniqueID());
        if (s != null) {
            Entity e = player.world.getEntityByID(s.entityId);
            if (e instanceof Drone && !e.isDead) hold((Drone) e, e);
        }
        if (s != null && reason >= 0) Net.sendViewEnd(player, reason);
    }

    public static boolean isViewing(UUID player) {
        return SESSIONS.containsKey(player);
    }

    public static void control(EntityPlayerMP player, PacketControl msg) {
        Session s = SESSIONS.get(player.getUniqueID());
        if (s == null || s.entityId != msg.entityId) return;
        Entity e = player.world.getEntityByID(msg.entityId);
        if (!(e instanceof Drone) || e.isDead) return;
        double dx = msg.dx, dy = msg.dy, dz = msg.dz;
        if (Double.isNaN(dx) || Double.isNaN(dy) || Double.isNaN(dz)) return;
        double len = Math.sqrt(dx * dx + dy * dy + dz * dz);
        if (len > 1.0) {
            dx /= len; dy /= len; dz /= len;
            len = 1.0;
        }
        s.lastControl = ticks;
        if (len < 0.01) {
            if (!s.hovering) hold((Drone) e, e);
            s.hovering = true;
            return;
        }
        s.hovering = false;
        double lead = Config.flightLead;
        ((Drone) e).setTarget(new Vec3d(e.posX + dx * lead, e.posY + dy * lead, e.posZ + dz * lead));
    }

    private static void hold(Drone drone, Entity e) {
        drone.setTarget(new Vec3d(e.posX, e.posY, e.posZ));
    }

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
            } else if (player.getDistance(e) > Config.maxRange) {
                reason = PacketViewEnd.SIGNAL_LOST;
            }
            if (reason >= 0) {
                it.remove();
                if (e instanceof Drone && !e.isDead) hold((Drone) e, e);
                Net.sendViewEnd(player, reason);
            } else if (!s.hovering && ticks - s.lastControl > CONTROL_TIMEOUT_TICKS) {
                hold((Drone) e, e);
                s.hovering = true;
            }
        }
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
