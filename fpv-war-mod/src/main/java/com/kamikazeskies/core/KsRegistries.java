package com.kamikazeskies.core;

import com.kamikazeskies.drone.DroneEntity;
import com.kamikazeskies.drone.DroneItem;

import net.minecraft.core.registries.Registries;
import net.minecraft.network.chat.Component;
import net.minecraft.world.entity.EntityType;
import net.minecraft.world.entity.MobCategory;
import net.minecraft.world.item.CreativeModeTab;
import net.minecraft.world.item.Item;
import net.neoforged.bus.api.IEventBus;
import net.neoforged.neoforge.registries.DeferredHolder;
import net.neoforged.neoforge.registries.DeferredItem;
import net.neoforged.neoforge.registries.DeferredRegister;

/** All registry entries of the mod. */
public final class KsRegistries {
    public static final DeferredRegister.Entities ENTITIES = DeferredRegister.createEntities(KamikazeSkies.MODID);
    public static final DeferredRegister.Items ITEMS = DeferredRegister.createItems(KamikazeSkies.MODID);
    public static final DeferredRegister<CreativeModeTab> TABS = DeferredRegister.create(Registries.CREATIVE_MODE_TAB, KamikazeSkies.MODID);

    /** 5" FPV quadcopter. Tiny hitbox, tracked every tick for smooth motion. */
    public static final DeferredHolder<EntityType<?>, EntityType<DroneEntity>> DRONE = ENTITIES.registerEntityType(
            "fpv_drone", DroneEntity::new, MobCategory.MISC,
            b -> b.sized(0.30f, 0.12f).eyeHeight(0.06f).clientTrackingRange(16).updateInterval(1));

    public static final DeferredItem<Item> DRONE_ITEM = ITEMS.registerItem(
            "fpv_drone", DroneItem::new, p -> p.stacksTo(1));

    public static final DeferredHolder<CreativeModeTab, CreativeModeTab> MAIN_TAB = TABS.register("main", () ->
            CreativeModeTab.builder()
                    .title(Component.translatable("itemGroup.kamikaze_skies"))
                    .icon(() -> DRONE_ITEM.get().getDefaultInstance())
                    .displayItems((params, output) -> output.accept(DRONE_ITEM.get()))
                    .build());

    private KsRegistries() {}

    public static void register(IEventBus bus) {
        ENTITIES.register(bus);
        ITEMS.register(bus);
        TABS.register(bus);
    }
}
