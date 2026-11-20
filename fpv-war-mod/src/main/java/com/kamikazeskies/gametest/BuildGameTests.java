package com.kamikazeskies.gametest;

import java.util.List;
import java.util.Optional;
import java.util.function.Consumer;

import com.kamikazeskies.build.DroneBuild;
import com.kamikazeskies.build.PartKind;
import com.kamikazeskies.core.KamikazeSkies;
import com.kamikazeskies.core.KsConfig;
import com.kamikazeskies.core.KsRegistries;
import com.kamikazeskies.drone.DroneEntity;
import com.kamikazeskies.drone.Warhead;
import com.kamikazeskies.item.DroneAssemblyRecipe;
import com.kamikazeskies.item.DroneWorkbenchBlock;
import com.kamikazeskies.item.FlightLog;
import com.kamikazeskies.item.KsComponents;
import com.kamikazeskies.sim.DroneParams;
import com.kamikazeskies.sim.FlightInput;

import net.minecraft.core.BlockPos;
import net.minecraft.core.registries.Registries;
import net.minecraft.gametest.framework.GameTestHelper;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.Identifier;
import net.minecraft.resources.ResourceKey;
import net.minecraft.server.level.ServerLevel;
import net.minecraft.server.level.ServerPlayer;
import net.minecraft.world.entity.item.ItemEntity;
import net.minecraft.world.inventory.ContainerLevelAccess;
import net.minecraft.world.item.Item;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.Items;
import net.minecraft.world.item.crafting.CraftingInput;
import net.minecraft.world.item.crafting.RecipeHolder;
import net.minecraft.world.item.crafting.RecipeType;
import net.minecraft.world.level.block.Blocks;
import net.minecraft.world.phys.AABB;
import net.minecraft.world.phys.Vec3;
import net.neoforged.bus.api.IEventBus;
import net.neoforged.neoforge.registries.DeferredRegister;

/** GameTests for crafting, assembly, the workbench, and how a build changes the drone. Each test has its own environment. */
public final class BuildGameTests {
    private static final DeferredRegister<Consumer<GameTestHelper>> FUNCTIONS =
            DeferredRegister.create(Registries.TEST_FUNCTION, KamikazeSkies.MODID);
    private static final DroneParams P = DroneParams.fiveInch();

    static {
        reg("assembly_recipe_builds_a_drone_from_parts", BuildGameTests::assemblyRecipe);
        reg("assembly_recipe_rejects_bad_grids", BuildGameTests::assemblyRejects);
        reg("part_recipes_craft_from_raw_materials", BuildGameTests::partRecipes);
        reg("workbench_menu_stays_valid_only_at_the_workbench", BuildGameTests::workbenchMenu);
        reg("build_changes_flight_performance", BuildGameTests::buildChangesPerformance);
        reg("antenna_tier_changes_signal_at_the_edge", BuildGameTests::antennaTier);
        reg("frame_tier_sets_crash_tolerance", BuildGameTests::frameTier);
        reg("blackbox_drops_a_flight_log_when_lost", BuildGameTests::blackboxDrops);
        reg("no_blackbox_means_no_log", BuildGameTests::noBlackbox);
        reg("blackbox_survives_a_detonation", BuildGameTests::blackboxSurvivesDetonation);
        reg("drone_item_carries_its_build", BuildGameTests::droneItemBuild);
        reg("tooltips_build_for_every_item", BuildGameTests::tooltips);
    }

    private BuildGameTests() {}

    private static void reg(String name, Consumer<GameTestHelper> fn) { FUNCTIONS.register(name, () -> fn); }
    public static void register(IEventBus bus) { FUNCTIONS.register(bus); }

    private static ItemStack part(PartKind k, int tier) { return new ItemStack(KsRegistries.part(k, tier)); }

    private static List<ItemStack> fullKit(int frame, int motor, int prop, int battery, int fc, int cam, int ant) {
        return List.of(part(PartKind.FRAME, frame), part(PartKind.MOTOR, motor), part(PartKind.PROP, prop), part(PartKind.BATTERY, battery),
                part(PartKind.FLIGHT_CONTROLLER, fc), part(PartKind.CAMERA, cam), part(PartKind.ANTENNA, ant));
    }

