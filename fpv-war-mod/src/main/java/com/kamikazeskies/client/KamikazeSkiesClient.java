package com.kamikazeskies.client;

import com.kamikazeskies.core.KamikazeSkies;

import net.neoforged.api.distmarker.Dist;
import net.neoforged.bus.api.SubscribeEvent;
import net.neoforged.fml.ModContainer;
import net.neoforged.fml.common.EventBusSubscriber;
import net.neoforged.fml.common.Mod;
import net.neoforged.fml.event.lifecycle.FMLClientSetupEvent;
import net.neoforged.neoforge.client.gui.ConfigurationScreen;
import net.neoforged.neoforge.client.gui.IConfigScreenFactory;

/** Client-only entry point; never loaded on a dedicated server. */
@Mod(value = KamikazeSkies.MODID, dist = Dist.CLIENT)
@EventBusSubscriber(modid = KamikazeSkies.MODID, value = Dist.CLIENT)
public class KamikazeSkiesClient {
    public KamikazeSkiesClient(ModContainer container) {
        container.registerExtensionPoint(IConfigScreenFactory.class, ConfigurationScreen::new);
    }

    @SubscribeEvent
    static void onClientSetup(FMLClientSetupEvent event) {
        KamikazeSkies.LOGGER.info("Kamikaze Skies client setup");
    }
}
