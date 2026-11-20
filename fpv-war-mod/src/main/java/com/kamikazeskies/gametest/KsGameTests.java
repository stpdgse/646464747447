package com.kamikazeskies.gametest;

import java.util.function.Consumer;

import com.kamikazeskies.core.KamikazeSkies;
import com.kamikazeskies.core.KsRegistries;
import com.kamikazeskies.drone.DroneEntity;
import com.kamikazeskies.drone.PilotRegistry;
import com.kamikazeskies.sim.DroneParams;
import com.kamikazeskies.sim.FlightInput;

import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.Registries;
import net.minecraft.gametest.framework.GameTestHelper;
import net.minecraft.network.chat.Component;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.phys.Vec3;
import net.neoforged.bus.api.IEventBus;
import net.neoforged.neoforge.registries.DeferredRegister;

/**
 * GameTest functions. Each function is registered in the TEST_FUNCTION registry and referenced from a
 * JSON file in {@code data/kamikaze_skies/test_instance/}. Run all with {@code ./gradlew runGameTestServer}.
 * Coordinates passed to helper methods are relative to the (empty) test structure origin.
 */
public final class KsGameTests {
    private static final DeferredRegister<Consumer<GameTestHelper>> FUNCTIONS =
            DeferredRegister.create(Registries.TEST_FUNCTION, KamikazeSkies.MODID);
    private static final double HOVER = DroneParams.fiveInch().hoverStick();

    static {
        reg("smoke", KsGameTests::smoke);
        reg("drone_climbs", KsGameTests::droneClimbs);
        reg("drone_hovers", KsGameTests::droneHovers);
        reg("drone_flies_forward", KsGameTests::droneFliesForward);
        reg("drone_blocked_by_wall", KsGameTests::droneBlockedByWall);
        reg("drone_crashes_on_hard_impact", KsGameTests::droneCrashesOnHardImpact);
        reg("drone_survives_soft_landing", KsGameTests::droneSurvivesSoftLanding);
        reg("drone_failsafe_when_link_lost", KsGameTests::droneFailsafe);
        reg("pilot_flow", KsGameTests::pilotFlow);
    }

    private KsGameTests() {}

    private static void reg(String name, Consumer<GameTestHelper> fn) { FUNCTIONS.register(name, () -> fn); }

    public static void register(IEventBus modEventBus) { FUNCTIONS.register(modEventBus); }

    // ---- helpers ------------------------------------------------------------------------------------
    /**
     * The 1x1 test structure only keeps its own chunk ticking; drones that fly into neighbouring chunks (or are
     * spawned there) would otherwise start ticking late and make tests nondeterministic. Force-load a 5x5 area.
     */
    private static void keepAreaTicking(GameTestHelper h) {
        BlockPos origin = h.absolutePos(BlockPos.ZERO);
        int cx = origin.getX() >> 4, cz = origin.getZ() >> 4;
        for (int dx = -2; dx <= 2; dx++) for (int dz = -2; dz <= 2; dz++) h.getLevel().setChunkForced(cx + dx, cz + dz, true);
    }

    private static DroneEntity spawnDrone(GameTestHelper h, double x, double y, double z) {
        keepAreaTicking(h);
        DroneEntity d = h.spawn(KsRegistries.DRONE.get(), new Vec3(x, y, z));
        d.resetOrientation(0);            // facing +Z
        d.debugSpinUpToHover();
        return d;
    }

    private static void floor(GameTestHelper h, int y, int half) {
        keepAreaTicking(h);
        for (int x = -half; x <= half; x++) for (int z = -half; z <= half + 20; z++) h.setBlock(new BlockPos(x, y, z), Blocks.STONE);
    }

