package com.openfpv.oc;

import com.openfpv.Config;
import com.openfpv.bay.ModuleBay;
import com.openfpv.item.ModuleType;
import com.openfpv.net.Net;
import com.openfpv.util.CameraMath;
import java.util.HashMap;
import java.util.Map;
import li.cil.oc.api.Network;
import li.cil.oc.api.internal.Drone;
import li.cil.oc.api.machine.Arguments;
import li.cil.oc.api.machine.Callback;
import li.cil.oc.api.machine.Context;
import li.cil.oc.api.network.Connector;
import li.cil.oc.api.network.Node;
import li.cil.oc.api.network.Visibility;
import li.cil.oc.api.prefab.AbstractManagedEnvironment;
import net.minecraft.block.state.IBlockState;
import net.minecraft.entity.Entity;
import net.minecraft.item.ItemStack;
import net.minecraft.nbt.NBTTagCompound;
import net.minecraft.util.EnumFacing;
import net.minecraft.util.math.BlockPos;
import net.minecraft.util.math.RayTraceResult;
import net.minecraft.util.math.Vec3d;
import net.minecraft.world.World;

/**
 * The "camera" component of a drone. Module-gated features (night vision, zoom, gimbal, thermal,
 * depth sensing) only work when the matching module sits in the Camera Upgrade's module bay.
 */
public class CameraEnvironment extends AbstractManagedEnvironment {
    private final Drone drone;
    private final int modules;
    private final int antennaTier;

    private volatile boolean active;
    private volatile boolean night;
    private volatile boolean thermal;
    private volatile float fov = CameraMath.BASE_FOV;
    private volatile float tilt;
    private volatile float pan;

    // home point + return-to-home autopilot
    private volatile boolean hasHome;
    private double homeX, homeY, homeZ;
    private volatile boolean returning;
    private boolean lowBatteryHandled;

    private volatile boolean dirty = true;
    private int sinceSync;

    public CameraEnvironment(Drone drone, ItemStack stack) {
        this.drone = drone;
        this.modules = ModuleBay.readMask(stack);
        this.antennaTier = ModuleBay.readAntennaTier(stack);
        setNode(Network.newNode(this, Visibility.Network).withComponent("camera").withConnector().create());
    }

    // ------------------------------------------------------------------ accessors for the mod

    public int entityId() {
        return ((Entity) drone).getEntityId();
    }

    public Drone drone() {
        return drone;
    }

    public boolean isActive() {
        return active;
    }

    public boolean isReturning() {
        return returning;
    }

    public boolean hasHome() {
        return hasHome;
    }

    public double[] home() {
        return new double[]{homeX, homeY, homeZ};
    }

    public void setHomeHere() {
        Entity e = (Entity) drone;
        homeX = e.posX;
        homeY = e.posY;
        homeZ = e.posZ;
        hasHome = true;
    }

    /** Starts the autopilot back to the home point. Returns false if there is no home or the camera is off. */
    public boolean returnHome() {
        if (!hasHome) return false;
        returning = true;
        dirty = true;
        return true;
    }

    public void cancelReturn() {
        if (returning) {
            returning = false;
            dirty = true;
            Entity e = (Entity) drone;
            DroneAccess.setTarget(e, e.posX, e.posY, e.posZ);
        }
    }

    public boolean isNight() {
        return night;
    }

    public boolean isThermal() {
        return thermal;
    }

    public float fov() {
        return fov;
    }

    public float tilt() {
        return tilt;
    }

    public float pan() {
        return pan;
    }

    public int modules() {
        return modules;
    }

    public int antennaTier() {
        return antennaTier;
    }

    /** Maximum pilot-to-drone distance in blocks, set by the installed antenna. */
    public double range() {
        return Config.rangeForTier(antennaTier);
    }

    public boolean hasModule(ModuleType t) {
        return ModuleType.has(modules, t);
    }

    /** Fraction [0,1] of the drone's energy buffer that is left. */
    public float energyFraction() {
        Node n = node();
        if (n instanceof Connector) {
            Connector c = (Connector) n;
            return (float) CameraMath.energyFraction(c.globalBuffer(), c.globalBufferSize());
        }
        return 0f;
    }

    /** Heading of the drone body in Minecraft yaw degrees. */
    public float bodyYaw() {
        EnumFacing f = drone.facing();
        if (f == null || f.getAxis().isVertical()) return 0f;
        return f.getHorizontalAngle();
    }

    // ------------------------------------------------------------------ OC environment plumbing

    @Override
    public boolean canUpdate() {
        return true;
    }