    private static List<ItemStack> with(List<ItemStack> base, ItemStack... extra) {
        java.util.ArrayList<ItemStack> l = new java.util.ArrayList<>(base);
        l.addAll(List.of(extra));
        return l;
    }

    private static CraftingInput grid(List<ItemStack> items) {
        java.util.ArrayList<ItemStack> padded = new java.util.ArrayList<>(items);
        while (padded.size() < 9) padded.add(ItemStack.EMPTY);
        return CraftingInput.of(3, 3, padded);
    }

    private static DroneEntity buildDrone(GameTestHelper h, DroneBuild b, double x, double y, double z) {
        DroneEntity d = KsGameTests.spawnDrone(h, x, y, z);
        d.setBuild(b);
        d.debugSpinUpToHover();
        return d;
    }

    // ---- recipes ------------------------------------------------------------------------------------
    /** The loaded assembly recipe turns a full kit (with a shaped-charge warhead and a blackbox) into a drone with that exact build. */
    private static void assemblyRecipe(GameTestHelper h) {
        List<ItemStack> kit = with(fullKit(3, 1, 2, 3, 2, 1, 3), part(PartKind.WARHEAD_SHAPED, 1), part(PartKind.BLACKBOX, 1));
        CraftingInput in = grid(kit);
        Optional<RecipeHolder<net.minecraft.world.item.crafting.CraftingRecipe>> r = h.getLevel().recipeAccess().getRecipeFor(RecipeType.CRAFTING, in, h.getLevel());
        h.assertTrue(r.isPresent(), Component.literal("the assembly recipe is not loaded or did not match"));
        ItemStack out = r.get().value().assemble(in);
        h.assertTrue(out.is(KsRegistries.DRONE_ITEM.get()), Component.literal("result is " + out));
        DroneBuild b = out.get(KsComponents.DRONE_BUILD.get());
        h.assertTrue(b != null && b.equals(new DroneBuild(3, 1, 2, 3, 2, 1, 3, Warhead.SHAPED, true)), Component.literal("wrong build: " + b));
        h.succeed();
    }

    private static void assemblyRejects(GameTestHelper h) {
        List<ItemStack> kit = fullKit(2, 2, 2, 2, 2, 2, 2);
        h.assertTrue(DroneAssemblyRecipe.parse(grid(kit)) != null, Component.literal("the plain kit should assemble"));
        h.assertTrue(DroneAssemblyRecipe.parse(grid(kit.subList(0, 6))) == null, Component.literal("a kit missing the antenna assembled"));
        h.assertTrue(DroneAssemblyRecipe.parse(grid(with(kit, new ItemStack(Items.STICK)))) == null, Component.literal("a stray stick was accepted"));
        h.assertTrue(DroneAssemblyRecipe.parse(grid(with(kit, part(PartKind.FRAME, 1)))) == null, Component.literal("two frames were accepted"));
        h.assertTrue(DroneAssemblyRecipe.parse(grid(with(kit, part(PartKind.WARHEAD_IMPACT, 1), part(PartKind.WARHEAD_SHAPED, 1)))) == null, Component.literal("two warheads were accepted"));
        h.assertTrue(DroneAssemblyRecipe.parse(grid(with(fullKit(2, 2, 2, 2, 1, 2, 2), part(PartKind.BLACKBOX, 1)))) == null, Component.literal("a blackbox with a tier 1 controller was accepted"));
        ItemStack two = part(PartKind.FRAME, 1); two.setCount(2);
        h.assertTrue(DroneAssemblyRecipe.parse(grid(with(kit.subList(1, 7), two))) == null, Component.literal("a stack of two frames was accepted"));
        h.succeed();
    }

