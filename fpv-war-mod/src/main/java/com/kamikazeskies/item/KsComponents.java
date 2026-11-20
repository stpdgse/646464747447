package com.kamikazeskies.item;

import com.kamikazeskies.build.DroneBuild;
import com.kamikazeskies.core.KamikazeSkies;
import com.mojang.serialization.Codec;

import net.minecraft.core.component.DataComponentType;
import net.minecraft.core.registries.Registries;
import net.minecraft.network.codec.ByteBufCodecs;
import net.neoforged.bus.api.IEventBus;
import net.neoforged.neoforge.registries.DeferredHolder;
import net.neoforged.neoforge.registries.DeferredRegister;

/** Data components carried by our items. */
public final class KsComponents {
    public static final DeferredRegister.DataComponents COMPONENTS = DeferredRegister.createDataComponents(Registries.DATA_COMPONENT_TYPE, KamikazeSkies.MODID);

    /** The parts a drone item was assembled from, stored as a compact integer. */
    public static final DeferredHolder<DataComponentType<?>, DataComponentType<DroneBuild>> DRONE_BUILD = COMPONENTS.registerComponentType("drone_build",
            b -> b.persistent(Codec.INT.xmap(DroneBuild::decode, DroneBuild::encode)).networkSynchronized(ByteBufCodecs.VAR_INT.map(DroneBuild::decode, DroneBuild::encode)));

    /** Telemetry recovered by a blackbox. */
    public static final DeferredHolder<DataComponentType<?>, DataComponentType<FlightLog>> FLIGHT_LOG = COMPONENTS.registerComponentType("flight_log",
            b -> b.persistent(FlightLog.CODEC).networkSynchronized(FlightLog.STREAM_CODEC));

    private KsComponents() {}

    public static void register(IEventBus bus) { COMPONENTS.register(bus); }
}
