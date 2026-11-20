package com.kamikazeskies.mission;

import java.util.HashMap;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import java.util.UUID;

import org.jspecify.annotations.Nullable;

import com.kamikazeskies.ai.Role;
import com.kamikazeskies.build.DroneBuild;
import com.kamikazeskies.core.KamikazeSkies;
import com.kamikazeskies.core.KsRegistries;
import com.kamikazeskies.drone.SoldierEntity;
import com.kamikazeskies.drone.TankEntity;
import com.kamikazeskies.drone.Warhead;

import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.BuiltInRegistries;
import net.minecraft.core.registries.Registries;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.Identifier;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.MinecraftServer;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.entity.LivingEntity;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.level.Level;
import net.neoforged.neoforge.common.NeoForge;
import net.neoforged.neoforge.event.entity.living.LivingDeathEvent;
import net.neoforged.neoforge.event.tick.ServerTickEvent;

/** Runs missions: builds the ground, spawns the enemy, counts kills, pays rewards. One mission per player. */
public final class MissionManager {
    public static final ResourceKey<Level> WARZONE = ResourceKey.create(Registries.DIMENSION, Identifier.fromNamespaceAndPath(KamikazeSkies.MODID, "warzone"));
    private static final double RADIUS = 140;
    private static final Map<UUID, Active> ACTIVE = new HashMap<>();
    private static int instances;

    private record Active(ServerPlayer player, ServerLevel level, BlockPos origin, MissionState state, java.util.List<net.minecraft.world.level.ChunkPos> forced) {}

    private static java.util.List<net.minecraft.world.level.ChunkPos> forceLoadPad(ServerLevel level, BlockPos origin) {
        java.util.List<net.minecraft.world.level.ChunkPos> list = new java.util.ArrayList<>();
        for (int cx = (origin.getX() - 28) >> 4; cx <= (origin.getX() + 32) >> 4; cx++)
            for (int cz = (origin.getZ() - 6) >> 4; cz <= (origin.getZ() + 84) >> 4; cz++) { level.setChunkForced(cx, cz, true); list.add(new net.minecraft.world.level.ChunkPos(cx, cz)); }
        return list;
    }

    private static void release(Active a) { for (var c : a.forced) a.level.setChunkForced(c.x(), c.z(), false); }

    private MissionManager() {}

    public static void init() {
        NeoForge.EVENT_BUS.addListener((LivingDeathEvent e) -> onDeath(e.getEntity()));
        NeoForge.EVENT_BUS.addListener((ServerTickEvent.Post e) -> tick(e.getServer()));
        NeoForge.EVENT_BUS.addListener((net.neoforged.neoforge.event.entity.player.PlayerEvent.PlayerLoggedOutEvent e) -> ACTIVE.remove(e.getEntity().getUUID()));
    }

    public static @Nullable MissionState stateOf(ServerPlayer p) { Active a = ACTIVE.get(p.getUUID()); return a == null ? null : a.state; }

    /** Start in the warzone dimension when it exists, otherwise where the player stands (used by tests). */
    public static boolean start(ServerPlayer player, MissionDef def) {
        MinecraftServer server = player.level().getServer();
        ServerLevel warzone = server == null ? null : server.getLevel(WARZONE);
        ServerLevel level = warzone != null ? warzone : (ServerLevel) player.level();
        BlockPos origin = warzone != null ? new BlockPos(512 * (++instances), 0, 0) : player.blockPosition();
        return start(player, def, level, origin, warzone != null);
    }

    /** @param teleport move the player to the launch point (the origin) */
    public static boolean start(ServerPlayer player, MissionDef def, ServerLevel level, BlockPos origin, boolean teleport) {
        if (ACTIVE.containsKey(player.getUUID())) return false;
        var forced = forceLoadPad(level, origin);          // the enemy must stay loaded and ticking while the mission runs
        int groundY = Outpost.build(level, origin);
        BlockPos ground = new BlockPos(origin.getX(), groundY, origin.getZ());
        RandomSourceHolder rnd = new RandomSourceHolder(def.id().hashCode() ^ origin.hashCode());
        for (MissionDef.Spawn s : def.spawns()) for (int i = 0; i < s.count(); i++) spawn(level, def.id(), s, ground, rnd);
        if (teleport) player.teleportTo(level, origin.getX() + 0.5, groundY + 1, origin.getZ() + 0.5, Set.of(), 0f, 5f, true);
        ItemStack kit = KsRegistries.droneStack(DroneBuild.STANDARD.withWarhead(Warhead.IMPACT));
        if (player.getInventory().countItem(KsRegistries.DRONE_ITEM.get()) == 0 && !player.getInventory().add(kit)) player.spawnAtLocation(level, kit);
        MissionState state = new MissionState(def);
        ACTIVE.put(player.getUUID(), new Active(player, level, ground, state, forced));
        player.sendSystemMessage(Component.literal("MISSION: " + def.title()));
        player.sendSystemMessage(Component.literal(def.briefing()));
        for (MissionDef.Objective o : def.objectives()) player.sendSystemMessage(Component.literal(" - " + o.text() + " (" + o.count() + ")"));
        return true;
    }

