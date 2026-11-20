package com.kamikazeskies.gametest;

import java.util.function.Consumer;

import com.kamikazeskies.core.KamikazeSkies;
import com.kamikazeskies.core.KsRegistries;
import com.kamikazeskies.drone.SoldierEntity;
import com.kamikazeskies.drone.TankEntity;
import com.kamikazeskies.mission.MissionManager;
import com.kamikazeskies.mission.MissionRegistry;
import com.kamikazeskies.mission.MissionState;

import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.Registries;
import net.minecraft.gametest.framework.GameTestHelper;
import net.minecraft.network.chat.Component;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.entity.Entity;
import net.minecraft.world.phys.AABB;
import net.neoforged.bus.api.IEventBus;
import net.neoforged.neoforge.registries.DeferredRegister;

/** Mission framework on the real server: the ground is built, the enemy spawns, kills count, rewards are paid, the operator's death fails the mission. */
public final class MissionGameTests {
    private static final DeferredRegister<Consumer<GameTestHelper>> FUNCTIONS = DeferredRegister.create(Registries.TEST_FUNCTION, KamikazeSkies.MODID);
    static {
        reg("mission_builds_ground_and_spawns_enemy", MissionGameTests::spawns);
        reg("mission_completes_and_pays_rewards", MissionGameTests::completes);
        reg("mission_fails_when_operator_dies", MissionGameTests::operatorDies);
    }
    private static void reg(String name, Consumer<GameTestHelper> fn) { FUNCTIONS.register(name, () -> fn); }
    public static void register(IEventBus bus) { FUNCTIONS.register(bus); }

    private static ServerPlayer begin(GameTestHelper h) {
        KsGameTests.keepAreaTicking(h);
        ServerPlayer p = h.makeMockServerPlayerInLevel();
        p.setGameMode(net.minecraft.world.level.GameType.SURVIVAL);
        BlockPos origin = h.absolutePos(new BlockPos(0, 2, 0));
        h.getLevel().setChunkForced(origin.getX() >> 4, (origin.getZ() + 60) >> 4, true);
        h.assertTrue(MissionManager.start(p, MissionRegistry.all().get("outpost_alpha"), h.getLevel(), origin, false), Component.literal("mission did not start"));
        return p;
    }

    private static java.util.List<Entity> enemies(GameTestHelper h) {
        BlockPos o = h.absolutePos(BlockPos.ZERO);
        return h.getLevel().getEntitiesOfClass(Entity.class, new AABB(o).inflate(150, 100, 150), e -> e.entityTags().contains(MissionManager.TAG_PREFIX + "outpost_alpha") && (e instanceof SoldierEntity || e instanceof TankEntity));
    }

    private static void spawns(GameTestHelper h) {
        ServerPlayer p = begin(h);
        h.assertTrue(MissionManager.stateOf(p) != null, Component.literal("no mission state"));
        h.succeedWhen(() -> {                  // entities become visible once their chunk is fully loaded
            var list = enemies(h);
            long soldiers = list.stream().filter(e -> e instanceof SoldierEntity).count(), tanks = list.stream().filter(e -> e instanceof TankEntity).count();
            h.assertTrue(soldiers == 4 && tanks == 1, Component.literal("spawned " + soldiers + " soldiers, " + tanks + " tanks"));
        });
    }

    private static void completes(GameTestHelper h) {
        ServerPlayer p = begin(h);
        int before = p.getInventory().countItem(KsRegistries.part(com.kamikazeskies.build.PartKind.BATTERY, 2));
        boolean[] killed = {false};
        h.succeedWhen(() -> {
            if (!killed[0]) {
                var list = enemies(h);
                h.assertTrue(list.size() == 5, Component.literal("waiting for the enemy to load: " + list.size()));
                for (Entity e : list) ((net.minecraft.world.entity.LivingEntity) e).hurtServer(h.getLevel(), h.getLevel().damageSources().generic(), 1000f);
                killed[0] = true;
            }
            h.assertTrue(MissionManager.stateOf(p) == null, Component.literal("mission still running"));          // finished missions are removed
            int after = p.getInventory().countItem(KsRegistries.part(com.kamikazeskies.build.PartKind.BATTERY, 2));
            h.assertTrue(after == before + 1, Component.literal("reward not paid: " + before + " -> " + after));
        });
    }

    private static void operatorDies(GameTestHelper h) {
        ServerPlayer p = begin(h);
        MissionState st = MissionManager.stateOf(p);
        net.neoforged.neoforge.common.NeoForge.EVENT_BUS.post(new net.neoforged.neoforge.event.entity.living.LivingDeathEvent(p, h.getLevel().damageSources().generic()));   // mock players cannot be killed
        h.succeedWhen(() -> {
            h.assertTrue(MissionManager.stateOf(p) == null, Component.literal("mission still running after the operator died: dying=" + p.isDeadOrDying() + " hp=" + p.getHealth() + " status=" + st.status()));
            h.assertTrue(st.status() == MissionState.Status.FAILED, Component.literal("status " + st.status()));
        });
    }
}
