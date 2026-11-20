package com.kamikazeskies.gametest;

import java.util.function.Consumer;

import com.kamikazeskies.core.KamikazeSkies;
import com.kamikazeskies.core.KsRegistries;
import com.kamikazeskies.destruction.BlastExecutor;
import com.kamikazeskies.destruction.core.BlastProfile;
import com.kamikazeskies.drone.DroneEntity;
import com.kamikazeskies.drone.JammerEntity;
import com.kamikazeskies.drone.TrainingRange;
import com.kamikazeskies.sim.DroneParams;
import com.kamikazeskies.sim.FlightInput;

import net.minecraft.core.BlockPos;
import net.minecraft.core.Direction;
import net.minecraft.core.registries.Registries;
import net.minecraft.gametest.framework.GameTestHelper;
import net.minecraft.network.chat.Component;
import net.minecraft.world.entity.EntitySpawnReason;
import net.minecraft.world.item.DyeColor;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.phys.Vec3;
import net.neoforged.bus.api.IEventBus;
import net.neoforged.neoforge.registries.DeferredRegister;

/** Battery and radio-link GameTests. Each runs in its own environment because a jammer reaches 48 blocks. */
public final class LinkBatteryGameTests {
    private static final DeferredRegister<Consumer<GameTestHelper>> FUNCTIONS =
            DeferredRegister.create(Registries.TEST_FUNCTION, KamikazeSkies.MODID);
    private static final DroneParams P = DroneParams.fiveInch();

    static {
        reg("battery_drains_and_sags_in_flight", LinkBatteryGameTests::batteryDrains);
        reg("weak_battery_cannot_hold_altitude", LinkBatteryGameTests::weakBatterySinks);
        reg("link_is_clean_with_line_of_sight", LinkBatteryGameTests::linkClean);
        reg("link_is_weakened_by_a_thick_wall", LinkBatteryGameTests::linkBlockedByWall);
        reg("jammer_causes_failsafe", LinkBatteryGameTests::jammerFailsafe);
        reg("link_recovers_when_jammer_is_removed", LinkBatteryGameTests::linkRecovers);
        reg("jammer_is_destroyed_by_a_blast", LinkBatteryGameTests::jammerDestroyedByBlast);
        reg("training_range_builds_gates_and_a_window", LinkBatteryGameTests::trainingRange);
    }

    private LinkBatteryGameTests() {}

    private static void reg(String name, Consumer<GameTestHelper> fn) { FUNCTIONS.register(name, () -> fn); }
    public static void register(IEventBus bus) { FUNCTIONS.register(bus); }

    private static FlightInput hover() { return new FlightInput(P.hoverStick(), 0, 0, 0, true, 1); }

    /** A hovering drone 30 blocks from an operator standing at the origin. */
    private static DroneEntity droneAtDistance(GameTestHelper h, double dist, double y) {
        DroneEntity d = KsGameTests.spawnDrone(h, 0.5, y, 0.5 + dist);
        d.debugOperator(h.absoluteVec(new Vec3(0.5, y, 0.5)));
        d.debugSetInput(hover());
        return d;
    }

    private static JammerEntity jammerAt(GameTestHelper h, double x, double y, double z) {
        JammerEntity j = KsRegistries.JAMMER.get().create(h.getLevel(), EntitySpawnReason.COMMAND);
        j.snapTo(h.absoluteVec(new Vec3(x, y, z)));
        h.getLevel().addFreshEntity(j);
        return j;
    }

    // ---- tests --------------------------------------------------------------------------------------
    /** Flying at full throttle drains the pack and the loaded voltage sags. */
    private static void batteryDrains(GameTestHelper h) {
        DroneEntity d = KsGameTests.spawnDrone(h, 0.5, 20, 0.5);
        double startVolts = d.voltage();
        d.debugSetInput(new FlightInput(1, 0, 0, 0, true, 1));
        KsGameTests.afterTicks(h, d, 200, () -> {
            h.assertTrue(d.battery().soc() < 0.85, Component.literal("battery barely drained: soc=" + d.battery().soc()));
            h.assertTrue(d.voltage() < startVolts - 0.8, Component.literal("no voltage sag: " + startVolts + " -> " + d.voltage()));
            h.assertTrue(d.voltage() > 10, Component.literal("voltage unrealistically low: " + d.voltage()));
        });
    }

    /** With a nearly empty pack the same hover throttle no longer holds altitude. */
    private static void weakBatterySinks(GameTestHelper h) {
        KsGameTests.floor(h, 0, 12);
        DroneEntity d = KsGameTests.spawnDrone(h, 0.5, 60, 0.5);
        d.debugBatterySoc(0.02);
        d.debugSetInput(hover());
        KsGameTests.afterTicks(h, d, 60, () -> {
            h.assertTrue(d.sim().powerScale() < 0.75, Component.literal("power scale " + d.sim().powerScale()));
            h.assertTrue(KsGameTests.rel(h, d).y < 56, Component.literal("a weak pack should sink at hover throttle: y=" + KsGameTests.rel(h, d).y));
        });
    }

    private static void linkClean(GameTestHelper h) {
        DroneEntity d = droneAtDistance(h, 30, 20);
        KsGameTests.afterTicks(h, d, 40, () -> {
            h.assertTrue(d.rssi() >= 0.95f, Component.literal("clear line of sight should be a strong link, rssi=" + d.rssi()));
            h.assertTrue(!d.link().controlLost(), Component.literal("link lost on a clear path"));
        });
    }

