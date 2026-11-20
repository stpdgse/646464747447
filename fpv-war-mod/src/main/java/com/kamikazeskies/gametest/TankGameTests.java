package com.kamikazeskies.gametest;

import java.util.function.Consumer;

import com.kamikazeskies.core.KamikazeSkies;
import com.kamikazeskies.core.KsRegistries;
import com.kamikazeskies.drone.TankEntity;

import net.minecraft.core.registries.Registries;
import net.minecraft.gametest.framework.GameTestHelper;
import net.minecraft.network.chat.Component;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.phys.Vec3;
import net.neoforged.bus.api.IEventBus;
import net.neoforged.neoforge.registries.DeferredRegister;

/** Tank entity tests: hit points, destruction, wreck lifetime, fire trigger. */
public final class TankGameTests {
    private static final DeferredRegister<Consumer<GameTestHelper>> FUNCTIONS = DeferredRegister.create(Registries.TEST_FUNCTION, KamikazeSkies.MODID);
    static {
        reg("tank_has_heavy_hit_points", TankGameTests::hitPoints);
        reg("tank_survives_small_arms_and_dies_to_a_big_hit", TankGameTests::damage);
        reg("tank_wreck_stays_and_cannot_fire", TankGameTests::wreck);
    }
    private static void reg(String name, Consumer<GameTestHelper> fn) { FUNCTIONS.register(name, () -> fn); }
    public static void register(IEventBus bus) { FUNCTIONS.register(bus); }

    private static TankEntity spawn(GameTestHelper h) {
        TankEntity t = KsRegistries.TANK.get().create(h.getLevel(), EntitySpawnReason.COMMAND);
        t.snapTo(h.absoluteVec(new Vec3(3, 2, 3)));
        h.getLevel().addFreshEntity(t);
        return t;
    }

    private static void hitPoints(GameTestHelper h) {
        TankEntity t = spawn(h);
        h.assertTrue(t.getMaxHealth() == TankEntity.MAX_HEALTH, Component.literal("max health " + t.getMaxHealth()));
        h.assertTrue(!t.isPushable(), Component.literal("a tank must not be pushable"));
        h.succeed();
    }

    private static void damage(GameTestHelper h) {
        TankEntity t = spawn(h);
        t.hurtServer(h.getLevel(), h.getLevel().damageSources().generic(), 6f);
        h.assertTrue(t.isAlive(), Component.literal("a rifle-sized hit killed the tank"));
        t.hurtServer(h.getLevel(), h.getLevel().damageSources().generic(), 500f);
        h.assertTrue(t.isDeadOrDying(), Component.literal("tank survived 500 damage"));
        h.succeed();
    }

    private static void wreck(GameTestHelper h) {
        TankEntity t = spawn(h);
        t.hurtServer(h.getLevel(), h.getLevel().damageSources().generic(), 500f);
        t.fireCannon();                                   // must be a harmless no-op on a wreck
        h.runAfterDelay(60, () -> {
            h.assertTrue(!t.isRemoved(), Component.literal("wreck vanished (deathTime=" + t.deathTime + ")"));
            h.succeed();
        });
    }
}