    /** Every generated recipe loaded, and a few craft the right part from raw vanilla materials. */
    private static void partRecipes(GameTestHelper h) {
        ServerLevel level = h.getLevel();
        int missing = 0;
        StringBuilder names = new StringBuilder();
        for (String reg : KsRegistries.PARTS.keySet()) {
            ResourceKey<net.minecraft.world.item.crafting.Recipe<?>> key = ResourceKey.create(Registries.RECIPE, Identifier.fromNamespaceAndPath(KamikazeSkies.MODID, "part_" + reg));
            if (level.recipeAccess().byKey(key).isEmpty()) { missing++; names.append(reg).append(' '); }
        }
        h.assertTrue(missing == 0, Component.literal(missing + " part recipes did not load: " + names));

        check(h, List.of(new ItemStack(Items.STICK, 1), new ItemStack(Items.STICK, 1), new ItemStack(Items.STICK, 1), new ItemStack(Items.STICK, 1), new ItemStack(Items.IRON_INGOT)),
                KsRegistries.part(PartKind.FRAME, 1));
        check(h, List.of(new ItemStack(Items.GUNPOWDER), new ItemStack(Items.GUNPOWDER), new ItemStack(Items.GUNPOWDER), new ItemStack(Items.IRON_INGOT)),
                KsRegistries.part(PartKind.WARHEAD_IMPACT, 1));
        check(h, List.of(new ItemStack(Items.IRON_INGOT), new ItemStack(Items.REDSTONE), new ItemStack(Items.OBSIDIAN)), KsRegistries.part(PartKind.BLACKBOX, 1));
        check(h, List.of(new ItemStack(Items.CRAFTING_TABLE), new ItemStack(Items.IRON_INGOT), new ItemStack(Items.IRON_INGOT), new ItemStack(Items.REDSTONE), new ItemStack(Items.REDSTONE)),
                KsRegistries.WORKBENCH_ITEM.get());
        h.succeed();
    }

    private static void check(GameTestHelper h, List<ItemStack> items, Item expected) {
        CraftingInput in = grid(items);
        Optional<RecipeHolder<net.minecraft.world.item.crafting.CraftingRecipe>> r = h.getLevel().recipeAccess().getRecipeFor(RecipeType.CRAFTING, in, h.getLevel());
        h.assertTrue(r.isPresent() && r.get().value().assemble(in).is(expected), Component.literal("expected " + expected + " from " + items + ", got " + r.map(x -> x.value().assemble(in)).orElse(ItemStack.EMPTY)));
    }

    // ---- workbench ----------------------------------------------------------------------------------
    private static void workbenchMenu(GameTestHelper h) {
        BlockPos pos = new BlockPos(2, 1, 2);
        h.setBlock(pos, KsRegistries.WORKBENCH.get());
        ServerPlayer player = h.makeMockServerPlayerInLevel();
        player.snapTo(h.absoluteVec(new Vec3(2.5, 1, 4.5)));
        DroneWorkbenchBlock.WorkbenchMenu menu = new DroneWorkbenchBlock.WorkbenchMenu(1, player.getInventory(), ContainerLevelAccess.create(h.getLevel(), h.absolutePos(pos)));
        h.assertTrue(menu.stillValid(player), Component.literal("the menu should be valid next to the workbench"));
        h.setBlock(pos, Blocks.AIR);
        h.assertTrue(!menu.stillValid(player), Component.literal("the menu should close when the workbench is gone"));
        h.succeed();
    }

    // ---- the build drives the drone -------------------------------------------------------------------
    private static void buildChangesPerformance(GameTestHelper h) {
        DroneEntity racer = buildDrone(h, new DroneBuild(3, 3, 3, 1, 3, 3, 1, Warhead.NONE, false), -3.5, 20, 0.5);
        DroneEntity hauler = buildDrone(h, new DroneBuild(1, 1, 1, 3, 1, 1, 1, Warhead.SHAPED, true), 4.5, 20, 0.5);
        racer.debugSetInput(new FlightInput(1, 0, 0, 0, true, 1));
        hauler.debugSetInput(new FlightInput(1, 0, 0, 0, true, 1));
        KsGameTests.afterTicks(h, racer, 25, () -> {
            double racerClimb = KsGameTests.rel(h, racer).y - 20, haulerClimb = KsGameTests.rel(h, hauler).y - 20;
            h.assertTrue(hauler.simTicks() > 0, Component.literal("the second drone never ticked"));
            h.assertTrue(racerClimb > haulerClimb + 2.0, Component.literal("racer climbed " + racerClimb + " vs hauler " + haulerClimb));
            h.assertTrue(racer.sim().params().mass() < hauler.sim().params().mass(), Component.literal("masses: " + racer.sim().params().mass() + " vs " + hauler.sim().params().mass()));
        });
    }

