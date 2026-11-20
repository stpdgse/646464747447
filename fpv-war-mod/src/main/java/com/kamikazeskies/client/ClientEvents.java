package com.kamikazeskies.client;

import com.kamikazeskies.core.KamikazeSkies;

import net.minecraft.resources.Identifier;
import net.neoforged.api.distmarker.Dist;
import net.neoforged.bus.api.SubscribeEvent;
import net.neoforged.fml.common.EventBusSubscriber;
import net.neoforged.neoforge.client.event.ClientTickEvent;
import net.neoforged.neoforge.client.event.EntityRenderersEvent;
import net.neoforged.neoforge.client.event.RegisterGuiLayersEvent;
import net.neoforged.neoforge.client.event.RegisterKeyMappingsEvent;
import net.neoforged.neoforge.client.event.RenderGuiLayerEvent;
import net.neoforged.neoforge.client.event.RenderHandEvent;
import net.neoforged.neoforge.client.event.ViewportEvent;

/** Client-only event wiring. */
public final class ClientEvents {
    private ClientEvents() {}

    @EventBusSubscriber(modid = KamikazeSkies.MODID, value = Dist.CLIENT)
    public static final class Mod {
        @SubscribeEvent
        static void onRegisterKeys(RegisterKeyMappingsEvent event) { PilotKeys.register(event); }

        @SubscribeEvent
        static void onRegisterRenderers(EntityRenderersEvent.RegisterRenderers event) {
            event.registerEntityRenderer(com.kamikazeskies.core.KsRegistries.DRONE.get(), DroneRenderer::new);
            event.registerEntityRenderer(com.kamikazeskies.core.KsRegistries.JAMMER.get(), JammerRenderer::new);
            event.registerEntityRenderer(com.kamikazeskies.core.KsRegistries.SOLDIER.get(), SoldierRenderer::new);
            event.registerEntityRenderer(com.kamikazeskies.core.KsRegistries.TANK.get(), TankRenderer::new);
        }

        @SubscribeEvent
        static void onRegisterGuiLayers(RegisterGuiLayersEvent event) {
            event.registerAboveAll(Identifier.fromNamespaceAndPath(KamikazeSkies.MODID, "fpv_osd"), new FpvOsd());
        }
    }

    @EventBusSubscriber(modid = KamikazeSkies.MODID, value = Dist.CLIENT)
    public static final class Game {
        @SubscribeEvent
        static void onClientTick(ClientTickEvent.Post event) { ClientPilot.tick(); FpvPostEffects.update(); }

        @SubscribeEvent
        static void onCameraAngles(ViewportEvent.ComputeCameraAngles event) { FpvView.onCameraAngles(event); }

        @SubscribeEvent
        static void onFov(ViewportEvent.ComputeFov event) { FpvView.onFov(event); }

        @SubscribeEvent
        static void onGuiLayer(RenderGuiLayerEvent.Pre event) { FpvView.onGuiLayer(event); }

        @SubscribeEvent
        static void onRenderHand(RenderHandEvent event) { FpvView.onRenderHand(event); }
    }
}
