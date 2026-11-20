package com.kamikazeskies.core;

import java.util.LinkedHashMap;
import java.util.Map;

import com.kamikazeskies.drone.DroneEntity;
import com.kamikazeskies.build.DroneBuild;
import com.kamikazeskies.build.PartKind;
import com.kamikazeskies.drone.DroneItem;
import com.kamikazeskies.drone.JammerEntity;
import com.kamikazeskies.drone.JammerItem;
import com.kamikazeskies.drone.SoldierEntity;
import com.kamikazeskies.item.DroneAssemblyRecipe;
import com.kamikazeskies.item.DroneWorkbenchBlock;
import com.kamikazeskies.item.KsComponents;
import com.kamikazeskies.item.PartItem;

import net.minecraft.core.registries.Registries;
import net.minecraft.world.item.ItemStack;
import net.minecraft.world.item.crafting.RecipeSerializer;
import net.minecraft.world.level.block.Block;
import net.minecraft.world.level.block.SoundType;
import net.minecraft.network.chat.Component;
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.entity.MobCategory;
import net.minecraft.world.item.CreativeModeTab;
import net.minecraft.world.item.Item;
import net.neoforged.bus.api.IEventBus;
import net.neoforged.neoforge.registries.DeferredHolder;
import net.neoforged.neoforge.registries.DeferredBlock;
import net.neoforged.neoforge.registries.DeferredItem;
import net.neoforged.neoforge.registries.DeferredRegister;

/** All registry entries of the mod. */
public final class KsRegistries {
    public static final DeferredRegister.Entities ENTITIES = DeferredRegister.createEntities(KamikazeSkies.MODID);
    public static final DeferredRegister.Blocks BLOCKS = DeferredRegister.createBlocks(KamikazeSkies.MODID);
    public static final DeferredRegister<RecipeSerializer<?>> RECIPE_SERIALIZERS = DeferredRegister.create(Registries.RECIPE_SERIALIZER, KamikazeSkies.MODID);
    public static final DeferredRegister.Items ITEMS = DeferredRegister.createItems(KamikazeSkies.MODID);
    public static final DeferredRegister<CreativeModeTab> TABS = DeferredRegister.create(Registries.CREATIVE_MODE_TAB, KamikazeSkies.MODID);

    /** 5" FPV quadcopter. Tiny hitbox, tracked every tick for smooth motion. */
    public static final DeferredHolder<EntityType<?>, EntityType<DroneEntity>> DRONE = ENTITIES.registerEntityType(
            "fpv_drone", DroneEntity::new, MobCategory.MISC,
            b -> b.sized(0.30f, 0.12f).eyeHeight(0.06f).clientTrackingRange(16).updateInterval(1));

    /** Anti-drone jammer mast. */
    public static final DeferredHolder<EntityType<?>, EntityType<JammerEntity>> JAMMER = ENTITIES.registerEntityType(
            "jammer", JammerEntity::new, MobCategory.MISC,
            b -> b.sized(0.6f, 1.5f).clientTrackingRange(10).updateInterval(20));

    /** Rifleman. Standard human hitbox; spawned by the AI/mission layer or {@code /kamikaze soldier}. */
    public static final DeferredHolder<EntityType<?>, EntityType<SoldierEntity>> SOLDIER = ENTITIES.registerEntityType(
            "soldier", SoldierEntity::new, MobCategory.MISC,
            b -> b.sized(0.6f, 1.8f).eyeHeight(1.62f).clientTrackingRange(10).updateInterval(2));

    public static final DeferredItem<Item> JAMMER_ITEM = ITEMS.registerItem("jammer", JammerItem::new, p -> p.stacksTo(16));

    /** The drone. Its parts (see {@link DroneBuild}) are stored in the {@code drone_build} component. */
    public static final DeferredItem<Item> DRONE_ITEM = ITEMS.registerItem("fpv_drone", DroneItem::new,
            p -> p.stacksTo(1).component(KsComponents.DRONE_BUILD.get(), DroneBuild.STANDARD));

    /** Craftable parts: tiers 1-3 of seven kinds, plus warheads and the blackbox. Keyed by "kind" or "kind_tN". */
    public static final Map<String, DeferredItem<Item>> PARTS = new LinkedHashMap<>();

    static {
        for (PartKind kind : PartKind.values()) {
            for (int tier = 1; tier <= (kind.tiered ? PartKind.MAX_TIER : 1); tier++) {
                final int t = tier;
                PARTS.put(partName(kind, tier), ITEMS.registerItem("part_" + partName(kind, tier), p -> new PartItem(p, kind, t), p -> p.stacksTo(kind == PartKind.BLACKBOX ? 1 : 16)));
            }
        }
    }

    public static String partName(PartKind kind, int tier) { return kind.tiered ? kind.id + "_t" + tier : kind.id; }
    public static Item part(PartKind kind, int tier) { return PARTS.get(partName(kind, tier)).get(); }

    /** The Drone Workbench, where drones are assembled from parts. */
    public static final DeferredBlock<Block> WORKBENCH = BLOCKS.registerBlock("drone_workbench", DroneWorkbenchBlock::new, p -> p.strength(2.5f).sound(SoundType.WOOD));
    public static final DeferredItem<net.minecraft.world.item.BlockItem> WORKBENCH_ITEM = ITEMS.registerSimpleBlockItem("drone_workbench", WORKBENCH, p -> p);

    public static final DeferredHolder<RecipeSerializer<?>, RecipeSerializer<DroneAssemblyRecipe>> ASSEMBLY_RECIPE =
            RECIPE_SERIALIZERS.register("drone_assembly", () -> DroneAssemblyRecipe.SERIALIZER);

    /** A drone item with the given build. */
    public static ItemStack droneStack(DroneBuild build) {
        ItemStack s = new ItemStack(DRONE_ITEM.get());
        s.set(KsComponents.DRONE_BUILD.get(), build);
        return s;
    }

    public static final DeferredHolder<CreativeModeTab, CreativeModeTab> MAIN_TAB = TABS.register("main", () ->
            CreativeModeTab.builder()
                    .title(Component.translatable("itemGroup.kamikaze_skies"))
                    .icon(() -> DRONE_ITEM.get().getDefaultInstance())
                    .displayItems((params, output) -> {
                        output.accept(WORKBENCH_ITEM.get());
                        output.accept(JAMMER_ITEM.get());
                        output.accept(droneStack(DroneBuild.STANDARD));
                        output.accept(droneStack(DroneBuild.STANDARD.withWarhead(com.kamikazeskies.drone.Warhead.IMPACT)));
                        output.accept(droneStack(DroneBuild.STANDARD.withWarhead(com.kamikazeskies.drone.Warhead.SHAPED)));
                        output.accept(droneStack(new DroneBuild(3, 3, 3, 3, 3, 3, 3, com.kamikazeskies.drone.Warhead.IMPACT, true)));
                        for (DeferredItem<Item> part : PARTS.values()) output.accept(part.get());
                    })
                    .build());

    private KsRegistries() {}

    public static void register(IEventBus bus) {
        ENTITIES.register(bus);
        BLOCKS.register(bus);
        RECIPE_SERIALIZERS.register(bus);
        ITEMS.register(bus);
        TABS.register(bus);
    }
}
