package com.openfpv;

import java.io.File;
import net.minecraftforge.common.config.Configuration;

/** Server/common configuration (config/openfpv.cfg). */
public final class Config {
    private Config() {}

    /**
     * Maximum distance in blocks between pilot and drone before the video signal is lost.
     * OpenComputers registers drones with an entity tracking range of 80 blocks, so the client
     * cannot see a drone any further away than that; the config is capped accordingly.
     */
    public static double maxRange = 64.0;
    /** Energy (OC units) the camera draws from the drone every tick while active. */
    public static double energyPerTick = 0.1;
    /** Extra energy per ray for depth/color scans. */
    public static double depthEnergyPerRay = 0.02;
    /** How far ahead (blocks) the keyboard-flight target is placed. */
    public static double flightLead = 4.0;
    public static int depthMaxWidth = 32;
    public static int depthMaxHeight = 16;
    public static double depthMaxDistance = 64.0;
    /** Analog static / scanlines on weak signal. */
    public static boolean noiseEnabled = true;

    public static void load(File file) {
        Configuration c = new Configuration(file);
        c.load();
        maxRange = c.get("camera", "maxRange", maxRange,
            "Max pilot-to-drone distance (blocks). Capped at 80 because OpenComputers drones are only tracked by "
                + "clients within 80 blocks; also limited by your render distance on the client.", 16.0, 80.0).getDouble();
        energyPerTick = c.get("camera", "energyPerTick", energyPerTick,
            "Energy the active camera draws from the drone each tick.", 0.0, 100.0).getDouble();
        depthEnergyPerRay = c.get("depth", "energyPerRay", depthEnergyPerRay,
            "Energy per ray for camera.depth()/camera.color().", 0.0, 10.0).getDouble();
        flightLead = c.get("flight", "lead", flightLead,
            "Distance (blocks) the flight target is placed ahead of the drone while a movement key is held.", 1.0, 16.0).getDouble();
        depthMaxWidth = c.get("depth", "maxWidth", depthMaxWidth, "Max grid width for depth scans.", 1, 64).getInt();
        depthMaxHeight = c.get("depth", "maxHeight", depthMaxHeight, "Max grid height for depth scans.", 1, 64).getInt();
        depthMaxDistance = c.get("depth", "maxDistance", depthMaxDistance, "Max ray length for depth scans.", 4.0, 256.0).getDouble();
        noiseEnabled = c.get("client", "noise", noiseEnabled, "Show static/scanlines on weak signal.").getBoolean();
        if (c.hasChanged()) c.save();
    }
}
