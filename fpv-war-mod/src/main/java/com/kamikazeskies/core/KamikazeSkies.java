package com.kamikazeskies.core;

import org.slf4j.Logger;

import com.mojang.logging.LogUtils;

import com.kamikazeskies.gametest.KsGameTests;

import com.kamikazeskies.drone.PilotRegistry;
import com.kamikazeskies.net.KsNetwork;

import net.neoforged.bus.api.IEventBus;
import net.neoforged.fml.ModContainer;
import net.neoforged.fml.common.Mod;
import net.neoforged.fml.config.ModConfig;
import net.neoforged.fml.event.lifecycle.FMLCommonSetupEvent;
import net.neoforged.neoforge.common.NeoForge;
import net.neoforged.neoforge.event.entity.player.PlayerEvent;
import net.neoforged.neoforge.event.server.ServerStoppedEvent;

/** Mod entry point. Common (client + server) bootstrap only; client-only code lives in {@code client/}. */
@Mod(KamikazeSkies.MODID)
public class KamikazeSkies {
    public static final String MODID = "kamikaze_skies";
    public static final Logger LOGGER = LogUtils.getLogger();

    public KamikazeSkies(IEventBus modEventBus, ModContainer modContainer) {
        KsRegistries.register(modEventBus);
        KsGameTests.register(modEventBus);
        modEventBus.addListener(KsNetwork::register);
        modEventBus.addListener(this::commonSetup);
        NeoForge.EVENT_BUS.addListener((PlayerEvent.PlayerLoggedOutEvent e) -> {
            if (e.getEntity() instanceof net.minecraft.server.level.ServerPlayer sp) PilotRegistry.releasePlayer(sp);
        });
        NeoForge.EVENT_BUS.addListener((ServerStoppedEvent e) -> PilotRegistry.clear());
        if (com.kamikazeskies.drone.DemoMode.ENABLED) {
            NeoForge.EVENT_BUS.addListener((PlayerEvent.PlayerLoggedInEvent e) -> {
                if (e.getEntity() instanceof net.minecraft.server.level.ServerPlayer sp) com.kamikazeskies.drone.DemoMode.start(sp);
            });
        }
        modContainer.registerConfig(ModConfig.Type.SYNCED, KsConfig.SPEC);
    }

    private void commonSetup(FMLCommonSetupEvent event) {
        // SYNCED config values are per-world and only readable once a world/server exists, not here.
        LOGGER.info("Kamikaze Skies common setup");
    }
}