    private static void linkBlockedByWall(GameTestHelper h) {
        for (int x = -4; x <= 4; x++) for (int y = 0; y <= 30; y++) for (int z = 12; z <= 15; z++) h.setBlock(new BlockPos(x, y, z), Blocks.STONE);   // 4 thick
        DroneEntity d = droneAtDistance(h, 30, 20);
        KsGameTests.afterTicks(h, d, 40, () -> {
            h.assertTrue(d.rssi() < 0.55f, Component.literal("a 4-block wall should cut the signal, rssi=" + d.rssi()));
            h.assertTrue(d.rssi() > 0.15f, Component.literal("signal should not be completely dead through 4 blocks, rssi=" + d.rssi()));
        });
    }

    /** Sitting on a jammer kills the link; after the failsafe delay the drone disarms and falls. */
    private static void jammerFailsafe(GameTestHelper h) {
        KsGameTests.floor(h, 0, 12);
        DroneEntity d = droneAtDistance(h, 30, 40);
        jammerAt(h, 0.5, 38, 30.5);
        KsGameTests.afterTicks(h, d, 30, () -> {   // soon after the failsafe, before the falling drone leaves the jammer's field
            h.assertTrue(d.rssi() < 0.05f, Component.literal("a jammer on top of the drone should kill the signal, rssi=" + d.rssi()));
            h.assertTrue(d.link().controlLost(), Component.literal("failsafe did not trigger"));
            h.assertTrue(!d.isArmed(), Component.literal("the drone should be disarmed by the failsafe"));
            h.assertTrue(KsGameTests.rel(h, d).y < 38, Component.literal("the drone should be falling, y=" + KsGameTests.rel(h, d).y));
        });
    }

    /** A partly jammed link is degraded but usable, and returns to full strength when the jammer is removed. */
    private static void linkRecovers(GameTestHelper h) {
        DroneEntity d = droneAtDistance(h, 30, 40);
        JammerEntity j = jammerAt(h, 0.5, 40, 30.5 + 24);
        boolean[] removed = {false};
        h.succeedWhen(() -> {
            h.assertTrue(d.simTicks() >= 40, Component.literal("waiting for ticks"));
            if (!removed[0]) {
                h.assertTrue(d.rssi() < 0.6f && d.rssi() > 0.05f, Component.literal("a distant jammer should degrade, not kill, the link: rssi=" + d.rssi()));
                h.assertTrue(!d.link().controlLost(), Component.literal("failsafe fired on a merely degraded link"));
                j.discard();
                removed[0] = true;
            }
            h.assertTrue(d.rssi() >= 0.9f, Component.literal("link should recover once the jammer is gone, rssi=" + d.rssi()));
        });
    }

    private static void jammerDestroyedByBlast(GameTestHelper h) {
        KsGameTests.floor(h, 0, 12);
        JammerEntity j = jammerAt(h, 0.5, 1, 6.5);
        BlastExecutor.detonate(h.getLevel(), h.absoluteVec(new Vec3(0.5, 1.5, 8.5)), new Vec3(0, 0, -1), BlastProfile.IMPACT, null);
        h.succeedWhen(() -> h.assertTrue(j.isRemoved(), Component.literal("the jammer survived a blast two blocks away")));
    }

    /** The training range has a clear opening in every gate, solid frames around them, and a window in the wall. */
    private static void trainingRange(GameTestHelper h) {
        KsGameTests.keepAreaTicking(h);
        BlockPos origin = h.absolutePos(new BlockPos(0, 1, 0));
        int placed = TrainingRange.build(h.getLevel(), origin, Direction.SOUTH);
        h.assertTrue(placed > 500, Component.literal("only " + placed + " blocks placed"));
        for (int k = 0; k < TrainingRange.GATE_DISTANCE.length; k++) {
            BlockPos c = TrainingRange.gateCenter(origin, Direction.SOUTH, k);
            h.assertTrue(h.getLevel().getBlockState(c).isAir(), Component.literal("gate " + k + " opening is blocked"));
            h.assertTrue(h.getLevel().getBlockState(c.above(3)).is(k == 5 ? TrainingRange.concrete(DyeColor.RED) : TrainingRange.concrete(DyeColor.LIME)), Component.literal("gate " + k + " has no top bar"));
            h.assertTrue(!h.getLevel().getBlockState(c.relative(Direction.SOUTH.getClockWise(), 3)).isAir(), Component.literal("gate " + k + " has no side"));
        }
        BlockPos windowCentre = origin.relative(Direction.SOUTH, TrainingRange.WALL_DISTANCE).above(3);
        h.assertTrue(h.getLevel().getBlockState(windowCentre).isAir(), Component.literal("the wall window is blocked"));
        h.assertTrue(h.getLevel().getBlockState(windowCentre.above(4)).is(Blocks.STONE_BRICKS), Component.literal("the wall is missing above the window"));
        h.assertTrue(h.getLevel().getBlockState(origin.below()).is(TrainingRange.concrete(DyeColor.LIGHT_GRAY)), Component.literal("no launch pad"));
        h.succeed();
    }
}
