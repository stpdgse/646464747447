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
        ClientRegistry.registerKeyBinding(FpvClient.KEY_YAW_LEFT);
        ClientRegistry.registerKeyBinding(FpvClient.KEY_YAW_RIGHT);
        // Developer self test (never set in normal play); see build.gradle -Popenfpv.autotest
        if (System.getProperty("openfpv.autotest") != null) {
            com.openfpv.dev.DevHarness.install();
        }
    }
}
