package com.openfpv.client;

import net.minecraftforge.common.MinecraftForge;
import net.minecraftforge.fml.client.registry.ClientRegistry;
import net.minecraftforge.fml.relauncher.Side;
import net.minecraftforge.fml.relauncher.SideOnly;

/** Client-only start-up. Only ever called behind a side check in {@link com.openfpv.OpenFPV}. */
@SideOnly(Side.CLIENT)
public final class ClientBootstrap {
    private ClientBootstrap() {}

    public static void preInit() {
        MinecraftForge.EVENT_BUS.register(new FpvEvents());
    }

    public static void init() {
        ClientRegistry.registerKeyBinding(FpvClient.KEY_TOGGLE);
        ClientRegistry.registerKeyBinding(FpvClient.KEY_NEXT);
        ClientRegistry.registerKeyBinding(FpvClient.KEY_MODE);
        ClientRegistry.registerKeyBinding(FpvClient.KEY_RECORD);
        ClientRegistry.registerKeyBinding(FpvClient.KEY_YAW_LEFT);
        ClientRegistry.registerKeyBinding(FpvClient.KEY_YAW_RIGHT);
        // Developer self test (never set in normal play); see build.gradle -Popenfpv.autotest
        if (System.getProperty("openfpv.autotest") != null) {
            com.openfpv.dev.DevHarness.install();
        }
    }

    /** Replaces OpenComputers' drone renderer (registered earlier) with the animated Blockbench quadcopter. */
    public static void postInit() {
        if (!com.openfpv.Config.customDroneModel) return;
        try {
            Class<?> droneClass = Class.forName("li.cil.oc.common.entity.Drone");
            net.minecraft.client.renderer.entity.RenderManager rm = net.minecraft.client.Minecraft.getMinecraft().getRenderManager();
            rm.entityRenderMap.put((Class) droneClass, new DroneRenderer(rm));
            com.openfpv.OpenFPV.log.info("Drone renderer replaced with the OpenFPV quadcopter model");
        } catch (Throwable t) {
            com.openfpv.OpenFPV.log.warn("Could not replace the drone renderer: {}", t.toString());
        }
    }
}
