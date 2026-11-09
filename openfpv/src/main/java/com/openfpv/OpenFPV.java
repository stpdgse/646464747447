package com.openfpv;

import com.openfpv.bay.GuiHandler;
import com.openfpv.net.Net;
import com.openfpv.oc.CameraRegistry;
import com.openfpv.oc.DriverCamera;
import com.openfpv.server.ViewerManager;
import li.cil.oc.api.Driver;
import net.minecraftforge.common.MinecraftForge;
import net.minecraftforge.fml.common.Mod;
import net.minecraftforge.fml.common.event.FMLInitializationEvent;
import net.minecraftforge.fml.common.event.FMLPreInitializationEvent;
import net.minecraftforge.fml.common.event.FMLServerStoppedEvent;
import net.minecraftforge.fml.common.network.NetworkRegistry;
import org.apache.logging.log4j.Logger;

@Mod(modid = OpenFPV.MOD_ID, name = OpenFPV.NAME, version = OpenFPV.VERSION,
     dependencies = "required-after:opencomputers@[1.7.5,)",
     acceptedMinecraftVersions = "[1.12.2]")
public class OpenFPV {
    public static final String MOD_ID = "openfpv";
    public static final String NAME = "OpenFPV";
    public static final String VERSION = "1.0.0";

    @Mod.Instance(MOD_ID)
    public static OpenFPV instance;

    public static Logger log;

    @Mod.EventHandler
    public void preInit(FMLPreInitializationEvent event) {
        log = event.getModLog();
        Config.load(event.getSuggestedConfigurationFile());
        Net.init();
        NetworkRegistry.INSTANCE.registerGuiHandler(this, new GuiHandler());
        MinecraftForge.EVENT_BUS.register(new ViewerManager());
        if (event.getSide().isClient()) {
            com.openfpv.client.ClientBootstrap.preInit();
        }
    }

    @Mod.EventHandler
    public void init(FMLInitializationEvent event) {
        Driver.add(new DriverCamera());
        if (event.getSide().isClient()) {
            com.openfpv.client.ClientBootstrap.init();
        }
        log.info("OpenFPV {} ready: camera driver registered with OpenComputers", VERSION);
    }

    @Mod.EventHandler
    public void serverStopped(FMLServerStoppedEvent event) {
        CameraRegistry.clear();
        ViewerManager.reset();
    }
}