    @Override
    public void update() {
        World world = drone.world();
        if (world == null || world.isRemote) return;
        Node n = node();
        if (active && n instanceof Connector && Config.energyPerTick > 0) {
            if (!((Connector) n).tryChangeBuffer(-Config.energyPerTick)) {
                active = false;
                dirty = true;
            }
        }
        if (active && Config.lowBatteryReturn > 0 && !lowBatteryHandled && hasHome
            && energyFraction() < Config.lowBatteryReturn) {
            lowBatteryHandled = true;
            returnHome();
        }
        if (energyFraction() > Config.lowBatteryReturn + 0.1) lowBatteryHandled = false;
        if (returning) autopilot();
        if (dirty || (active && ++sinceSync >= 20)) {
            sinceSync = 0;
            dirty = false;
            Net.sendState(this, world);
        }
    }

    /** Straight-line flight home, slowing down on approach; stops and hovers within 1.5 blocks. */
    private void autopilot() {
        Entity e = (Entity) drone;
        double dx = homeX - e.posX, dy = homeY - e.posY, dz = homeZ - e.posZ;
        double dist = Math.sqrt(dx * dx + dy * dy + dz * dz);
        if (dist < 1.5) {
            returning = false;
            dirty = true;
            DroneAccess.setTarget(e, e.posX, e.posY, e.posZ);
            return;
        }
        double step = CameraMath.clamp(dist * 0.02, 0.03, 0.09);
        DroneAccess.setTarget(e, e.posX + dx / dist * step, e.posY + dy / dist * step, e.posZ + dz / dist * step);
    }

    @Override
    public void onConnect(Node node) {
        super.onConnect(node);
        if (node == node()) {
            CameraRegistry.register(entityId(), this);
        }
    }

    @Override
    public void onDisconnect(Node node) {
        super.onDisconnect(node);
        if (node == node()) {
            CameraRegistry.unregister(entityId(), this);
            if (active) {
                active = false;
                World world = drone.world();
                if (world != null && !world.isRemote) Net.sendState(this, world);
            }
        }
    }

    @Override
    public void load(NBTTagCompound nbt) {
        super.load(nbt);
        night = nbt.getBoolean("night");
        thermal = nbt.getBoolean("thermal");
        if (nbt.hasKey("fov")) fov = CameraMath.clampFov(nbt.getFloat("fov"));
        tilt = CameraMath.clampTilt(nbt.getFloat("tilt"));
        pan = CameraMath.wrapPan(nbt.getFloat("pan"));
        dirty = true;
    }

    @Override
    public void save(NBTTagCompound nbt) {
        super.save(nbt);
        nbt.setBoolean("night", night);
        nbt.setBoolean("thermal", thermal);
        nbt.setFloat("fov", fov);
        nbt.setFloat("tilt", tilt);
        nbt.setFloat("pan", pan);
    }

    // ------------------------------------------------------------------ helpers

    private static Object[] fail(String message) {
        return new Object[]{null, message};
    }

    private Object[] missing(ModuleType type) {
        return hasModule(type) ? null : fail("module not installed: " + type.luaName);
    }

    private static Object[] ok(Object value) {
        return new Object[]{value};
    }

    // ------------------------------------------------------------------ Lua: base camera

    @Callback(direct = true, doc = "function():boolean -- Switch the camera on. Pilots can then start the FPV view with the V key. Costs a little energy every tick. Returns false and a reason if the drone has no energy.")
    public Object[] start(Context context, Arguments args) {
        Node n = node();
        if (n instanceof Connector && ((Connector) n).globalBuffer() <= 0) return fail("no energy");
        if (!active) {
            active = true;
            dirty = true;
        }
        if (!hasHome) setHomeHere();   // the first start() marks the home point
        return ok(true);
    }

    @Callback(direct = true, doc = "function():boolean -- Switch the camera off. Anybody flying the drone is returned to their body.")
    public Object[] stop(Context context, Arguments args) {
        if (active) {
            active = false;
            dirty = true;
        }
        return ok(true);
    }

    @Callback(direct = true, doc = "function():boolean -- Remember the drone's current position as the home point (set automatically by the first start()).")
    public Object[] setHome(Context context, Arguments args) {
        setHomeHere();
        return ok(true);
    }

    @Callback(direct = true, doc = "function():number, number, number -- The home point (x, y, z), or nil if none is set.")
    public Object[] getHome(Context context, Arguments args) {
        return hasHome ? new Object[]{homeX, homeY, homeZ} : new Object[]{null};
    }

    @Callback(direct = true, doc = "function():boolean -- Fly back to the home point on autopilot. Piloting takes over again. Also happens automatically when the signal is lost or the battery is low.")
    public Object[] returnHome(Context context, Arguments args) {
        return returnHome() ? ok(true) : fail("no home point set");
    }

    @Callback(direct = true, doc = "function():boolean -- Whether the return-home autopilot is flying.")
    public Object[] isReturning(Context context, Arguments args) {
        return ok(returning);
    }

