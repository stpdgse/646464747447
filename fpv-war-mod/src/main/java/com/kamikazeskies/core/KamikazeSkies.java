package com.kamikazeskies.core;

import org.slf4j.Logger;

import com.mojang.logging.LogUtils;

import com.kamikazeskies.gametest.KsGameTests;

import net.neoforged.bus.api.IEventBus;
import net.neoforged.fml.ModContainer;
import net.neoforged.fml.common.Mod;
import net.neoforged.fml.config.ModConfig;
import net.neoforged.fml.event.lifecycle.FMLCommonSetupEvent;

/** Mod entry point. Common (client + server) bootstrap only; client-only code lives in {@code client/}. */
@Mod(KamikazeSkies.MODID)
public class KamikazeSkies {
    public static final String MODID = "kamikaze_skies";
    public static final Logger LOGGER = LogUtils.getLogger();

    public KamikazeSkies(IEventBus modEventBus, ModContainer modContainer) {
        KsGameTests.register(modEventBus);
        modEventBus.addListener(this::commonSetup);
        modContainer.registerConfig(ModConfig.Type.SYNCED, KsConfig.SPEC);
    }

    private void commonSetup(FMLCommonSetupEvent event) {
        // SYNCED config values are per-world and only readable once a world/server exists, not here.
        LOGGER.info("Kamikaze Skies common setup");
    }
}
