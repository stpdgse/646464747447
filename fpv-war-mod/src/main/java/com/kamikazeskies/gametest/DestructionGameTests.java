package com.kamikazeskies.gametest;

import java.util.List;
import java.util.function.Consumer;

import com.kamikazeskies.core.KamikazeSkies;
import com.kamikazeskies.core.KsConfig;
import com.kamikazeskies.destruction.BlastExecutor;
import com.kamikazeskies.destruction.CollapseManager;
import com.kamikazeskies.destruction.core.BlastProfile;
import com.kamikazeskies.destruction.core.QualityPreset;
import com.kamikazeskies.drone.DroneEntity;
import com.kamikazeskies.drone.Warhead;
import com.kamikazeskies.sim.DroneParams;
import com.kamikazeskies.sim.FlightInput;

import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.Registries;
import net.minecraft.gametest.framework.GameTestHelper;
import net.minecraft.network.chat.Component;
import net.minecraft.world.entity.item.FallingBlockEntity;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;
import net.neoforged.bus.api.IEventBus;
import net.neoforged.neoforge.registries.DeferredRegister;

/**
 * Destruction GameTests. Each test has its own test environment (see data/.../test_environment) so tests run one
 * after another: a blast's stability analysis reaches ~20 blocks and would otherwise pull a neighbouring test's
 * structures into the result.
 */
public final class DestructionGameTests {
    private static final DeferredRegister<Consumer<GameTestHelper>> FUNCTIONS =
            DeferredRegister.create(Registries.TEST_FUNCTION, KamikazeSkies.MODID);

    static {
        reg("blast_breaks_walls_but_not_obsidian", DestructionGameTests::blastBreaksWallsButNotObsidian);
        reg("blast_debris_is_capped", DestructionGameTests::blastDebrisIsCapped);
        reg("shaped_charge_penetrates_deeper", DestructionGameTests::shapedChargePenetratesDeeper);
        reg("tower_collapses_when_base_is_blasted", DestructionGameTests::towerCollapses);
        reg("terrain_never_collapses", DestructionGameTests::terrainNeverCollapses);
        reg("collapse_respects_tick_budget", DestructionGameTests::collapseRespectsBudget);
        reg("strike_drone_detonates_on_wall", DestructionGameTests::strikeDroneDetonates);
        reg("unarmed_drone_crash_makes_no_blast", DestructionGameTests::unarmedCrashMakesNoBlast);
    }

    private DestructionGameTests() {}

    private static void reg(String name, Consumer<GameTestHelper> fn) { FUNCTIONS.register(name, () -> fn); }
    public static void register(IEventBus bus) { FUNCTIONS.register(bus); }

    // ---- helpers ------------------------------------------------------------------------------------
    private static void fill(GameTestHelper h, int x0, int y0, int z0, int x1, int y1, int z1, Block b) {
        for (int x = Math.min(x0, x1); x <= Math.max(x0, x1); x++)
            for (int y = Math.min(y0, y1); y <= Math.max(y0, y1); y++)
                for (int z = Math.min(z0, z1); z <= Math.max(z0, z1); z++) h.setBlock(new BlockPos(x, y, z), b);
    }

    private static boolean is(GameTestHelper h, int x, int y, int z, Block b) { return h.getBlockState(new BlockPos(x, y, z)).is(b); }
    private static boolean air(GameTestHelper h, int x, int y, int z) { return h.getBlockState(new BlockPos(x, y, z)).isAir(); }

    private static int count(GameTestHelper h, int x0, int y0, int z0, int x1, int y1, int z1, Block b) {
        int n = 0;
        for (int x = x0; x <= x1; x++) for (int y = y0; y <= y1; y++) for (int z = z0; z <= z1; z++) if (is(h, x, y, z, b)) n++;
        return n;
    }

    private static BlastExecutor.Report blast(GameTestHelper h, double x, double y, double z, Vec3 axis, BlastProfile p) {
        return BlastExecutor.detonate(h.getLevel(), h.absoluteVec(new Vec3(x, y, z)), axis, p, null);
    }

