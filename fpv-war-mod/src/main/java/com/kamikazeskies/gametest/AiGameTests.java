package com.kamikazeskies.gametest;

import java.util.function.Consumer;

import com.kamikazeskies.ai.Behaviour;
import com.kamikazeskies.ai.Role;
import com.kamikazeskies.core.KamikazeSkies;
import com.kamikazeskies.core.KsRegistries;
import com.kamikazeskies.drone.DroneEntity;
import com.kamikazeskies.drone.SoldierAnim;
import com.kamikazeskies.drone.SoldierEntity;
import com.kamikazeskies.drone.TankEntity;
import com.kamikazeskies.sim.DroneParams;
import com.kamikazeskies.sim.FlightInput;

import net.minecraft.core.registries.Registries;
import net.minecraft.gametest.framework.GameTestHelper;
import net.minecraft.network.chat.Component;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.phys.Vec3;
import net.neoforged.bus.api.IEventBus;
import net.neoforged.neoforge.registries.DeferredRegister;

/** Soldier and tank behaviour: fear, fleeing, engaging, cover, alarm, tank traverse. */
public final class AiGameTests {
    private static final DeferredRegister<Consumer<GameTestHelper>> FUNCTIONS = DeferredRegister.create(Registries.TEST_FUNCTION, KamikazeSkies.MODID);
    private static final double HOVER = DroneParams.fiveInch().hoverStick();
    static {
        reg("soldier_flees_when_terrified", AiGameTests::flees);
        reg("veteran_engages_a_hovering_drone", AiGameTests::engages);
        reg("conscript_takes_cover_instead_of_shooting", AiGameTests::cover);
        reg("blast_alarm_frightens_nearby_soldiers", AiGameTests::alarm);
        reg("soldiers_fear_a_friend_dying", AiGameTests::allyDeath);
        reg("calm_soldier_stays_idle_without_threat", AiGameTests::calm);
        reg("tank_traverses_onto_a_drone", AiGameTests::tankTraverses);
    }
    private static void reg(String name, Consumer<GameTestHelper> fn) { FUNCTIONS.register(name, () -> fn); }
    public static void register(IEventBus bus) { FUNCTIONS.register(bus); }

    private static SoldierEntity soldier(GameTestHelper h, Role role, double x, double z) {
        KsGameTests.keepAreaTicking(h);
        SoldierEntity s = h.spawn(KsRegistries.SOLDIER.get(), new Vec3(x, 2, z));
        s.setRole(role);
        return s;
    }

    private static DroneEntity armedHover(GameTestHelper h, double x, double y, double z) {
        DroneEntity d = KsGameTests.spawnDrone(h, x, y, z);
        d.debugSetInput(new FlightInput(HOVER, 0, 0, 0, true, 1));
        return d;
    }

    private static void flees(GameTestHelper h) {
        KsGameTests.floor(h, 1, 20);
        SoldierEntity s = soldier(h, Role.CONSCRIPT, 0.5, 0.5);
        Vec3 start = s.position();
        DroneEntity d = armedHover(h, 0.5, 4, 8.5);
        s.setFear(0.97);
        boolean[] fled = {false};
        h.succeedWhen(() -> {                                   // fear decays, so catch him while he is running
            if (s.behaviour() == Behaviour.FLEE && s.stance() == SoldierAnim.Stance.PANIC) fled[0] = true;
            h.assertTrue(s.aiTicks() > 20, Component.literal("waiting for the soldier to tick"));
            h.assertTrue(fled[0], Component.literal("never fled: " + s.behaviour() + " fear " + s.fear()));
            h.assertTrue(s.position().distanceTo(start) > 3.0, Component.literal("barely moved: " + s.position().distanceTo(start)));
            h.assertTrue(s.position().distanceTo(d.position()) > start.distanceTo(d.position()) - 0.5, Component.literal("ran toward the drone"));
        });
    }