    private static void antennaTier(GameTestHelper h) {
        double original = KsConfig.LINK_RANGE.getAsDouble();
        KsConfig.LINK_RANGE.set(60.0);              // shrink the radio range so 30 blocks is near the edge for a weak antenna
        DroneEntity[] d = new DroneEntity[3];
        for (int t = 1; t <= 3; t++) {
            d[t - 1] = buildDrone(h, DroneBuild.STANDARD.withTier(PartKind.ANTENNA, t), -6.5 + 6 * t, 20, 30.5);
            d[t - 1].debugOperator(h.absoluteVec(new Vec3(0.5, 20, 0.5)));
            d[t - 1].debugSetInput(new FlightInput(P.hoverStick(), 0, 0, 0, true, 1));
        }
        KsGameTests.afterTicks(h, d[0], 40, () -> {
            float r1 = d[0].rssi(), r2 = d[1].rssi(), r3 = d[2].rssi();
            KsConfig.LINK_RANGE.set(original);
            h.assertTrue(r1 < r2 && r2 < r3, Component.literal("signal should rise with antenna tier: " + r1 + ", " + r2 + ", " + r3));
            h.assertTrue(r3 >= 0.95f, Component.literal("the best antenna should still be strong: " + r3));
            h.assertTrue(r1 < 0.8f, Component.literal("the basic antenna should be weakened at the edge: " + r1));
        });
    }

    private static void frameTier(GameTestHelper h) {
        KsGameTests.floor(h, 0, 12);
        DroneEntity weak = buildDrone(h, DroneBuild.STANDARD.withTier(PartKind.FRAME, 1), -4.5, 7, 0.5);
        DroneEntity strong = buildDrone(h, DroneBuild.STANDARD.withTier(PartKind.FRAME, 3), 4.5, 7, 0.5);
        weak.debugSetInput(FlightInput.DISARMED);
        strong.debugSetInput(FlightInput.DISARMED);
        h.succeedWhen(() -> {
            h.assertTrue(weak.isRemoved(), Component.literal("a tier 1 frame should break in a 7 m fall"));
            h.assertTrue(!strong.isRemoved(), Component.literal("a tier 3 frame should survive a 7 m fall"));
        });
    }

    // ---- blackbox -----------------------------------------------------------------------------------
    private static FlightLog findLog(GameTestHelper h, double x, double z) {
        AABB area = new AABB(h.absoluteVec(new Vec3(x, 0, z)), h.absoluteVec(new Vec3(x, 0, z))).inflate(40, 60, 40);
        for (ItemEntity e : h.getLevel().getEntitiesOfClass(ItemEntity.class, area)) {
            FlightLog log = e.getItem().get(KsComponents.FLIGHT_LOG.get());
            if (log != null) return log;
        }
        return null;
    }

    private static void blackboxDrops(GameTestHelper h) {
        KsGameTests.floor(h, 0, 12);
        DroneEntity d = buildDrone(h, DroneBuild.STANDARD.withBlackbox(true), 0.5, 30, 0.5);
        d.debugSetInput(new FlightInput(P.hoverStick(), 0, 0, 0, true, 1));
        boolean[] dropped = {false};
        h.succeedWhen(() -> {
            if (!dropped[0]) {
                h.assertTrue(d.simTicks() >= 60, Component.literal("flying"));
                d.debugSetInput(FlightInput.DISARMED);                      // cut the motors: it falls and is lost
                dropped[0] = true;
            }
            h.assertTrue(d.isRemoved(), Component.literal("waiting for the crash"));
            FlightLog log = findLog(h, 0.5, 0.5);
            h.assertTrue(log != null, Component.literal("no flight log was dropped"));
            h.assertTrue(log.cause() == FlightLog.CRASH, Component.literal("cause " + log.cause()));
            h.assertTrue(log.flightSeconds() >= 2.5f, Component.literal("flight time " + log.flightSeconds()));
            h.assertTrue(log.samples().length >= 7 * 10, Component.literal("only " + log.samples().length + " sample ints"));
            h.assertTrue(log.maxSpeedKmh() > 40, Component.literal("max speed " + log.maxSpeedKmh()));
            h.assertTrue(log.minVolts() < 17 && log.minVolts() > 10, Component.literal("min volts " + log.minVolts()));
        });
    }