    private record RandomSourceHolder(java.util.Random r) { RandomSourceHolder(long seed) { this(new java.util.Random(seed)); } }

    public static final String TAG_PREFIX = "ks_mission:";

    private static void spawn(ServerLevel level, String missionId, MissionDef.Spawn s, BlockPos ground, RandomSourceHolder rnd) {
        int x = ground.getX() + s.dx() + (s.radius() > 0 ? rnd.r.nextInt(2 * s.radius() + 1) - s.radius() : 0);
        int z = ground.getZ() + s.dz() + (s.radius() > 0 ? rnd.r.nextInt(2 * s.radius() + 1) - s.radius() : 0);
        Entity e = switch (s.entity()) {
            case "soldier" -> {
                SoldierEntity sol = KsRegistries.SOLDIER.get().create(level, EntitySpawnReason.COMMAND);
                if (sol != null && !s.role().isEmpty()) sol.setRole(Role.valueOf(s.role().toUpperCase(Locale.ROOT)));
                yield sol;
            }
            case "tank" -> KsRegistries.TANK.get().create(level, EntitySpawnReason.COMMAND);
            default -> throw new IllegalArgumentException("unknown mission entity " + s.entity());
        };
        if (e == null) return;
        e.snapTo(x + 0.5, ground.getY() + 1, z + 0.5, 180f, 0f);        // facing the operator (who is at -z)
        if (e instanceof net.minecraft.world.entity.Mob m) m.setPersistenceRequired();
        e.addTag(TAG_PREFIX + missionId);
        level.addFreshEntity(e);
    }

    public static void abort(ServerPlayer p) {
        Active a = ACTIVE.remove(p.getUUID());
        if (a != null) release(a);
        if (a != null) p.sendSystemMessage(Component.literal("Mission aborted."));
    }

    private static void onDeath(LivingEntity dead) {
        if (dead.level().isClientSide()) return;
        if (dead instanceof ServerPlayer p) {
            Active a = ACTIVE.get(p.getUUID());
            if (a != null) { a.state.onOperatorDied(); finish(p, a); }
            return;
        }
        String kind = dead instanceof SoldierEntity ? "soldier" : dead instanceof TankEntity ? "tank" : null;
        if (kind == null) return;
        for (Map.Entry<UUID, Active> en : Set.copyOf(ACTIVE.entrySet())) {
            Active a = en.getValue();
            if (a.level != dead.level() || dead.position().distanceTo(net.minecraft.world.phys.Vec3.atCenterOf(a.origin)) > RADIUS) continue;
            ServerPlayer p = a.player;
            if (!a.state.onKill(kind)) continue;
            if (a.state.status() == MissionState.Status.ACTIVE) p.sendOverlayMessage(Component.literal(progressLine(a.state)));
            else finish(p, a);
        }
    }

    private static String progressLine(MissionState s) {
        StringBuilder b = new StringBuilder();
        for (int i = 0; i < s.def().objectives().size(); i++) { var o = s.def().objectives().get(i); b.append(o.text()).append(' ').append(s.progress(i)).append('/').append(o.count()).append(i + 1 < s.def().objectives().size() ? "   " : ""); }
        return b.toString();
    }

    private static void tick(MinecraftServer server) {
        if (ACTIVE.isEmpty()) return;
        for (Map.Entry<UUID, Active> en : Set.copyOf(ACTIVE.entrySet())) {
            Active a = en.getValue();
            a.state.tick();
            ServerPlayer p = a.player;
            if (p.isRemoved() && !p.isDeadOrDying()) { release(ACTIVE.remove(en.getKey())); continue; }
            if (a.state.status() == MissionState.Status.FAILED) finish(p, a);
        }
    }

    private static void finish(ServerPlayer p, Active a) {
        if (!ACTIVE.remove(p.getUUID(), a)) return;
        release(a);
        MissionState s = a.state;
        if (s.status() == MissionState.Status.COMPLETE) {
            p.sendSystemMessage(Component.literal("MISSION COMPLETE: " + s.def().title() + " (" + s.elapsedTicks() / 20 + " s)"));
            for (MissionDef.Bonus b : s.earnedBonus()) p.sendSystemMessage(Component.literal("Bonus: " + b.text()));
            for (MissionDef.Reward r : s.def().rewards()) {
                Item item = BuiltInRegistries.ITEM.getValue(Identifier.parse(r.item()));
                ItemStack st = new ItemStack(item, r.count());
                if (!p.getInventory().add(st)) p.spawnAtLocation((ServerLevel) p.level(), st);
                p.sendSystemMessage(Component.literal("Reward: " + r.count() + " x " + st.getHoverName().getString()));
            }
        } else {
            p.sendSystemMessage(Component.literal("MISSION FAILED: " + s.failReason()));
        }
    }
}