    /**
     * Run {@code check} exactly once, after the drone has simulated {@code ticks} server ticks. Waits (retrying every
     * tick) until then, so results do not depend on when the chunk started entity-ticking. A failing check keeps
     * failing, which surfaces as the test failure at timeout.
     */
    private static void afterTicks(GameTestHelper h, DroneEntity d, int ticks, Runnable check) {
        RuntimeException[] failure = {null};
        boolean[] ran = {false};
        h.succeedWhen(() -> {
            h.assertTrue(d.simTicks() >= ticks, Component.literal("waiting: drone simulated " + d.simTicks() + "/" + ticks + " ticks"));
            if (!ran[0]) {
                ran[0] = true;
                try { check.run(); } catch (RuntimeException e) { failure[0] = e; }
            }
            if (failure[0] != null) throw failure[0];
        });
    }

    private static Vec3 rel(GameTestHelper h, DroneEntity d) { return h.relativeVec(d.position()); }

    // ---- tests --------------------------------------------------------------------------------------
    /** Phase 0 smoke test: the world is live, blocks can be placed and read back. */
    private static void smoke(GameTestHelper helper) {
        BlockPos pos = new BlockPos(1, 1, 1);
        helper.setBlock(pos, Blocks.STONE);
        helper.assertBlock(pos, b -> b == Blocks.STONE, b -> Component.literal("expected stone, found " + b));
        helper.setBlock(pos, Blocks.AIR);
        helper.assertBlock(pos, b -> b == Blocks.AIR, b -> Component.literal("expected air, found " + b));
        helper.succeed();
    }

    /** Full throttle: the server-simulated drone gains altitude quickly. */
    private static void droneClimbs(GameTestHelper h) {
        DroneEntity d = spawnDrone(h, 1.5, 5, 1.5);
        d.debugSetInput(new FlightInput(1, 0, 0, 0, true, 1));
        afterTicks(h, d, 20, () -> {
            double climbed = rel(h, d).y - 5;
            h.assertTrue(climbed > 5, Component.literal("climbed only " + climbed + " blocks in 1 s"));
        });
    }

    /** Hover throttle in angle mode: altitude and position hold. */
    private static void droneHovers(GameTestHelper h) {
        DroneEntity d = spawnDrone(h, 1.5, 5, 1.5);
        d.debugSetInput(new FlightInput(HOVER, 0, 0, 0, true, 1));
        afterTicks(h, d, 80, () -> {
            Vec3 p = rel(h, d);
            h.assertTrue(Math.abs(p.y - 5) < 0.3, Component.literal("altitude drifted to " + p.y));
            h.assertTrue(Math.abs(p.x - 1.5) < 0.3 && Math.abs(p.z - 1.5) < 0.3, Component.literal("drifted to " + p));
        });
    }

    /** Angle mode, nose-down stick while facing +Z: moves toward +Z only. */
    private static void droneFliesForward(GameTestHelper h) {
        DroneEntity d = spawnDrone(h, 1.5, 8, 1.5);
        d.debugSetInput(new FlightInput(HOVER + 0.04, 0, 0.5, 0, true, 1));
        afterTicks(h, d, 40, () -> {
            Vec3 p = rel(h, d);
            h.assertTrue(p.z > 4.0, Component.literal("only moved to z=" + p.z));
            h.assertTrue(Math.abs(p.x - 1.5) < 0.6, Component.literal("sideways drift x=" + p.x));
        });
    }

    /** A wall stops the drone (real block collision) and a gentle bump does not destroy it. */
    private static void droneBlockedByWall(GameTestHelper h) {
        keepAreaTicking(h);
        for (int x = -4; x <= 6; x++) for (int y = 0; y <= 40; y++) h.setBlock(new BlockPos(x, y, 12), Blocks.STONE);   // tall: the drone climbs a bit
        DroneEntity d = spawnDrone(h, 1.5, 8, 4.5);
        d.debugSetInput(new FlightInput(HOVER + 0.03, 0, 0.25, 0, true, 1));
        afterTicks(h, d, 100, () -> {
            h.assertTrue(!d.isRemoved(), Component.literal("drone was destroyed by a gentle contact"));
            Vec3 p = rel(h, d);
            h.assertTrue(p.z < 12.0, Component.literal("drone went through the wall: z=" + p.z));
            h.assertTrue(p.z > 11.5, Component.literal("drone never reached the wall: pos=" + p));
        });
    }