    private static void noBlackbox(GameTestHelper h) {
        KsGameTests.floor(h, 0, 12);
        DroneEntity d = buildDrone(h, DroneBuild.STANDARD, 0.5, 20, 0.5);
        d.debugSetInput(FlightInput.DISARMED);
        h.succeedWhen(() -> {
            h.assertTrue(d.isRemoved(), Component.literal("waiting for the crash"));
            h.assertTrue(findLog(h, 0.5, 0.5) == null, Component.literal("a drone without a blackbox left a flight log"));
        });
    }

    private static void blackboxSurvivesDetonation(GameTestHelper h) {
        KsGameTests.floor(h, 0, 12);
        for (int x = -5; x <= 5; x++) for (int y = 1; y <= 7; y++) h.setBlock(new BlockPos(x, y, 16), Blocks.OAK_PLANKS);
        DroneEntity d = buildDrone(h, DroneBuild.STANDARD.withWarhead(Warhead.IMPACT).withBlackbox(true), 0.5, 4.0, 4.5);
        double tilt = Math.toRadians(0.5 * P.maxAngleDeg());
        d.debugSetInput(new FlightInput(d.sim().params().stickForThrustFraction(d.sim().params().hoverThrustFraction() / Math.cos(tilt) * 1.03), 0, 0.5, 0, true, 1));
        h.succeedWhen(() -> {
            h.assertTrue(d.isRemoved(), Component.literal("drone has not hit the wall yet: pos=" + KsGameTests.rel(h, d) + " vel=" + d.sim().velocity() + " armed=" + d.isArmed()
                    + " onGround=" + d.onGround() + " simTicks=" + d.simTicks() + " power=" + d.sim().powerScale() + " volts=" + d.voltage() + " rssi=" + d.rssi()));
            FlightLog log = findLog(h, 0.5, 16.5);
            h.assertTrue(log != null, Component.literal("the blackbox did not survive the blast"));
            h.assertTrue(log.cause() == FlightLog.WARHEAD, Component.literal("cause " + log.cause()));
        });
    }

    private static void droneItemBuild(GameTestHelper h) {
        DroneBuild b = new DroneBuild(3, 2, 1, 3, 2, 1, 3, Warhead.SHAPED, true);
        ItemStack s = KsRegistries.droneStack(b);
        h.assertTrue(b.equals(s.get(KsComponents.DRONE_BUILD.get())), Component.literal("the stack lost its build"));
        h.assertTrue(DroneBuild.STANDARD.equals(new ItemStack(KsRegistries.DRONE_ITEM.get()).get(KsComponents.DRONE_BUILD.get())), Component.literal("a bare drone item should default to the standard build"));
        h.succeed();
    }

    /** Every part, the drone and a recovered blackbox build their tooltips without errors. */
    private static void tooltips(GameTestHelper h) {
        java.util.List<ItemStack> all = new java.util.ArrayList<>();
        for (var part : KsRegistries.PARTS.values()) all.add(new ItemStack(part.get()));
        all.add(KsRegistries.droneStack(new DroneBuild(3, 2, 1, 3, 2, 1, 3, Warhead.SHAPED, true)));
        ItemStack box = part(PartKind.BLACKBOX, 1);
        com.kamikazeskies.build.FlightRecorder rec = new com.kamikazeskies.build.FlightRecorder();
        for (int t = 0; t < 100; t++) rec.record(t, t, 70, 0, 12, 15.5, 0.9, 64);
        box.set(KsComponents.FLIGHT_LOG.get(), FlightLog.of(FlightLog.CRASH, rec));
        all.add(box);
        for (ItemStack s : all) {
            java.util.List<Component> lines = new java.util.ArrayList<>();
            s.getItem().appendHoverText(s, Item.TooltipContext.EMPTY, net.minecraft.world.item.component.TooltipDisplay.DEFAULT, lines::add, net.minecraft.world.item.TooltipFlag.NORMAL);
            h.assertTrue(lines.size() >= 2, Component.literal(s.getItem() + " built only " + lines.size() + " tooltip lines"));
        }
        h.succeed();
    }
}