    private static void engages(GameTestHelper h) {
        KsGameTests.floor(h, 1, 20);
        SoldierEntity s = soldier(h, Role.VETERAN, 0.5, 0.5);
        DroneEntity d = armedHover(h, 0.5, 4, 14.5);
        h.succeedWhen(() -> h.assertTrue(s.ammo() < SoldierEntity.MAG_SIZE || d.isRemoved(), Component.literal("veteran never fired: behaviour " + s.behaviour() + ", fear " + s.fear())));
    }

    private static void cover(GameTestHelper h) {
        KsGameTests.floor(h, 1, 20);
        SoldierEntity s = soldier(h, Role.CONSCRIPT, 0.5, 0.5);
        armedHover(h, 0.5, 4, 6.5);
        s.setFear(0.45);
        boolean[] hid = {false};
        h.succeedWhen(() -> {
            if (s.behaviour() == Behaviour.COVER || s.behaviour() == Behaviour.FLEE) hid[0] = true;
            h.assertTrue(s.aiTicks() > 20, Component.literal("waiting for the soldier to tick"));
            h.assertTrue(hid[0], Component.literal("never took cover: " + s.behaviour() + " fear " + s.fear()));
            h.assertTrue(s.ammo() == SoldierEntity.MAG_SIZE, Component.literal("a frightened conscript fired anyway"));
        });
    }

    private static void alarm(GameTestHelper h) {
        KsGameTests.floor(h, 1, 20);
        SoldierEntity near = soldier(h, Role.RIFLEMAN, 0.5, 0.5), far = soldier(h, Role.RIFLEMAN, 0.5, 30.5);
        SoldierEntity.alarm(h.getLevel(), h.absoluteVec(new Vec3(0.5, 2, 4.5)), 6);
        h.assertTrue(near.fear() > 0.3, Component.literal("near soldier fear " + near.fear()));
        h.assertTrue(far.fear() < near.fear(), Component.literal("far soldier was as scared as the near one"));
        h.succeed();
    }

    private static void allyDeath(GameTestHelper h) {
        KsGameTests.floor(h, 1, 20);
        SoldierEntity victim = soldier(h, Role.RIFLEMAN, 0.5, 0.5), witness = soldier(h, Role.CONSCRIPT, 3.5, 0.5);
        double before = witness.fear();
        victim.hurtServer(h.getLevel(), h.getLevel().damageSources().generic(), 100f);
        h.assertTrue(witness.fear() > before + 0.1, Component.literal("witness fear " + witness.fear()));
        h.succeed();
    }

    private static void calm(GameTestHelper h) {
        KsGameTests.floor(h, 1, 20);
        SoldierEntity s = soldier(h, Role.RIFLEMAN, 0.5, 0.5);
        h.succeedWhen(() -> {
            h.assertTrue(s.aiTicks() >= 60, Component.literal("waiting for the soldier to tick"));
            h.assertTrue(s.behaviour() == Behaviour.IDLE && s.fear() < 0.01, Component.literal("behaviour " + s.behaviour() + " fear " + s.fear()));
        });
    }

    private static void tankTraverses(GameTestHelper h) {
        KsGameTests.floor(h, 1, 40);
        TankEntity t = h.spawn(KsRegistries.TANK.get(), new Vec3(0.5, 2, 0.5));
        t.setScanning(true);
        DroneEntity d = armedHover(h, 20.5, 5, 0.5);
        h.succeedWhen(() -> {
            h.assertTrue(t.aiTicks() >= 60, Component.literal("waiting for the tank to tick"));
            h.assertTrue(!t.scanning() || d.isRemoved(), Component.literal("tank still scanning with a drone in view"));
            double want = Math.toDegrees(Math.atan2(-(d.getX() - t.getX()), d.getZ() - t.getZ()));
            double diff = Math.abs(net.minecraft.util.Mth.wrapDegrees((float) (want - t.getYRot())));
            h.assertTrue(d.isRemoved() || diff < 15, Component.literal("hull not aligned: off by " + diff));
        });
    }
}