    @Callback(direct = true, doc = "function():boolean -- Stop the return-home autopilot and hover.")
    public Object[] cancelReturn(Context context, Arguments args) {
        cancelReturn();
        return ok(true);
    }

    @Callback(direct = true, doc = "function():boolean -- Whether the camera is currently on.")
    public Object[] isActive(Context context, Arguments args) {
        return ok(active);
    }

    @Callback(direct = true, doc = "function():number -- Maximum distance in blocks between pilot and drone before the video signal is lost. Set by the antenna in the camera's bay.")
    public Object[] getRange(Context context, Arguments args) {
        return ok(range());
    }

    @Callback(direct = true, doc = "function():number -- Tier of the installed antenna: 0 = none, 1..4 = Antenna Mk1..Mk4.")
    public Object[] getAntennaTier(Context context, Arguments args) {
        return ok(antennaTier);
    }

    @Callback(direct = true, doc = "function():number -- Energy the active camera consumes per second.")
    public Object[] getEnergyUsage(Context context, Arguments args) {
        return ok(Config.energyPerTick * 20.0);
    }

    @Callback(direct = true, doc = "function():table -- List the names of the installed modules (nightvision, zoom, gimbal, thermal, depth).")
    public Object[] getModules(Context context, Arguments args) {
        Map<Integer, Object> list = new HashMap<>();
        int i = 1;
        for (ModuleType t : ModuleType.VALUES) {
            if (hasModule(t)) list.put(i++, t.luaName);
        }
        return ok(list);
    }

    @Callback(direct = true, doc = "function():number -- The heading the camera is looking along, in degrees (0 = north, 90 = east), including the gimbal pan.")
    public Object[] getHeading(Context context, Arguments args) {
        return ok((double) CameraMath.compassHeading(bodyYaw() + pan));
    }

    // ------------------------------------------------------------------ Lua: night vision module

    @Callback(direct = true, doc = "function(enabled:boolean):boolean -- [Night Vision Module] Turn night vision on or off.")
    public Object[] setNightVision(Context context, Arguments args) {
        Object[] err = missing(ModuleType.NIGHT_VISION);
        if (err != null) return err;
        boolean v = args.checkBoolean(0);
        if (v != night) {
            night = v;
            dirty = true;
        }
        return ok(night);
    }

    @Callback(direct = true, doc = "function():boolean -- [Night Vision Module] Whether night vision is on.")
    public Object[] isNightVision(Context context, Arguments args) {
        Object[] err = missing(ModuleType.NIGHT_VISION);
        return err != null ? err : ok(night);
    }

    // ------------------------------------------------------------------ Lua: thermal module

    @Callback(direct = true, doc = "function(enabled:boolean):boolean -- [Thermal Module] Turn thermal vision on or off.")
    public Object[] setThermal(Context context, Arguments args) {
        Object[] err = missing(ModuleType.THERMAL);
        if (err != null) return err;
        boolean v = args.checkBoolean(0);
        if (v != thermal) {
            thermal = v;
            dirty = true;
        }
        return ok(thermal);
    }

    @Callback(direct = true, doc = "function():boolean -- [Thermal Module] Whether thermal vision is on.")
    public Object[] isThermal(Context context, Arguments args) {
        Object[] err = missing(ModuleType.THERMAL);
        return err != null ? err : ok(thermal);
    }

    // ------------------------------------------------------------------ Lua: zoom module

    @Callback(direct = true, doc = "function(degrees:number):number -- [Zoom Module] Set the field of view (10-110 degrees, 70 is normal). Returns the value that was applied.")
    public Object[] setFov(Context context, Arguments args) {
        Object[] err = missing(ModuleType.ZOOM);
        if (err != null) return err;
        fov = CameraMath.clampFov((float) args.checkDouble(0));
        dirty = true;
        return ok((double) fov);
    }

    @Callback(direct = true, doc = "function():number -- [Zoom Module] Current field of view in degrees.")
    public Object[] getFov(Context context, Arguments args) {
        Object[] err = missing(ModuleType.ZOOM);
        return err != null ? err : ok((double) fov);
    }

    @Callback(direct = true, doc = "function(factor:number):number -- [Zoom Module] Set the zoom factor (1 = normal, up to 8). Returns the factor that was applied.")
    public Object[] setZoom(Context context, Arguments args) {
        Object[] err = missing(ModuleType.ZOOM);
        if (err != null) return err;
        fov = CameraMath.zoomToFov((float) args.checkDouble(0));
        dirty = true;
        return ok((double) CameraMath.fovToZoom(fov));
    }

    @Callback(direct = true, doc = "function():number -- [Zoom Module] Current zoom factor.")
    public Object[] getZoom(Context context, Arguments args) {
        Object[] err = missing(ModuleType.ZOOM);
        return err != null ? err : ok((double) CameraMath.fovToZoom(fov));
    }

