package com.kamikazeskies.core;

import net.neoforged.neoforge.common.ModConfigSpec;

/** Server-authoritative configuration, synced to clients (ModConfig.Type.SYNCED). Grows as systems are added. */
public final class KsConfig {
    private static final ModConfigSpec.Builder BUILDER = new ModConfigSpec.Builder();

    public static final ModConfigSpec.IntValue FLIGHT_SUBSTEPS = BUILDER
            .comment("Flight simulation substeps per game tick (4 = 80 Hz).")
            .defineInRange("flightSubsteps", 4, 1, 16);

    public static final ModConfigSpec.DoubleValue CRASH_SPEED = BUILDER
            .comment("Impact speed (m/s) at which a drone is destroyed.")
            .defineInRange("crashSpeed", 9.0, 1.0, 100.0);

    static final ModConfigSpec SPEC = BUILDER.build();

    private KsConfig() {}
}
