package com.openfpv.server;

import com.openfpv.Config;
import com.openfpv.OpenFPV;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.util.HashMap;
import java.util.HashSet;
import java.util.Iterator;
import java.util.Map;
import java.util.Set;
import net.minecraft.entity.Entity;
import net.minecraft.entity.EntityTracker;
import net.minecraft.entity.EntityTrackerEntry;
import net.minecraft.entity.player.EntityPlayer;
import net.minecraft.entity.player.EntityPlayerMP;
import net.minecraft.server.management.PlayerChunkMap;
import net.minecraft.server.management.PlayerChunkMapEntry;
import net.minecraft.util.IntHashMap;
import net.minecraft.util.math.AxisAlignedBB;
import net.minecraft.world.WorldServer;
import net.minecraftforge.fml.relauncher.ReflectionHelper;

/**
 * Lets a pilot see a drone that is far outside their normal loading/tracking range.
 *
 * Minecraft only sends a player the chunks around <em>themselves</em> and the entities within ~80
 * blocks of themselves. While a session is active this class
 * <ol>
 * <li>makes the pilot a "watcher" of the chunks around the drone (so the server streams them), and</li>
 * <li>raises the tracking range of the drone and of the entities near it, so the pilot's client is
 *     told about them.</li>
 * </ol>
 * Everything is undone in {@link #release()}.
 */
public final class LongRangeLink {
    private static boolean reflected;
    private static boolean ok;
    private static Method getOrCreateEntry;
    private static Field trackerTable;
    private static Field entryRange, entryMaxRange;

    private final EntityPlayerMP player;
    private final int droneId;
    private final Set<Long> watched = new HashSet<>();
    /** entity id -> {original range, original maxRange} for entries whose tracking range we raised. */
    private final Map<Integer, int[]> boosted = new HashMap<>();
    private long age;

    public LongRangeLink(EntityPlayerMP player, int droneId) {
        this.player = player;
        this.droneId = droneId;
        reflect();
    }

    private static synchronized void reflect() {
        if (reflected) return;
        reflected = true;
        try {
            getOrCreateEntry = ReflectionHelper.findMethod(PlayerChunkMap.class, "getOrCreateEntry", "func_187302_c",
                int.class, int.class);
            trackerTable = ReflectionHelper.findField(EntityTracker.class, "trackedEntityHashTable", "field_72794_c");
            entryRange = ReflectionHelper.findField(EntityTrackerEntry.class, "range", "field_73130_b");
            entryMaxRange = ReflectionHelper.findField(EntityTrackerEntry.class, "maxRange", "field_187262_f");
            ok = true;
        } catch (Throwable t) {
            OpenFPV.log.error("long-range link disabled: could not reach the chunk map / entity tracker internals", t);
        }
    }

    public static boolean isAvailable() {
        reflect();
        return ok;
    }

    private static long key(int cx, int cz) {
        return ((long) cx << 32) ^ (cz & 0xFFFFFFFFL);
    }

    private static int keyX(long k) {
        return (int) (k >> 32);
    }

    private static int keyZ(long k) {
        return (int) k;
    }

    /** Call every server tick; does its work every few ticks. */
    public void tick(Entity drone) {
        if (!ok || drone == null) return;
        if (age++ % 5 != 0) return;
        try {
            if (!(drone.world instanceof WorldServer) || player.world != drone.world) return;
            WorldServer world = (WorldServer) drone.world;
            streamChunks(world, drone);
            if (Config.boostEntities) boostEntities(world, drone);
        } catch (Throwable t) {
            OpenFPV.log.warn("long-range link update failed: {}", t.toString());
        }
    }

    private int viewDistance(WorldServer world) {
        return Math.max(2, world.getMinecraftServer().getPlayerList().getViewDistance());
    }

    private boolean insideOwnView(int cx, int cz, int pad) {
        int pcx = ((int) Math.floor(player.posX)) >> 4;
        int pcz = ((int) Math.floor(player.posZ)) >> 4;
        int vd = viewDistance((WorldServer) player.world) + pad;
        return Math.abs(cx - pcx) <= vd && Math.abs(cz - pcz) <= vd;
    }