    // ------------------------------------------------------------------ Lua: gimbal module

    @Callback(direct = true, doc = "function(degrees:number):number -- [Gimbal Module] Tilt the camera relative to the view direction. Positive looks up, range -90 to 90. Returns the applied value.")
    public Object[] setTilt(Context context, Arguments args) {
        Object[] err = missing(ModuleType.GIMBAL);
        if (err != null) return err;
        tilt = CameraMath.clampTilt((float) args.checkDouble(0));
        dirty = true;
        return ok((double) tilt);
    }

    @Callback(direct = true, doc = "function():number -- [Gimbal Module] Current tilt in degrees (positive = up).")
    public Object[] getTilt(Context context, Arguments args) {
        Object[] err = missing(ModuleType.GIMBAL);
        return err != null ? err : ok((double) tilt);
    }

    @Callback(direct = true, doc = "function(degrees:number):number -- [Gimbal Module] Pan the camera sideways relative to the view direction. Positive looks right, wraps at +-180. Returns the applied value.")
    public Object[] setPan(Context context, Arguments args) {
        Object[] err = missing(ModuleType.GIMBAL);
        if (err != null) return err;
        pan = CameraMath.wrapPan((float) args.checkDouble(0));
        dirty = true;
        return ok((double) pan);
    }

    @Callback(direct = true, doc = "function():number -- [Gimbal Module] Current pan in degrees (positive = right).")
    public Object[] getPan(Context context, Arguments args) {
        Object[] err = missing(ModuleType.GIMBAL);
        return err != null ? err : ok((double) pan);
    }

    @Callback(direct = true, doc = "function():boolean -- [Gimbal Module] Point the camera straight ahead again (pan 0, tilt 0).")
    public Object[] centerGimbal(Context context, Arguments args) {
        Object[] err = missing(ModuleType.GIMBAL);
        if (err != null) return err;
        pan = 0f;
        tilt = 0f;
        dirty = true;
        return ok(true);
    }

    // ------------------------------------------------------------------ Lua: depth module

    @Callback(doc = "function(width:number, height:number[, maxDistance:number]):table -- [Depth Sensor Module] Scan a width*height grid in front of the camera. Returns rows (top to bottom) of distances in blocks, -1 where nothing was hit. The grid follows the camera's field of view, pan and tilt.")
    public Object[] depth(Context context, Arguments args) {
        return scan(args, false);
    }

    @Callback(doc = "function(width:number, height:number[, maxDistance:number]):table -- [Depth Sensor Module] Like depth(), but returns the map colour (0xRRGGBB) of the block hit by each ray, -1 where nothing was hit.")
    public Object[] color(Context context, Arguments args) {
        return scan(args, true);
    }

    private Object[] scan(Arguments args, boolean colors) {
        Object[] err = missing(ModuleType.DEPTH);
        if (err != null) return err;
        int w = args.checkInteger(0);
        int h = args.checkInteger(1);
        if (w < 1 || h < 1 || w > Config.depthMaxWidth || h > Config.depthMaxHeight) {
            return fail("grid must be 1.." + Config.depthMaxWidth + " wide and 1.." + Config.depthMaxHeight + " high");
        }
        double maxDist = CameraMath.clamp(args.optDouble(2, Config.depthMaxDistance), 1.0, Config.depthMaxDistance);
        Node n = node();
        double cost = (double) w * h * Config.depthEnergyPerRay;
        if (cost > 0 && n instanceof Connector && !((Connector) n).tryChangeBuffer(-cost)) {
            return fail("not enough energy");
        }
        World world = drone.world();
        Entity e = (Entity) drone;
        Vec3d origin = new Vec3d(e.posX, e.posY + e.height * 0.5, e.posZ);
        double yaw = bodyYaw() + pan;
        double pitch = -tilt;
        Map<Integer, Object> rows = new HashMap<>();
        for (int row = 0; row < h; row++) {
            Map<Integer, Object> line = new HashMap<>();
            for (int col = 0; col < w; col++) {
                double[] d = CameraMath.gridRay(yaw, pitch, fov, w, h, col, row);
                Vec3d end = origin.addVector(d[0] * maxDist, d[1] * maxDist, d[2] * maxDist);
                RayTraceResult hit = world.rayTraceBlocks(origin, end, false, true, false);
                Object value;
                if (hit == null || hit.typeOfHit != RayTraceResult.Type.BLOCK) {
                    value = -1;
                } else if (colors) {
                    BlockPos p = hit.getBlockPos();
                    IBlockState s = world.getBlockState(p);
                    value = s.getMapColor(world, p).colorValue;
                } else {
                    value = Math.round(hit.hitVec.distanceTo(origin) * 100.0) / 100.0;
                }
                line.put(col + 1, value);
            }
            rows.put(row + 1, line);
        }
        return ok(rows);
    }
}