    /** A hollow planks room: 13x13 footprint, walls 3 thick around a 7x7x7 interior centred on (0, 5, 0). */
    private static void room(GameTestHelper h) {
        KsGameTests.floor(h, 0, 12);
        fill(h, -6, 1, -6, 6, 13, 6, Blocks.OAK_PLANKS);
        fill(h, -3, 2, -3, 3, 8, 3, Blocks.AIR);
    }

    private static CollapseManager.Stats stats() { return CollapseManager.STATS; }

    // ---- tests --------------------------------------------------------------------------------------
    /** An impact blast inside a room breaks the nearest wall layer; far blocks, and unbreakable obsidian, survive. */
    private static void blastBreaksWallsButNotObsidian(GameTestHelper h) {
        room(h);
        h.setBlock(new BlockPos(4, 5, 0), Blocks.OBSIDIAN);          // inside the blast's reach, but unbreakable
        BlastExecutor.Report r = blast(h, 0.5, 5.5, 0.5, new Vec3(0, 0, 1), BlastProfile.IMPACT);

        h.assertTrue(r.blocksDestroyed() > 20, Component.literal("blast destroyed only " + r.blocksDestroyed() + " blocks"));
        int innerLayerGone = 0, innerLayerTotal = 0;
        for (int y = 3; y <= 7; y++) for (int z = -2; z <= 2; z++) { innerLayerTotal++; if (air(h, -4, y, z)) innerLayerGone++; }
        h.assertTrue(innerLayerGone >= innerLayerTotal / 2, Component.literal("only " + innerLayerGone + "/" + innerLayerTotal + " of the nearest wall layer broke"));
        h.assertTrue(is(h, 4, 5, 0, Blocks.OBSIDIAN), Component.literal("obsidian was destroyed"));
        h.assertTrue(is(h, 6, 13, 6, Blocks.OAK_PLANKS) && is(h, -6, 13, -6, Blocks.OAK_PLANKS), Component.literal("far corners were damaged"));
        h.assertTrue(r.radius() < 8, Component.literal("crater radius " + r.radius() + " is too large for this warhead"));
        h.succeed();
    }

    /** Thrown debris never exceeds the quality preset's cap. */
    private static void blastDebrisIsCapped(GameTestHelper h) {
        room(h);
        BlastExecutor.Report r = blast(h, 0.5, 5.5, 0.5, new Vec3(0, 0, 1), BlastProfile.IMPACT);
        QualityPreset q = KsConfig.QUALITY.get();
        AABB area = new AABB(h.absoluteVec(new Vec3(0.5, 5.5, 0.5)), h.absoluteVec(new Vec3(0.5, 5.5, 0.5))).inflate(60);
        int falling = h.getLevel().getEntitiesOfClass(FallingBlockEntity.class, area).size();
        h.assertTrue(r.debris() > 0 && falling > 0, Component.literal("a blast should throw some debris, got " + r.debris()));
        h.assertTrue(r.debris() <= q.maxFlyingDebris, Component.literal("debris " + r.debris() + " exceeds cap " + q.maxFlyingDebris));
        h.assertTrue(falling <= q.maxFallingBlocks, Component.literal(falling + " falling blocks exceeds cap " + q.maxFallingBlocks));
        h.succeed();
    }

    /** Same wall of stone bricks: the shaped charge goes several blocks deeper than the impact warhead. */
    private static void shapedChargePenetratesDeeper(GameTestHelper h) {
        KsGameTests.floor(h, 0, 12);
        fill(h, -8, 1, 3, -6, 3, 17, Blocks.STONE_BRICKS);    // bar A
        fill(h, 6, 1, 3, 8, 3, 17, Blocks.STONE_BRICKS);      // bar B
        blast(h, -6.5 + 1, 2.5, 2.0, new Vec3(0, 0, 1), BlastProfile.IMPACT);
        blast(h, 6.5 + 1, 2.5, 2.0, new Vec3(0, 0, 1), BlastProfile.SHAPED);
        int impactDepth = depth(h, -7), shapedDepth = depth(h, 7);
        h.assertTrue(shapedDepth >= impactDepth + 2, Component.literal("shaped depth " + shapedDepth + " vs impact " + impactDepth));
        h.assertTrue(shapedDepth >= 4, Component.literal("shaped charge only went " + shapedDepth + " blocks"));
        h.succeed();
    }

