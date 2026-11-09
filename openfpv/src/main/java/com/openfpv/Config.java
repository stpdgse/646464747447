package com.openfpv;

import java.io.File;
import net.minecraftforge.common.config.Configuration;

/** Server/common configuration (config/openfpv.cfg). */
public final class Config {
    private Config() {}

    // ---- range / antennas
    /** Signal range in blocks per antenna tier: index 0 = no antenna, 1..4 = Antenna Mk1..Mk4. */
    public static double[] rangeTiers = {64.0, 160.0, 400.0, 800.0, 1500.0};
    /** Energy (OC units) the camera draws from the drone every tick while active. */
    public static double energyPerTick = 0.1;
    /** Extra energy per ray for depth/color scans. */
    public static double depthEnergyPerRay = 0.02;
    public static int depthMaxWidth = 32;
    public static int depthMaxHeight = 16;
    public static double depthMaxDistance = 64.0;
    /** Analog static / scanlines on weak signal. */
    public static boolean noiseEnabled = true;

    // ---- quadcopter flight model (angle mode)
    /** Maximum bank/pitch angle in degrees at full stick. */
    public static double maxTilt = 35.0;
    /** Fraction of the remaining attitude error closed each tick (higher = snappier). */
    public static double attitudeResponse = 0.2;
    /** Horizontal acceleration (blocks/tick^2) per unit of sin(tilt). */
    public static double accelScale = 0.12;
    /** Vertical acceleration (blocks/tick^2) at full throttle. */
    public static double throttleAccel = 0.05;
    /** Yaw rate in degrees per tick at full stick. */
    public static double yawRate = 4.5;
    /** Air drag while piloting (per-tick velocity multiplier; OC's own is 0.8, higher = longer glide). */
    public static double fpvDrag = 0.9;
    /** Multiplies OC's drone speed cap (8 m/s) while somebody is piloting it. */
    public static double speedMultiplier = 2.5;
    /** FPV cameras are mounted tilted up by this many degrees, like on a real racing quad. */
    public static double cameraUptilt = 15.0;

    /** Rate of rotation in acro mode, degrees per tick at full stick. */
    public static double acroRate = 12.0;
    /** If true the pilot must wear FPV Goggles in the helmet slot to fly. */
    public static boolean requireGoggles = false;

    // ---- control link
    public static boolean controlLag = true;
    /** Control delay in ticks at the very edge of the range. */
    public static int maxLagTicks = 10;
    /** Fraction of control packets lost at the very edge of the range. */
    public static double maxPacketLoss = 0.3;

    // ---- failsafe
    /** On signal loss the drone flies back to its home point instead of just hovering. */
    public static boolean returnOnSignalLoss = true;
    /** Battery fraction below which an active camera drone automatically returns home (0 = off). */
    public static double lowBatteryReturn = 0.15;

    // ---- long range streaming
    /** Chunks (radius) around the drone that are streamed to the pilot, capped by the server view distance. */
    public static int streamRadius = 5;
    /** Also sync entities near the drone to the pilot, however far away they are. */
    public static boolean boostEntities = true;
    public static double boostRadius = 96.0;

    public static double rangeForTier(int tier) {
        int t = Math.max(0, Math.min(tier, rangeTiers.length - 1));
        return rangeTiers[t];
    }

    public static int maxTier() {
        return rangeTiers.length - 1;
    }

    /** The live configuration (also edited by the in-game config screen). */
    public static Configuration cfg;

    public static void load(File file) {
        cfg = new Configuration(file);
        cfg.load();
        read();
    }