    private void streamChunks(WorldServer world, Entity drone) throws Exception {
        PlayerChunkMap map = world.getPlayerChunkMap();
        int dcx = ((int) Math.floor(drone.posX)) >> 4;
        int dcz = ((int) Math.floor(drone.posZ)) >> 4;
        int radius = Math.min(Config.streamRadius, viewDistance(world));
        Set<Long> desired = new HashSet<>();
        for (int dx = -radius; dx <= radius; dx++) {
            for (int dz = -radius; dz <= radius; dz++) {
                int cx = dcx + dx, cz = dcz + dz;
                if (insideOwnView(cx, cz, 0)) continue;       // vanilla already sends these
                desired.add(key(cx, cz));
                PlayerChunkMapEntry entry = (PlayerChunkMapEntry) getOrCreateEntry.invoke(map, cx, cz);
                if (entry != null && !entry.containsPlayer(player)) entry.addPlayer(player);
            }
        }
        Iterator<Long> it = watched.iterator();
        while (it.hasNext()) {
            long k = it.next();
            if (desired.contains(k)) continue;
            it.remove();
            unwatch(map, k);
        }
        watched.addAll(desired);
    }

    private void unwatch(PlayerChunkMap map, long k) {
        int cx = keyX(k), cz = keyZ(k);
        if (insideOwnView(cx, cz, 1)) return;               // the pilot's own view owns it now
        PlayerChunkMapEntry entry = map.getEntry(cx, cz);
        if (entry != null && entry.containsPlayer(player)) entry.removePlayer(player);
    }

    @SuppressWarnings("unchecked")
    private EntityTrackerEntry entryFor(WorldServer world, int entityId) throws Exception {
        IntHashMap<EntityTrackerEntry> table = (IntHashMap<EntityTrackerEntry>) trackerTable.get(world.getEntityTracker());
        return table.lookup(entityId);
    }

    private void boost(WorldServer world, int entityId) throws Exception {
        EntityTrackerEntry entry = entryFor(world, entityId);
        if (entry == null) return;
        if (!boosted.containsKey(entityId)) {
            boosted.put(entityId, new int[]{entryRange.getInt(entry), entryMaxRange.getInt(entry)});
        }
        entryRange.setInt(entry, 8192);
        entryMaxRange.setInt(entry, 8192);
        // The tracker only re-checks who can see an entity when that entity moves more than 4 blocks, so a
        // hovering drone or a standing pig would never be sent. Ask it to re-evaluate this pilot now.
        entry.updatePlayerEntity(player);
    }

    private void restore(WorldServer world, int entityId, int[] original) {
        try {
            EntityTrackerEntry entry = entryFor(world, entityId);
            if (entry == null) return;
            entryRange.setInt(entry, original[0]);
            entryMaxRange.setInt(entry, original[1]);
            entry.updatePlayerEntity(player);   // far entities are now out of range: stop tracking them for the pilot
        } catch (Exception ignored) {
            // the entity is gone; nothing to restore
        }
    }

    private void boostEntities(WorldServer world, Entity drone) throws Exception {
        double r = Math.min(Config.boostRadius, Config.streamRadius * 16.0);
        AxisAlignedBB box = new AxisAlignedBB(drone.posX - r, 0, drone.posZ - r, drone.posX + r, 256, drone.posZ + r);
        Set<Integer> now = new HashSet<>();
        boost(world, droneId);                      // the drone itself is always boosted
        now.add(droneId);
        for (Entity e : world.getEntitiesWithinAABB(Entity.class, box)) {
            if (e instanceof EntityPlayer || e.isDead || e.getEntityId() == droneId) continue;
            now.add(e.getEntityId());
            boost(world, e.getEntityId());
        }
        Iterator<Map.Entry<Integer, int[]>> it = boosted.entrySet().iterator();
        while (it.hasNext()) {
            Map.Entry<Integer, int[]> en = it.next();
            if (!now.contains(en.getKey())) {
                restore(world, en.getKey(), en.getValue());
                it.remove();
            }
        }
    }

    /** Undoes everything: tracking ranges go back to normal and the extra chunks are released. */
    public void release() {
        if (!ok) return;
        try {
            if (player.world instanceof WorldServer) {
                WorldServer world = (WorldServer) player.world;
                for (Map.Entry<Integer, int[]> en : boosted.entrySet()) restore(world, en.getKey(), en.getValue());
                PlayerChunkMap map = world.getPlayerChunkMap();
                for (long k : watched) unwatch(map, k);
            }
        } catch (Throwable t) {
            OpenFPV.log.warn("long-range link release failed: {}", t.toString());
        }
        boosted.clear();
        watched.clear();
    }

    public int watchedChunks() {
        return watched.size();
    }

    public int boostedEntities() {
        return boosted.size();
    }
}