    private static int depth(GameTestHelper h, int x) {
        int d = 0;
        for (int z = 3; z <= 17; z++) if (air(h, x, 2, z)) d = z - 2;
        return d;
    }

    /** Blasting the base of a tower removes its support; the pillar and platform above fall. */
    private static void towerCollapses(GameTestHelper h) {
        KsGameTests.floor(h, 0, 12);
        fill(h, 0, 1, 0, 0, 8, 0, Blocks.STONE_BRICKS);                     // pillar
        fill(h, -1, 9, -1, 1, 9, 1, Blocks.OAK_PLANKS);                     // 3x3 platform
        h.assertTrue(CollapseManager.analyzeNow(h.getLevel(), h.absolutePos(new BlockPos(0, 5, 0)), 14).isEmpty(),
                Component.literal("the intact tower should be stable"));
        stats().reset();
        blast(h, 0.5, 1.2, 0.5, new Vec3(0, 0, 1), BlastProfile.IMPACT);

        h.succeedWhen(() -> {
            h.assertTrue(!CollapseManager.busy(), Component.literal("waiting for the collapse to finish"));
            h.assertTrue(air(h, 0, 9, 0) && air(h, 1, 9, 1) && air(h, -1, 9, -1), Component.literal("the platform is still standing"));
            h.assertTrue(air(h, 0, 6, 0), Component.literal("the pillar is still standing"));
            h.assertTrue(stats().converted >= 10, Component.literal("only " + stats().converted + " blocks collapsed"));
        });
    }

    /** Natural terrain is an anchor, not a structure: a floating island and what sits on it stay put. */
    private static void terrainNeverCollapses(GameTestHelper h) {
        KsGameTests.floor(h, 0, 12);
        fill(h, -3, 12, -3, 3, 12, 3, Blocks.GRASS_BLOCK);                  // floating terrain island
        h.setBlock(new BlockPos(0, 13, 0), Blocks.OAK_PLANKS);              // a built block resting on it
        List<Long> unsupported = CollapseManager.analyzeNow(h.getLevel(), h.absolutePos(new BlockPos(0, 12, 0)), 12);
        h.assertTrue(unsupported.isEmpty(), Component.literal("a floating island of terrain must not count as unsupported; " + unsupported.size() + " cells, e.g. "
                + unsupported.stream().limit(4).map(k -> h.relativePos(new BlockPos(com.kamikazeskies.destruction.core.CellKey.x(k), com.kamikazeskies.destruction.core.CellKey.y(k),
                        com.kamikazeskies.destruction.core.CellKey.z(k))) + "=" + h.getBlockState(h.relativePos(new BlockPos(com.kamikazeskies.destruction.core.CellKey.x(k),
                        com.kamikazeskies.destruction.core.CellKey.y(k), com.kamikazeskies.destruction.core.CellKey.z(k)))).getBlock()).toList()));
        blast(h, 9.5, 12.5, 0.5, new Vec3(-1, 0, 0), BlastProfile.IMPACT);  // nearby, but out of reach of the island's interior

        h.succeedWhen(() -> {
            h.assertTrue(!CollapseManager.busy(), Component.literal("waiting for the analysis to finish"));
            h.assertTrue(is(h, -3, 12, 0, Blocks.GRASS_BLOCK) && is(h, 0, 12, 0, Blocks.GRASS_BLOCK), Component.literal("terrain collapsed"));
            h.assertTrue(is(h, 0, 13, 0, Blocks.OAK_PLANKS), Component.literal("a block resting on terrain fell"));
        });
    }