    /** Copies the values from the configuration file into the static fields. */
    public static void read() {
        Configuration c = cfg;
        double[] tiers = c.get("range", "tiers", rangeTiers,
            "Signal range (blocks) with no antenna, then Antenna Mk1..Mk4. Five values, ascending.", 16.0, 20000.0).getDoubleList();
        if (tiers.length == 5) rangeTiers = tiers;
        energyPerTick = c.get("camera", "energyPerTick", energyPerTick,
            "Energy the active camera draws from the drone each tick.", 0.0, 100.0).getDouble();
        depthEnergyPerRay = c.get("depth", "energyPerRay", depthEnergyPerRay,
            "Energy per ray for camera.depth()/camera.color().", 0.0, 10.0).getDouble();
        depthMaxWidth = c.get("depth", "maxWidth", depthMaxWidth, "Max grid width for depth scans.", 1, 64).getInt();
        depthMaxHeight = c.get("depth", "maxHeight", depthMaxHeight, "Max grid height for depth scans.", 1, 64).getInt();
        depthMaxDistance = c.get("depth", "maxDistance", depthMaxDistance, "Max ray length for depth scans.", 4.0, 256.0).getDouble();
        noiseEnabled = c.get("client", "noise", noiseEnabled, "Show static/scanlines on weak signal.").getBoolean();

        maxTilt = c.get("flight", "maxTilt", maxTilt, "Maximum bank/pitch angle (degrees).", 10.0, 70.0).getDouble();
        attitudeResponse = c.get("flight", "attitudeResponse", attitudeResponse,
            "How fast the drone reaches the commanded tilt (0.05 sluggish .. 0.6 twitchy).", 0.05, 0.9).getDouble();
        accelScale = c.get("flight", "accelScale", accelScale,
            "Horizontal acceleration (blocks/tick^2) at 90 degrees of tilt; scales every other angle by sin(tilt).", 0.02, 0.3).getDouble();
        throttleAccel = c.get("flight", "throttleAccel", throttleAccel, "Climb/descend acceleration (blocks/tick^2).", 0.01, 0.09).getDouble();
        yawRate = c.get("flight", "yawRate", yawRate, "Turn rate (degrees/tick) at full yaw input.", 1.0, 12.0).getDouble();
        fpvDrag = c.get("flight", "drag", fpvDrag, "Velocity kept per tick while piloting (0.8 = OC default, 0.95 = long glide).", 0.7, 0.97).getDouble();
        speedMultiplier = c.get("flight", "speedMultiplier", speedMultiplier,
            "Multiplier on OC's 8 m/s drone speed cap while piloting.", 1.0, 6.0).getDouble();
        cameraUptilt = c.get("flight", "cameraUptilt", cameraUptilt, "Camera mounting angle (degrees up).", 0.0, 45.0).getDouble();

        acroRate = c.get("flight", "acroRate", acroRate, "Rotation speed (degrees/tick) in acro mode.", 2.0, 30.0).getDouble();
        requireGoggles = c.get("flight", "requireGoggles", requireGoggles, "Pilots must wear FPV Goggles to fly.").getBoolean();
        controlLag = c.get("link", "controlLag", controlLag, "Delay and drop control inputs as the signal weakens.").getBoolean();
        maxLagTicks = c.get("link", "maxLagTicks", maxLagTicks, "Control delay (ticks) at the edge of the range.", 0, 40).getInt();
        maxPacketLoss = c.get("link", "maxPacketLoss", maxPacketLoss, "Fraction of control packets lost at the edge of the range.", 0.0, 0.9).getDouble();

        returnOnSignalLoss = c.get("failsafe", "returnOnSignalLoss", returnOnSignalLoss,
            "Fly back to the home point when the signal is lost.").getBoolean();
        lowBatteryReturn = c.get("failsafe", "lowBatteryReturn", lowBatteryReturn,
            "Battery fraction that triggers an automatic return home (0 disables).", 0.0, 0.9).getDouble();
        streamRadius = c.get("longrange", "streamRadius", streamRadius,
            "Chunk radius streamed around the drone to the pilot (also capped by the server view distance).", 2, 12).getInt();
        boostEntities = c.get("longrange", "boostEntities", boostEntities,
            "Sync entities near the drone to the pilot even when they are beyond normal tracking distance.").getBoolean();
        boostRadius = c.get("longrange", "boostRadius", boostRadius, "Radius (blocks) around the drone for boosted entities.", 16.0, 192.0).getDouble();
        String[][] meta = {
            {"range", "Video range per antenna tier."}, {"camera", "Camera power use."},
            {"depth", "Depth sensor limits and cost."}, {"client", "Client-side visuals."},
            {"flight", "Quadcopter handling: tilt, response, drag, speed, acro, goggles."},
            {"link", "Control delay and packet loss as the signal weakens."},
            {"failsafe", "Return-to-home triggers."}, {"longrange", "Chunk and mob streaming around a far drone."}};
        for (String[] m : meta) {
            c.setCategoryComment(m[0], m[1]);
            c.setCategoryLanguageKey(m[0], "openfpv.configgui.ctgy." + m[0]);
        }
        if (c.hasChanged()) c.save();
    }
}