    /** Dropping from height onto the ground at high speed destroys the drone. */
    private static void droneCrashesOnHardImpact(GameTestHelper h) {
        floor(h, 0, 4);
        DroneEntity d = spawnDrone(h, 1.5, 30, 1.5);
        d.debugSetInput(FlightInput.DISARMED);
        h.succeedWhen(() -> h.assertTrue(d.isRemoved(), Component.literal("drone survived a 30 m fall at y=" + d.getY())));
    }

    /** A short drop (about 4 m/s) lands intact and stays on the ground. */
    private static void droneSurvivesSoftLanding(GameTestHelper h) {
        floor(h, 0, 4);
        DroneEntity d = spawnDrone(h, 1.5, 2.5, 1.5);
        d.debugSetInput(FlightInput.DISARMED);
        afterTicks(h, d, 60, () -> {
            h.assertTrue(!d.isRemoved(), Component.literal("drone crashed on a soft landing"));
            h.assertTrue(d.onGround(), Component.literal("drone is not on the ground, y=" + d.getY()));
            h.assertTrue(d.sim().speed() < 0.5, Component.literal("still moving at " + d.sim().speed()));
        });
    }

    /** Without fresh pilot input the link times out and the drone drops (failsafe). */
    private static void droneFailsafe(GameTestHelper h) {
        DroneEntity d = spawnDrone(h, 1.5, 20, 1.5);
        // simulate a pilot whose packets stopped arriving: input is set, then goes stale
        d.debugSetInput(new FlightInput(1, 0, 0, 0, true, 1));
        d.debugExpireInput();
        afterTicks(h, d, 30, () -> {
            h.assertTrue(rel(h, d).y < 20 || d.isRemoved(), Component.literal("drone kept climbing without a link"));
            h.assertTrue(!d.isArmed(), Component.literal("drone still armed without a link"));
        });
    }

    /**
     * Full pilot flow on the server: bind, reject a stranger's input, accept the pilot's input (kept fresh
     * every tick like the real client does), then release.
     */
    private static void pilotFlow(GameTestHelper h) {
        DroneEntity d = spawnDrone(h, 1.5, 5, 1.5);
        ServerPlayer pilot = h.makeMockServerPlayerInLevel();
        ServerPlayer stranger = h.makeMockServerPlayerInLevel();

        d.startPilot(pilot);
        h.assertTrue(PilotRegistry.of(pilot) == d, Component.literal("pilot not registered"));
        h.assertTrue(d.pilotEntityId() == pilot.getId(), Component.literal("pilot id not synced"));

        FlightInput full = new FlightInput(1, 0, 0, 0, true, 1);
        d.acceptPilotInput(stranger, full);                      // not the pilot: must be ignored
        h.assertTrue(d.sim().velocity().y() == 0 && !d.isArmed(), Component.literal("stranger's input was accepted"));

        for (int t = 1; t <= 400; t++) h.runAfterDelay(t, () -> d.acceptPilotInput(pilot, full));   // client sends every tick

        boolean[] released = {false};
        h.succeedWhen(() -> {
            if (!released[0]) {
                h.assertTrue(d.simTicks() >= 20, Component.literal("waiting for ticks"));
                h.assertTrue(rel(h, d).y > 8, Component.literal("pilot input did not make the drone climb: y=" + rel(h, d).y));
                h.assertTrue(d.isArmed(), Component.literal("drone not armed by pilot input"));
                PilotRegistry.releasePlayer(pilot);               // what ExitPilotPayload does
                released[0] = true;
            }
            h.assertTrue(d.pilot() == null && d.pilotEntityId() == -1, Component.literal("pilot not released"));
            h.assertTrue(PilotRegistry.of(pilot) == null, Component.literal("registry still bound"));
        });
    }
}