    /** A huge unsupported slab is converted gradually: never more than the per-tick budget, and falling entities stay under the cap. */
    private static void collapseRespectsBudget(GameTestHelper h) {
        KsGameTests.floor(h, 0, 12);
        fill(h, -10, 30, -10, 9, 31, 9, Blocks.OAK_PLANKS);                 // 800 blocks hanging in the air
        QualityPreset q = KsConfig.QUALITY.get();
        BlockPos centre = h.absolutePos(new BlockPos(0, 30, 0));
        List<Long> cells = CollapseManager.analyzeNow(h.getLevel(), centre, 25);
        h.assertTrue(cells.size() >= 800, Component.literal("expected at least the 800 slab blocks, found " + cells.size()));   // leftovers of other tests may add more
        int expected = cells.size();
        stats().reset();
        CollapseManager.enqueue(h.getLevel(), centre, cells);

        AABB area = new AABB(h.absoluteVec(new Vec3(0, 30, 0)), h.absoluteVec(new Vec3(0, 30, 0))).inflate(60);
        int[] peakFalling = {0};
        h.succeedWhen(() -> {
            peakFalling[0] = Math.max(peakFalling[0], h.getLevel().getEntitiesOfClass(FallingBlockEntity.class, area).size());
            h.assertTrue(!CollapseManager.busy(), Component.literal("collapse still in progress"));
            h.assertTrue(stats().maxPerTick > 0 && stats().maxPerTick <= q.blocksPerTick,
                    Component.literal("per-tick conversions " + stats().maxPerTick + " vs budget " + q.blocksPerTick));
            h.assertTrue(stats().converted >= 800 && stats().converted <= expected, Component.literal("converted " + stats().converted + " of " + expected));
            h.assertTrue(peakFalling[0] <= q.maxFallingBlocks, Component.literal("peak falling blocks " + peakFalling[0] + " exceeded cap " + q.maxFallingBlocks));
            h.assertTrue(stats().crumbled > 0, Component.literal("blocks beyond the falling cap should crumble"));
        });
    }

    /** A strike drone flying into a wall explodes: the drone is gone and the wall has a hole. */
    private static void strikeDroneDetonates(GameTestHelper h) {
        KsGameTests.floor(h, 0, 12);
        fill(h, -5, 1, 16, 5, 7, 16, Blocks.OAK_PLANKS);                    // one-block-thick wall
        int before = count(h, -5, 1, 16, 5, 7, 16, Blocks.OAK_PLANKS);
        DroneEntity d = KsGameTests.spawnDrone(h, 0.5, 4.0, 4.5);
        d.setWarhead(Warhead.IMPACT);
        DroneParams p = DroneParams.fiveInch();
        double tilt = Math.toRadians(0.5 * p.maxAngleDeg());
        d.debugSetInput(new FlightInput(p.stickForThrustFraction(p.hoverThrustFraction() / Math.cos(tilt) * 1.03), 0, 0.5, 0, true, 1));   // level flight at a 27 degree tilt

        h.succeedWhen(() -> {
            h.assertTrue(d.isRemoved(), Component.literal("drone has not hit the wall yet: pos=" + KsGameTests.rel(h, d) + " vel=" + d.sim().velocity()
                    + " armed=" + d.isArmed() + " onGround=" + d.onGround() + " simTicks=" + d.simTicks()));
            int after = count(h, -5, 1, 16, 5, 7, 16, Blocks.OAK_PLANKS);
            h.assertTrue(before - after >= 6, Component.literal("the blast removed only " + (before - after) + " wall blocks"));
            h.assertTrue(is(h, -5, 1, 16, Blocks.OAK_PLANKS) && is(h, 5, 7, 16, Blocks.OAK_PLANKS), Component.literal("blast was too large; wall ends damaged"));
        });
    }

    /** Without a warhead a crash destroys only the drone. */
    private static void unarmedCrashMakesNoBlast(GameTestHelper h) {
        KsGameTests.keepAreaTicking(h);
        fill(h, -2, 0, -2, 2, 0, 2, Blocks.OAK_PLANKS);
        DroneEntity d = KsGameTests.spawnDrone(h, 0.5, 30, 0.5);
        d.debugSetInput(FlightInput.DISARMED);
        h.succeedWhen(() -> {
            h.assertTrue(d.isRemoved(), Component.literal("drone has not crashed yet"));
            h.assertTrue(count(h, -2, 0, -2, 2, 0, 2, Blocks.OAK_PLANKS) == 25, Component.literal("an unarmed crash damaged the floor"));
        });
    }
}
