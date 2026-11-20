package com.kamikazeskies.gametest;

import java.util.function.Consumer;

import com.kamikazeskies.core.KamikazeSkies;
import com.kamikazeskies.core.KsRegistries;
import com.kamikazeskies.drone.SoldierAnim;
import com.kamikazeskies.drone.SoldierEntity;

import net.minecraft.core.registries.Registries;
import net.minecraft.gametest.framework.GameTestHelper;
import net.minecraft.network.chat.Component;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.phys.Vec3;
import net.neoforged.bus.api.IEventBus;
import net.neoforged.neoforge.registries.DeferredRegister;

/** Rifleman entity tests: spawn, stance persistence, damage, corpse lifetime and death direction. */
public final class SoldierGameTests {
    private static final DeferredRegister<Consumer<GameTestHelper>> FUNCTIONS =
            DeferredRegister.create(Registries.TEST_FUNCTION, KamikazeSkies.MODID);

    static {
        reg("soldier_spawns_with_attributes", SoldierGameTests::spawns);
        reg("soldier_stance_is_saved", SoldierGameTests::stanceSaved);
        reg("soldier_survives_light_hit_and_dies_to_heavy", SoldierGameTests::damage);
        reg("soldier_corpse_stays_after_death_animation", SoldierGameTests::corpseStays);
    }
    private static void reg(String name, Consumer<GameTestHelper> fn) { FUNCTIONS.register(name, () -> fn); }
    public static void register(IEventBus bus) { FUNCTIONS.register(bus); }

    private static SoldierEntity spawn(GameTestHelper h) {
        SoldierEntity s = KsRegistries.SOLDIER.get().create(h.getLevel(), EntitySpawnReason.COMMAND);
        s.snapTo(h.absoluteVec(new Vec3(1.5, 2, 1.5)));
        h.getLevel().addFreshEntity(s);
        return s;
    }

    private static void spawns(GameTestHelper h) {
        SoldierEntity s = spawn(h);
        h.assertTrue(s.getMaxHealth() == 20.0f, Component.literal("max health " + s.getMaxHealth()));
        h.assertTrue(s.stance() == SoldierAnim.Stance.STAND, Component.literal("default stance " + s.stance()));
        h.succeed();
    }

    private static void stanceSaved(GameTestHelper h) {
        SoldierEntity s = spawn(h);
        s.setStance(SoldierAnim.Stance.CROUCH);
        var tag = net.minecraft.world.level.storage.TagValueOutput.createWithContext(net.minecraft.util.ProblemReporter.DISCARDING, h.getLevel().registryAccess());
        s.saveWithoutId(tag);
        SoldierEntity copy = KsRegistries.SOLDIER.get().create(h.getLevel(), EntitySpawnReason.COMMAND);
        copy.load(net.minecraft.world.level.storage.TagValueInput.create(net.minecraft.util.ProblemReporter.DISCARDING, h.getLevel().registryAccess(), tag.buildResult()));
        h.assertTrue(copy.stance() == SoldierAnim.Stance.CROUCH, Component.literal("stance after reload: " + copy.stance()));
        h.succeed();
    }

    private static void damage(GameTestHelper h) {
        SoldierEntity s = spawn(h);
        s.hurtServer(h.getLevel(), h.getLevel().damageSources().generic(), 5f);
        h.assertTrue(s.isAlive() && Math.abs(s.getHealth() - 15f) < 0.01f, Component.literal("health after light hit " + s.getHealth()));
        s.hurtServer(h.getLevel(), h.getLevel().damageSources().generic(), 30f);
        h.assertTrue(s.isDeadOrDying(), Component.literal("still alive with health " + s.getHealth()));
        h.succeed();
    }

    private static void corpseStays(GameTestHelper h) {
        SoldierEntity s = spawn(h);
        s.hurtServer(h.getLevel(), h.getLevel().damageSources().generic(), 100f);
        h.runAfterDelay(60, () -> {      // vanilla would have removed it after 20 ticks
            h.assertTrue(!s.isRemoved(), Component.literal("corpse removed too early (deathTime=" + s.deathTime + ")"));
            h.assertTrue(s.deathTime >= 55, Component.literal("death not ticking: " + s.deathTime));
            h.succeed();
        });
    }
}
