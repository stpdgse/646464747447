package com.kamikazeskies.core;

import com.kamikazeskies.destruction.core.QualityPreset;

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

    public static final ModConfigSpec.EnumValue<QualityPreset> QUALITY = BUILDER
            .comment("Destruction quality preset: LOW, MEDIUM or HIGH. Limits blast rays, blocks changed per tick and debris entities.")
            .defineEnum("destructionQuality", QualityPreset.HIGH);

    public static final ModConfigSpec.DoubleValue DETONATION_SPEED = BUILDER
            .comment("Impact speed (m/s) at which a warhead's impact fuze fires.")
            .defineInRange("detonationSpeed", 3.0, 0.5, 50.0);

    public static final ModConfigSpec.DoubleValue LINK_RANGE = BUILDER
            .comment("Radio range of the standard antenna, in blocks. Signal fades with distance, blocks in the way and jammers.")
            .defineInRange("linkRangeBlocks", 400.0, 50.0, 5000.0);

    public static final ModConfigSpec.BooleanValue BATTERY_ENABLED = BUILDER
            .comment("Drain the battery while flying. Turn off for practice or creative play.")
            .define("batteryEnabled", true);

    static final ModConfigSpec SPEC = BUILDER.build();

    private KsConfig() {}
}
