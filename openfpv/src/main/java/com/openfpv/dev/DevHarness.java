package com.openfpv.dev;

import com.openfpv.Config;
import com.openfpv.ModItems;
import com.openfpv.OpenFPV;
import com.openfpv.bay.ContainerModuleBay;
import com.openfpv.bay.ModuleBay;
import com.openfpv.client.FpvClient;
import com.openfpv.item.ModuleType;
import com.openfpv.net.PacketViewEnd;
import com.openfpv.oc.CameraEnvironment;
import com.openfpv.oc.CameraRegistry;
import com.openfpv.server.ViewerManager;
import com.openfpv.util.CameraMath;
import java.io.File;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.ArrayList;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Map;
import java.util.concurrent.Callable;
import java.util.function.BooleanSupplier;
import li.cil.oc.api.Items;
import li.cil.oc.api.machine.Machine;
import li.cil.oc.api.network.Component;
import li.cil.oc.api.internal.Drone;
import net.minecraft.block.Block;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiIngameMenu;
import net.minecraft.client.gui.GuiMainMenu;
import net.minecraft.client.settings.KeyBinding;
import net.minecraft.entity.Entity;
import net.minecraft.entity.EntityLiving;
import net.minecraft.entity.monster.EntityZombie;
import net.minecraft.entity.passive.EntityPig;
import net.minecraft.entity.passive.EntitySheep;
import net.minecraft.entity.player.EntityPlayerMP;
import net.minecraft.init.Blocks;
import net.minecraft.item.EnumDyeColor;
import net.minecraft.item.ItemStack;
import net.minecraft.nbt.NBTTagCompound;
import net.minecraft.nbt.NBTTagList;
import net.minecraft.server.MinecraftServer;
import net.minecraft.util.EnumFacing;
import net.minecraft.util.EnumHand;
import net.minecraft.util.ScreenShotHelper;
import net.minecraft.util.math.BlockPos;
import net.minecraft.world.GameType;
import net.minecraft.world.World;
import net.minecraft.world.WorldServer;
import net.minecraft.world.WorldSettings;
import net.minecraft.world.WorldType;
import net.minecraftforge.common.MinecraftForge;
import net.minecraftforge.fml.common.eventhandler.SubscribeEvent;
import net.minecraftforge.fml.common.gameevent.TickEvent;
import net.minecraftforge.fml.relauncher.Side;
import net.minecraftforge.fml.relauncher.SideOnly;

/**
 * End-to-end self test, active only when started with -Popenfpv.autotest (see build.gradle).
 * Builds a flat world with real OpenComputers drones running real Lua, then drives the real
 * client code path: module bay GUI, FPV entry, keyboard flight, vision modes, signal loss, Esc.
 * Results go to run/autotest/summary.txt and screenshots to run/autotest/screenshots/.
 * This class is excluded from the release jar.
 */
@SideOnly(Side.CLIENT)
public final class DevHarness {
    private static final File OUT = new File("autotest");
    private static final String TAG = "AUTOTEST";

    private final Minecraft mc = Minecraft.getMinecraft();
    private final List<BooleanSupplier> steps = new ArrayList<>();
    private final List<String> results = new ArrayList<>();
    private int index;
    private boolean launched;
    private int fails;
    private int tickInStep;

    // scene
    private int droneA = -1, droneB = -1, droneC = -1, droneE = -1;
    private double dirX, dirZ;     // unit axis vector B faces
    private double wallDistance;
    private double expectedA;
    private double maxSpeed;

    public static void install() {
        MinecraftForge.EVENT_BUS.register(new DevHarness());
    }

    private DevHarness() {
        OUT.mkdirs();
        deleteWorld();
        buildSteps();
        OpenFPV.log.info("{} harness installed", TAG);
    }

    // ================================================================== plumbing

    @SubscribeEvent
    public void onTick(TickEvent.ClientTickEvent e) {
        if (e.phase != TickEvent.Phase.END) return;
        if (!launched) {
            if (mc.currentScreen instanceof GuiMainMenu) {
                launched = true;
                WorldSettings ws = new WorldSettings(4242L, GameType.CREATIVE, false, false, WorldType.FLAT);
                ws.setGeneratorOptions("3;minecraft:bedrock,2*minecraft:dirt,minecraft:grass;1;");
                ws.enableCommands();
                mc.launchIntegratedServer("openfpv_autotest", "openfpv_autotest", ws);
                log("launching integrated server");
            }
            return;
        }
        if (index >= steps.size()) return;
        tickInStep++;
        try {
            if (steps.get(index).getAsBoolean()) {
                index++;
                tickInStep = 0;
            }
        } catch (Throwable t) {
            check("step " + index + " threw", false, t.toString());
            OpenFPV.log.error("harness step failed", t);
            index = steps.size() - 1; // jump to the final "finish" step
            tickInStep = 0;
        }
    }

    private void log(String s) {
        OpenFPV.log.info("{} {}", TAG, s);
    }

    private void check(String name, boolean ok, String detail) {
        String line = (ok ? "PASS " : "FAIL ") + name + (detail == null || detail.isEmpty() ? "" : "  -- " + detail);
        if (!ok) fails++;
        results.add(line);
        log(line);
    }

    private MinecraftServer server() {
        return mc.getIntegratedServer();
    }

    private <T> T onServer(Callable<T> c) throws Exception {
        return server().callFromMainThread(c).get();
    }

    private void shot(String name) {
        ScreenShotHelper.saveScreenshot(OUT, name + ".png", mc.displayWidth, mc.displayHeight, mc.getFramebuffer());
        log("screenshot " + name);
    }

    /** True once cond holds; records a failure and moves on after timeout ticks. */
    private boolean until(BooleanSupplier cond, int timeout, String label) {
        if (cond.getAsBoolean()) return true;
        if (tickInStep > timeout) {
            check("timeout waiting for " + label, false, timeout + " ticks");
            return true;
        }
        return false;
    }

    private BooleanSupplier once(Runnable r) {
        boolean[] done = {false};
        return () -> {
            if (!done[0]) {
                done[0] = true;
                r.run();
            }
            return true;
        };
    }

    private BooleanSupplier waitTicks(int n) {
        return () -> tickInStep >= n;
    }

    private void add(BooleanSupplier s) {
        steps.add(s);
    }

    private void deleteWorld() {
        File w = new File("saves/openfpv_autotest");
        if (w.exists()) {
            try {
                org.apache.commons.io.FileUtils.deleteDirectory(w);
            } catch (IOException ex) {
                OpenFPV.log.warn("could not delete old test world", ex);
            }
        }
    }

    // ================================================================== world helpers (server thread)

    private Entity entity(int id) {
        WorldServer w = server().getWorld(0);
        return id < 0 ? null : w.getEntityByID(id);
    }

    private CameraEnvironment cam(int id) {
        return CameraRegistry.get(id);
    }

    private Object[] invoke(int droneId, String method, Object... args) throws Exception {
        CameraEnvironment env = cam(droneId);
        if (env == null) throw new IllegalStateException("no camera for drone " + droneId);
        Component c = (Component) env.node();
        return onServer(() -> c.invoke(method, null, args));
    }

    private ItemStack cameraStack(boolean withModules) {
        return cameraStack(withModules, 0);
    }

    private ItemStack cameraStack(boolean withModules, int antennaTier) {
        ItemStack s = new ItemStack(ModItems.CAMERA);
        if (withModules || antennaTier > 0) {
            ItemStack[] bay = new ItemStack[ModuleBay.SIZE];
            for (int i = 0; i < bay.length; i++) bay[i] = ItemStack.EMPTY;
            if (withModules) {
                for (int i = 0; i < ModuleBay.MODULE_SLOTS; i++) bay[i] = new ItemStack(ModItems.MODULES[i]);
            }
            if (antennaTier > 0) bay[ModuleBay.ANTENNA_SLOT] = new ItemStack(ModItems.ANTENNAS[antennaTier - 1]);
            ModuleBay.write(s, bay);
        }
        return s;
    }

    private ItemStack makeDroneItem(String lua, boolean modules, int antenna, boolean chunkloader) {
        ItemStack drone = Items.get("drone").createItemStack(1);
        NBTTagCompound tag = new NBTTagCompound();
        tag.setByte("oc:tier", (byte) 3);
        tag.setInteger("oc:storedEnergy", 50000);
        NBTTagList comps = new NBTTagList();
        ItemStack eeprom = Items.registerEEPROM("openfpv-test", lua.getBytes(StandardCharsets.UTF_8), new byte[0], false);
        List<ItemStack> parts = new ArrayList<>(java.util.Arrays.asList(
            Items.get("cpu3").createItemStack(1), Items.get("ram6").createItemStack(1),
            Items.get("ram6").createItemStack(1), eeprom, cameraStack(modules, antenna)));
        // A drone that is far from every player only keeps running with a chunk loader upgrade (as in normal OC play).
        if (chunkloader) parts.add(Items.get("chunkloaderupgrade").createItemStack(1));
        for (ItemStack s : parts) comps.appendTag(s.writeToNBT(new NBTTagCompound()));
        tag.setTag("oc:components", comps);
        drone.setTagCompound(tag);
        return drone;
    }

    /** Places a drone just above the block at (x, 3, z) using OC's own item-use path. */
    private int spawnDrone(String lua, boolean modules, int antenna, boolean chunkloader, int x, int z) throws Exception {
        return onServer(() -> {
            WorldServer w = server().getWorld(0);
            EntityPlayerMP p = server().getPlayerList().getPlayers().get(0);
            ItemStack item = makeDroneItem(lua, modules, antenna, chunkloader);
            // OC's item delegator reads the player's held stack, so the drone must be in hand.
            ItemStack before = p.getHeldItem(EnumHand.MAIN_HAND);
            p.setHeldItem(EnumHand.MAIN_HAND, item);
            net.minecraft.util.EnumActionResult res =
                item.getItem().onItemUse(p, w, new BlockPos(x, 3, z), EnumHand.MAIN_HAND, EnumFacing.UP, 0.5f, 1.0f, 0.5f);
            p.setHeldItem(EnumHand.MAIN_HAND, before);
            log("onItemUse -> " + res);
            Entity found = null;
            for (Entity e : w.loadedEntityList) {
                String cn = e.getClass().getName();
                if (cn.contains("Drone")) {
                    log("entity " + cn + " id=" + e.getEntityId() + " at " + e.posX + "," + e.posY + "," + e.posZ
                        + " isDrone=" + (e instanceof Drone) + " dead=" + e.isDead);
                }
                if (e instanceof Drone && Math.abs(e.posX - x) < 3 && Math.abs(e.posZ - z) < 3) found = e;
            }
            if (found == null) log("no drone near " + x + "," + z + "; loaded entities=" + w.loadedEntityList.size()
                + " chunkLoaded=" + w.isChunkGeneratedAt(x >> 4, z >> 4));
            return found == null ? -1 : found.getEntityId();
        });
    }

    /** Unit axis vector the drone's body faces (defaults to south for vertical facings). */
    private double[] heading(Entity e) {
        EnumFacing f = ((Drone) e).facing();
        if (f == null || f.getAxis().isVertical()) return new double[]{0, 1};
        return new double[]{f.getFrontOffsetX(), f.getFrontOffsetZ()};
    }

    /** Builds a stone wall (with two wool columns) {@code distance} blocks in front of the drone. */
    private void buildWall(WorldServer w, Entity drone, double distance) {
        double[] d = heading(drone);
        int bx = (int) Math.floor(drone.posX), bz = (int) Math.floor(drone.posZ);
        double rx = -d[1], rz = d[0];
        for (int s = -6; s <= 6; s++) {
            for (int h = 0; h < 8; h++) {
                int x = (int) (bx + d[0] * distance + rx * s);
                int z = (int) (bz + d[1] * distance + rz * s);
                put(w, x, 4 + h, z, (s == -3 || s == 3)
                    ? Blocks.WOOL.getStateFromMeta((s < 0 ? EnumDyeColor.RED : EnumDyeColor.BLUE).getMetadata())
                    : Blocks.STONE.getDefaultState());
            }
        }
    }

    /** Distance from the drone's centre to the front face of the wall built by buildWall. */
    private double wallFaceDistance(Entity drone, double distance) {
        double[] d = heading(drone);
        int bx = (int) Math.floor(drone.posX), bz = (int) Math.floor(drone.posZ);
        if (d[1] > 0) return (bz + distance) - drone.posZ;
        if (d[1] < 0) return drone.posZ - (bz - distance + 1);
        if (d[0] > 0) return (bx + distance) - drone.posX;
        return drone.posX - (bx - distance + 1);
    }

    private void startMachines(int... ids) throws Exception {
        onServer(() -> {
            for (int id : ids) {
                Entity e = entity(id);
                // Same path as a player sneak-right-clicking the drone: Drone.start() runs preparePowerUp()
                // (which gives the drone its acceleration) and then starts the machine.
                if (e instanceof Drone) e.getClass().getMethod("start").invoke(e);
            }
            return null;
        });
    }

    private void put(World w, int x, int y, int z, net.minecraft.block.state.IBlockState s) {
        w.setBlockState(new BlockPos(x, y, z), s, 2);
    }

    // ================================================================== the scenario

    private static final String LUA_API =
        "local cam = component.proxy(component.list('camera')())\n"
        + "local out = {}\n"
        + "local function put(k, v) out[#out+1] = k .. '=' .. tostring(v) end\n"
        + "local mods = cam.getModules(); local names = {}\n"
        + "for i = 1, #mods do names[#names+1] = mods[i] end\n"
        + "put('modules', table.concat(names, ','))\n"
        + "put('start', cam.start()); put('active', cam.isActive()); put('range', cam.getRange()); put('antenna', cam.getAntennaTier())\n"
        + "put('setFov', cam.setFov(40)); put('getFov', cam.getFov()); put('zoom', string.format('%.2f', cam.getZoom()))\n"
        + "put('setZoom', cam.setZoom(2)); put('fovAfterZoom', string.format('%.1f', cam.getFov()))\n"
        + "put('setTilt', cam.setTilt(15)); put('tiltClamp', cam.setTilt(200))\n"
        + "put('setPan', cam.setPan(200)); put('getPan', string.format('%.1f', cam.getPan()))\n"
        + "put('center', cam.centerGimbal()); put('pan0', cam.getPan()); put('tilt0', cam.getTilt())\n"
        + "put('nv', cam.setNightVision(true)); put('isNv', cam.isNightVision())\n"
        + "put('th', cam.setThermal(true)); put('isTh', cam.isThermal())\n"
        + "local d = cam.depth(4, 3); put('rows', #d); put('cols', #d[1])\n"
        + "local cells = {}; for r = 1, #d do for c = 1, #d[r] do cells[#cells+1] = tostring(d[r][c]) end end\n"
        + "put('depthCells', #cells)\n"
        + "local col = cam.color(2, 2); put('colorRows', #col)\n"
        + "put('tooBig', select(2, cam.depth(100, 100)))\n"
        + "put('usage', cam.getEnergyUsage())\n"
        + "cam.setFov(70)\n"
        + "put('wallDepth', string.format('%.3f', cam.depth(1, 1)[1][1]))\n"
        + "put('wallColor', cam.color(1, 1)[1][1])\n"
        + "cam.setTilt(80); put('skyDepth', cam.depth(1, 1)[1][1])\n"
        + "cam.setTilt(-80); put('groundDepth', string.format('%.2f', cam.depth(1, 1)[1][1]))\n"
        + "cam.centerGimbal()\n"
        + "cam.stop(); put('stopped', cam.isActive())\n"
        + "error('RESULT|' .. table.concat(out, ';'), 0)\n";

    private static final String LUA_IDLE =
        "local cam = component.proxy(component.list('camera')())\n"
        + "cam.start()\n"
        + "while true do computer.pullSignal(1) end\n";

    private static final String LUA_FAR =
        "local cl = component.list('chunkloader')()\n"
        + "if cl then component.invoke(cl, 'setActive', true) end\n"
        + "local cam = component.proxy(component.list('camera')())\n"
        + "cam.start()\n"
        + "while true do computer.pullSignal(1) end\n";

    private void buildSteps() {
        // ---- 0. wait for the world
        add(() -> until(() -> mc.world != null && mc.player != null && mc.getIntegratedServer() != null
            && mc.player.ticksExisted > 60, 3000, "world load"));

        // ---- 1. scenery + drones: build the world first, then switch the machines on
        add(once(() -> {
            try {
                onServer(() -> {
                    WorldServer w = server().getWorld(0);
                    w.getGameRules().setOrCreateGameRule("doDaylightCycle", "false");
                    w.getGameRules().setOrCreateGameRule("doMobSpawning", "false");
                    w.setWorldTime(6000);
                    w.getWorldInfo().setRaining(false);
                    EntityPlayerMP p = server().getPlayerList().getPlayers().get(0);
                    p.setPositionAndUpdate(0.5, 4, -6.5);
                    // entities can only be spawned in loaded chunks, so load the whole test area up front
                    for (int cx = -5; cx <= 5; cx++) {
                        for (int cz = -5; cz <= 5; cz++) w.getChunkFromChunkCoords(cx, cz);
                    }
                    return null;
                });
                onServer(() -> {   // the far drone's area must be loaded before anything can spawn there
                    WorldServer w = server().getWorld(0);
                    for (int cx = 15; cx <= 22; cx++) {
                        for (int cz = -3; cz <= 3; cz++) w.getChunkFromChunkCoords(cx, cz);
                    }
                    return null;
                });
                droneA = spawnDrone(LUA_API, true, 2, false, 20, 0);
                droneB = spawnDrone(LUA_FAR, true, 0, true, 0, 0);
                droneC = spawnDrone(LUA_FAR, false, 0, true, -20, 0);
                droneE = spawnDrone(LUA_FAR, true, 3, true, 300, 0);
                log("drones A=" + droneA + " B=" + droneB + " C=" + droneC + " E=" + droneE);
                check("four drones spawned", droneA >= 0 && droneB >= 0 && droneC >= 0 && droneE >= 0,
                    droneA + "," + droneB + "," + droneC + "," + droneE);
                onServer(() -> {   // something alive next to the far drone, to prove entities get streamed too
                    WorldServer w = server().getWorld(0);
                    Entity far = entity(droneE);
                    spawnMob(w, new EntityPig(w), far.posX + 6.5, 4, far.posZ + 3.5);
                    return null;
                });

                wallDistance = 14;
                Entity a = entity(droneA), b = entity(droneB);
                expectedA = wallFaceDistance(a, wallDistance);
                double[] d = heading(b);
                dirX = d[0];
                dirZ = d[1];
                log("A faces " + java.util.Arrays.toString(heading(a)) + ", B faces " + java.util.Arrays.toString(d)
                    + ", expected wall distance for A = " + expectedA);
                onServer(() -> {
                    WorldServer w = server().getWorld(0);
                    buildWall(w, a, wallDistance);
                    buildWall(w, b, wallDistance);
                    int bx = (int) Math.floor(b.posX), bz = (int) Math.floor(b.posZ);
                    double rx = -dirZ, rz = dirX;
                    // lava + glowstone for thermal contrast
                    for (int s = 1; s <= 3; s++) {
                        put(w, (int) (bx + dirX * 7 + rx * s), 3, (int) (bz + dirZ * 7 + rz * s), Blocks.LAVA.getDefaultState());
                    }
                    put(w, (int) (bx + dirX * 9 - rx * 4), 4, (int) (bz + dirZ * 9 - rz * 4), Blocks.GLOWSTONE.getDefaultState());
                    // living things in view of drone B
                    spawnMob(w, new EntityPig(w), bx + dirX * 6 + 0.5 - rx * 1, 4, bz + dirZ * 6 + 0.5 - rz * 1);
                    spawnMob(w, new EntitySheep(w), bx + dirX * 8 + 0.5 + rx * 3, 4, bz + dirZ * 8 + 0.5 + rz * 3);
                    spawnMob(w, new EntityZombie(w), bx + dirX * 5 + 0.5 + rx * 2, 4, bz + dirZ * 5 + 0.5 + rz * 2);
                    return null;
                });
                startMachines(droneA, droneB, droneC, droneE);
            } catch (Exception ex) {
                check("scene setup", false, ex.toString());
                OpenFPV.log.error("scene setup failed", ex);
            }
        }));
        add(waitTicks(20));

        // ---- 2. wait for Lua to run on all drones
        add(() -> until(() -> {
            try {
                Entity a = entity(droneA);
                String err = a instanceof Drone ? ((Drone) a).machine().lastError() : null;
                return err != null && cam(droneB) != null && cam(droneB).isActive() && cam(droneC) != null && cam(droneC).isActive()
                    && cam(droneE) != null && cam(droneE).isActive();
            } catch (Exception ex) {
                return false;
            }
        }, 1200, "drone Lua to finish/boot"));
        add(waitTicks(40));

        // ---- 3. Lua API results from drone A (real Lua, real tables)
        add(once(() -> {
            try {
                String err = onServer(() -> ((Drone) entity(droneA)).machine().lastError());
                log("drone A result: " + err);
                boolean has = err != null && err.contains("RESULT|");
                check("drone A Lua ran to completion", has, err);
                if (!has) return;
                Map<String, String> m = new LinkedHashMap<>();
                for (String kv : err.substring(err.indexOf("RESULT|") + 7).split(";")) {
                    int i = kv.indexOf('=');
                    if (i > 0) m.put(kv.substring(0, i), kv.substring(i + 1));
                }
                eq(m, "modules", "nightvision,zoom,gimbal,thermal,depth");
                eq(m, "start", "true");
                eq(m, "active", "true");
                eq(m, "range", "400.0");
                eq(m, "antenna", "2");
                eq(m, "setFov", "40.0");
                eq(m, "getFov", "40.0");
                eq(m, "zoom", "1.75");
                eq(m, "setZoom", "2.0");
                eq(m, "fovAfterZoom", "35.0");
                eq(m, "setTilt", "15.0");
                eq(m, "tiltClamp", "90.0");
                eq(m, "getPan", "-160.0");
                eq(m, "pan0", "0.0");
                eq(m, "tilt0", "0.0");
                eq(m, "nv", "true");
                eq(m, "isNv", "true");
                eq(m, "th", "true");
                eq(m, "isTh", "true");
                eq(m, "rows", "3");
                eq(m, "cols", "4");
                eq(m, "depthCells", "12");
                eq(m, "colorRows", "2");
                eq(m, "usage", "2.0");
                eq(m, "stopped", "false");
                check("Lua: oversized depth grid rejected", String.valueOf(m.get("tooBig")).startsWith("grid must be"), m.get("tooBig"));
                double wall = Double.parseDouble(m.getOrDefault("wallDepth", "-99"));
                check("depth(1,1) hits the wall at the right distance", Math.abs(wall - expectedA) < 0.2,
                    "got " + wall + " expected ~" + expectedA);
                int stone = net.minecraft.block.material.MapColor.STONE.colorValue;
                check("color(1,1) returns the wall block's map colour", String.valueOf(stone).equals(m.get("wallColor")),
                    m.get("wallColor") + " vs " + stone);
                eq(m, "skyDepth", "-1");
                double ground = Double.parseDouble(m.getOrDefault("groundDepth", "-99"));
                check("tilt 80 deg down hits the ground close by", ground > 0 && ground < 6, String.valueOf(ground));
            } catch (Exception ex) {
                check("read drone A result", false, ex.toString());
            }
        }));

        // ---- 4. module gating on the module-less camera (drone C), through OC's real invoke path
        add(once(() -> {
            try {
                Object[] r = invoke(droneC, "setNightVision", true);
                check("no-module camera refuses night vision", r != null && r.length == 2 && r[0] == null
                    && String.valueOf(r[1]).contains("module not installed"), java.util.Arrays.toString(r));
                r = invoke(droneC, "depth", 2, 2);
                check("no-module camera refuses depth", r != null && r.length == 2 && r[0] == null, java.util.Arrays.toString(r));
                r = invoke(droneC, "getModules");
                // OC hands tables back as Scala maps; an empty one prints as "Map()"
                check("no-module camera lists no modules", r != null && r.length == 1 && String.valueOf(r[0]).replace(" ", "").equals("Map()"),
                    String.valueOf(r == null ? null : r[0]));
            } catch (Exception ex) {
                check("module gating", false, ex.toString());
            }
        }));
        add(waitTicks(20));

        // ---- 5b. what OC's Assembler will accept (it asks the driver registry the same question)
        add(once(() -> {
            ItemStack camera = cameraStack(true);
            li.cil.oc.api.driver.DriverItem forDrone = li.cil.oc.api.Driver.driverFor(camera, Drone.class);
            check("OC lets the camera be installed in a drone", forDrone != null, String.valueOf(forDrone));
            check("camera goes in an upgrade slot", forDrone != null && li.cil.oc.api.driver.item.Slot.Upgrade.equals(forDrone.slot(camera)),
                forDrone == null ? "no driver" : forDrone.slot(camera));
            check("camera fits any upgrade tier (tier 0)", forDrone != null && forDrone.tier(camera) == 0, "");
            check("OC refuses the camera in a robot",
                li.cil.oc.api.Driver.driverFor(camera, li.cil.oc.api.internal.Robot.class) == null, "");
            check("OC refuses the camera in a micro-controller-less host (generic EnvironmentHost)",
                li.cil.oc.api.Driver.driverFor(camera, li.cil.oc.api.network.EnvironmentHost.class) == null, "");
            boolean modulesLoose = true;
            for (int i = 0; i < ModuleBay.MODULE_SLOTS; i++) {
                modulesLoose &= li.cil.oc.api.Driver.driverFor(new ItemStack(ModItems.MODULES[i]), Drone.class) == null;
            }
            for (int i = 0; i < ModItems.ANTENNAS.length; i++) {
                modulesLoose &= li.cil.oc.api.Driver.driverFor(new ItemStack(ModItems.ANTENNAS[i]), Drone.class) == null;
            }
            check("modules and antennas cannot be put into a drone directly (only into the camera's bay)", modulesLoose, "");
        }));

        // ---- 6. recipes
        add(once(() -> {
            String[] names = {"camera_upgrade", "night_vision_module", "zoom_module", "gimbal_module", "thermal_module", "depth_module",
                "antenna_mk1", "antenna_mk2", "antenna_mk3", "antenna_mk4", "fpv_goggles"};
            for (String n : names) {
                net.minecraft.item.crafting.IRecipe r = net.minecraft.item.crafting.CraftingManager.REGISTRY
                    .getObject(new net.minecraft.util.ResourceLocation(OpenFPV.MOD_ID, n));
                check("recipe registered: " + n, r != null && !r.getRecipeOutput().isEmpty()
                    && r.getRecipeOutput().getItem().getRegistryName().getResourcePath().equals(n),
                    r == null ? "missing" : String.valueOf(r.getIngredients()));
            }
        }));

        // ---- 7. module bay GUI
        add(once(() -> {
            try {
                onServer(() -> {
                    EntityPlayerMP p = server().getPlayerList().getPlayers().get(0);
                    p.inventory.setInventorySlotContents(0, cameraStack(false));
                    p.inventory.currentItem = 0;
                    p.openGui(OpenFPV.instance, com.openfpv.bay.GuiHandler.GUI_MODULE_BAY, p.world, 0, 0, 0);
                    return null;
                });
            } catch (Exception ex) {
                check("open bay", false, ex.toString());
            }
        }));
        add(() -> until(() -> mc.currentScreen instanceof com.openfpv.bay.GuiModuleBay, 200, "bay GUI to open"));
        add(waitTicks(10));
        add(once(() -> shot("01_bay_empty")));
        add(waitTicks(5));
        add(once(() -> {
            try {
                // put every module in its slot like a player would, then verify the camera item
                boolean ok = onServer(() -> {
                    EntityPlayerMP p = server().getPlayerList().getPlayers().get(0);
                    ContainerModuleBay c = (ContainerModuleBay) p.openContainer;
                    // wrong module in a slot must be refused
                    boolean refused = !c.getSlot(0).isItemValid(new ItemStack(ModItems.MODULES[ModuleType.ZOOM.ordinal()]));
                    boolean junk = !c.getSlot(0).isItemValid(new ItemStack(Blocks.DIRT));
                    boolean antennaOnly = !c.getSlot(ModuleBay.ANTENNA_SLOT).isItemValid(new ItemStack(ModItems.MODULES[0]))
                        && c.getSlot(ModuleBay.ANTENNA_SLOT).isItemValid(new ItemStack(ModItems.ANTENNAS[0]))
                        && !c.getSlot(0).isItemValid(new ItemStack(ModItems.ANTENNAS[0]));
                    for (int i = 0; i < ModuleBay.MODULE_SLOTS; i++) c.getSlot(i).putStack(new ItemStack(ModItems.MODULES[i]));
                    c.getSlot(ModuleBay.ANTENNA_SLOT).putStack(new ItemStack(ModItems.ANTENNAS[2]));
                    check("antenna slot takes only antennas, module slots only modules", antennaOnly, "");
                    return refused && junk;
                });
                check("bay refuses wrong-slot module and junk", ok, "");
            } catch (Exception ex) {
                check("bay fill", false, ex.toString());
            }
        }));
        add(waitTicks(10));
        add(once(() -> shot("02_bay_filled")));
        add(waitTicks(5));
        add(once(() -> {
            try {
                int[] bay = onServer(() -> {
                    EntityPlayerMP p = server().getPlayerList().getPlayers().get(0);
                    int[] r = {ModuleBay.readMask(p.inventory.getStackInSlot(0)), ModuleBay.readAntennaTier(p.inventory.getStackInSlot(0))};
                    p.closeScreen();
                    return r;
                });
                check("module bay persisted to the camera item NBT", bay[0] == 31, "mask=" + bay[0]);
                check("antenna persisted to the camera item NBT", bay[1] == 3, "tier=" + bay[1]);
            } catch (Exception ex) {
                check("bay persist", false, ex.toString());
            }
        }));
        add(() -> until(() -> mc.currentScreen == null, 100, "bay GUI to close"));

        // ---- 8. FPV: enter
        add(once(() -> {
            try {
                Entity b = entity(droneB);
                onServer(() -> {
                    EntityPlayerMP p = server().getPlayerList().getPlayers().get(0);
                    p.setPositionAndUpdate(b.posX - dirX * 3, 4, b.posZ - dirZ * 3);
                    return null;
                });
                float yaw = (float) Math.toDegrees(Math.atan2(-dirX, dirZ));
                mc.player.rotationYaw = mc.player.prevRotationYaw = yaw;
                mc.player.rotationPitch = mc.player.prevRotationPitch = 4f;
                mc.gameSettings.hideGUI = false;
            } catch (Exception ex) {
                check("position pilot", false, ex.toString());
            }
        }));
        add(waitTicks(30));
        add(once(() -> {
            check("client knows B has an active camera", FpvClient.get(droneB) != null && FpvClient.get(droneB).active, "");
            FpvClient.toggle();
        }));
        add(() -> until(FpvClient::isFlying, 100, "FPV to start"));
        add(waitTicks(40));
        add(once(() -> {
            check("FPV started on drone B", FpvClient.viewingId() == droneB, "viewing=" + FpvClient.viewingId());
            check("render view entity is the drone", mc.getRenderViewEntity() != null && mc.getRenderViewEntity().getEntityId() == droneB, "");
            boolean serverSide;
            try {
                serverSide = onServer(() -> ViewerManager.isViewing(mc.player.getUniqueID()));
            } catch (Exception ex) {
                serverSide = false;
            }
            check("server registered the FPV session", serverSide, "");
            shot("03_fpv_normal");
        }));

        // ---- 10. vision modes
        add(once(() -> modes(true, false, "05_fpv_night")));
        add(waitTicks(30));
        add(once(() -> {
            check("night vision post-effect is active", mc.entityRenderer.isShaderActive(), "");
            FpvClient.Cam c = FpvClient.current();
            check("client received night-vision state", c != null && c.night, "");
            shot("05_fpv_night");
            modes(false, true, null);
        }));
        add(waitTicks(30));
        add(once(() -> {
            check("thermal post-effect is active", mc.entityRenderer.isShaderActive(), "");
            FpvClient.Cam c = FpvClient.current();
            check("client received thermal state", c != null && c.thermal, "");
            shot("06_fpv_thermal");
            modes(false, false, null);
        }));
        add(waitTicks(30));
        add(once(() -> {
            check("post-effect cleared when modes are off", !mc.entityRenderer.isShaderActive(), "");
            try {
                invoke(droneB, "setFov", 25.0);
            } catch (Exception ex) {
                check("zoom", false, ex.toString());
            }
        }));
        add(waitTicks(25));
        add(once(() -> {
            FpvClient.Cam c = FpvClient.current();
            check("client received zoomed FOV", c != null && Math.abs(c.fov - 25f) < 0.01, c == null ? "null" : "" + c.fov);
            shot("07_fpv_zoom");
            try {
                invoke(droneB, "setFov", 70.0);
                invoke(droneB, "setPan", 35.0);
                invoke(droneB, "setTilt", 15.0);
            } catch (Exception ex) {
                check("gimbal", false, ex.toString());
            }
        }));
        add(waitTicks(25));
        add(once(() -> {
            FpvClient.Cam c = FpvClient.current();
            check("client received gimbal state", c != null && Math.abs(c.pan - 35f) < 0.01 && Math.abs(c.tilt - 15f) < 0.01, "");
            float expectedYaw = mc.player.rotationYaw + 35f;
            check("view yaw includes the gimbal pan", Math.abs(MathHelperWrap(FpvClient.viewYaw() - expectedYaw)) < 1.0,
                FpvClient.viewYaw() + " vs " + expectedYaw);
            shot("08_fpv_gimbal");
            try {
                invoke(droneB, "centerGimbal");
            } catch (Exception ex) {
                check("gimbal reset", false, ex.toString());
            }
        }));
        add(waitTicks(20));

        // ---- 9. quadcopter flight (angle mode)
        double[] start = new double[3];
        float[] peak = new float[3];       // peak pitch / roll seen on the client
        double[] glide = new double[3];
        add(once(() -> {
            Entity b = entity(droneB);
            start[0] = b.posX; start[1] = b.posY; start[2] = b.posZ;
            maxSpeed = 0;
            peak[0] = peak[1] = 0;
            KeyBinding.setKeyBindState(mc.gameSettings.keyBindForward.getKeyCode(), true);
        }));
        add(() -> { // sample the attitude and HUD speed every tick
            maxSpeed = Math.max(maxSpeed, FpvClient.speed);
            peak[0] = Math.max(peak[0], FpvClient.attitude()[1]);
            return tickInStep >= 22;
        });
        add(once(() -> {
            Entity b = entity(droneB);
            double along = (b.posX - start[0]) * dirX + (b.posZ - start[2]) * dirZ;
            check("W leans the drone forward up to the max tilt", peak[0] > 25f && peak[0] <= Config.maxTilt + 0.5,
                String.format("peak pitch %.1f deg (max %.0f)", peak[0], Config.maxTilt));
            check("W flies the drone forward", along > 3, String.format("moved %.2f blocks along heading", along));
            check("HUD speed telemetry is live", maxSpeed > 1.0, String.format("peak %.2f m/s", maxSpeed));
            check("piloted drone beats OC's stock 8 m/s cap", maxSpeed > 9.0, String.format("peak %.2f m/s", maxSpeed));
            shot("04_fpv_flying");
            glide[0] = b.posX; glide[1] = b.posY; glide[2] = b.posZ;
            KeyBinding.setKeyBindState(mc.gameSettings.keyBindForward.getKeyCode(), false);
        }));
        add(waitTicks(20));
        add(once(() -> {
            Entity b = entity(droneB);
            double coasted = (b.posX - glide[0]) * dirX + (b.posZ - glide[2]) * dirZ;
            check("drone glides after release (inertia, not an instant stop)", coasted > 1.5, String.format("coasted %.2f blocks", coasted));
        }));
        add(waitTicks(70));
        double[] rest = new double[3];
        add(once(() -> {
            float[] att = FpvClient.attitude();
            check("drone levels itself when the sticks are released", Math.abs(att[1]) < 1.5f && Math.abs(att[2]) < 1.5f,
                String.format("pitch %.2f roll %.2f", att[1], att[2]));
            Entity b = entity(droneB);
            rest[0] = b.posX; rest[1] = b.posY; rest[2] = b.posZ;
        }));
        add(waitTicks(20));
        add(once(() -> {
            Entity b = entity(droneB);
            double drift = Math.sqrt(Math.pow(b.posX - rest[0], 2) + Math.pow(b.posY - rest[1], 2) + Math.pow(b.posZ - rest[2], 2));
            check("drone hovers once it has settled", drift < 0.15, String.format("drift %.3f", drift));
            start[0] = b.posX; start[2] = b.posZ;
            peak[1] = 0;
            KeyBinding.setKeyBindState(mc.gameSettings.keyBindRight.getKeyCode(), true);
        }));
        add(() -> {
            peak[1] = Math.max(peak[1], FpvClient.attitude()[2]);
            return tickInStep >= 20;
        });
        add(once(() -> {
            Entity b = entity(droneB);
            double[] right = {-dirZ, dirX};
            double sideways = (b.posX - start[0]) * right[0] + (b.posZ - start[2]) * right[1];
            check("D banks the drone to the right", peak[1] > 25f, String.format("peak roll %.1f deg", peak[1]));
            check("banking right moves the drone to its right", sideways > 1.0, String.format("moved %.2f blocks right", sideways));
            check("camera roll follows the bank", FpvClient.viewRoll(0f) > 20f, String.format("view roll %.1f", FpvClient.viewRoll(0f)));
            shot("05b_fpv_bank");
            KeyBinding.setKeyBindState(mc.gameSettings.keyBindRight.getKeyCode(), false);
        }));
        add(waitTicks(80));
        float[] yaw0 = new float[1];
        add(once(() -> {
            yaw0[0] = FpvClient.attitude()[0];
            KeyBinding.setKeyBindState(FpvClient.KEY_YAW_RIGHT.getKeyCode(), true);
        }));
        add(waitTicks(20));
        add(once(() -> {
            float turned = MathHelperWrap(FpvClient.attitude()[0] - yaw0[0]);
            check("arrow key yaws the drone at the configured rate", turned > 60f && turned < 110f, String.format("turned %.1f deg", turned));
            FpvClient.Cam cam = FpvClient.current();
            float expect = FpvClient.attitude()[0] + (cam == null ? 0f : cam.pan);
            // the view is interpolated between ticks and software rendering skips frames: allow ~2.5 ticks of turning
            check("camera heading follows the drone's yaw", Math.abs(MathHelperWrap(FpvClient.viewYaw() - expect)) < 2.5f * (float) Config.yawRate + 1f,
                FpvClient.viewYaw() + " vs " + expect);
            check("the drone model turns to its heading (visible to everyone)",
                Math.abs(MathHelperWrap(entity(droneB).rotationYaw - FpvClient.attitude()[0])) < 10f,
                entity(droneB).rotationYaw + " vs " + FpvClient.attitude()[0]);
            KeyBinding.setKeyBindState(FpvClient.KEY_YAW_RIGHT.getKeyCode(), false);
        }));
        add(waitTicks(40));
        // ---- acro mode: rate control, no self-levelling
        float[] acroPeak = new float[2];
        add(once(() -> {
            FpvClient.setAcro(true);
            acroPeak[0] = acroPeak[1] = 0;
            KeyBinding.setKeyBindState(mc.gameSettings.keyBindForward.getKeyCode(), true);
        }));
        add(() -> {
            float[] a = FpvClient.attitude();
            acroPeak[0] = Math.max(acroPeak[0], Math.abs(MathHelperWrap(a[1])));
            float vp = FpvClient.viewPitch(0f);
            acroPeak[1] = Math.max(acroPeak[1], Math.abs(vp));
            return tickInStep >= 15;
        });
        add(once(() -> {
            check("acro pitches far beyond the angle-mode limit", acroPeak[0] > 90f, String.format("peak pitch %.1f deg", acroPeak[0]));
            check("camera pitch stays renderable (flip-safe)", acroPeak[1] <= 90.01f, String.format("peak view pitch %.1f", acroPeak[1]));
            KeyBinding.setKeyBindState(mc.gameSettings.keyBindForward.getKeyCode(), false);
        }));
        add(waitTicks(25));
        add(once(() -> {
            float p = Math.abs(MathHelperWrap(FpvClient.attitude()[1]));
            check("acro does NOT self-level when the stick is released", p > 30f, String.format("pitch still %.1f deg", p));
            FpvClient.setAcro(false);
        }));
        add(waitTicks(120));
        add(once(() -> {
            float[] a = FpvClient.attitude();
            check("switching back to angle mode levels the drone again", Math.abs(MathHelperWrap(a[1])) < 2f && Math.abs(MathHelperWrap(a[2])) < 2f,
                String.format("pitch %.2f roll %.2f", a[1], a[2]));
        }));
        add(waitTicks(60));
        add(once(() -> {
            Entity b = entity(droneB);
            start[1] = b.posY;
            KeyBinding.setKeyBindState(mc.gameSettings.keyBindJump.getKeyCode(), true);
        }));
        add(waitTicks(30));
        add(once(() -> {
            Entity b = entity(droneB);
            check("Space climbs", b.posY - start[1] > 1.0, String.format("+%.2f", b.posY - start[1]));
            start[2] = b.posY;   // where the descent starts from
            KeyBinding.setKeyBindState(mc.gameSettings.keyBindJump.getKeyCode(), false);
            KeyBinding.setKeyBindState(mc.gameSettings.keyBindSneak.getKeyCode(), true);
        }));
        add(waitTicks(30));
        add(once(() -> {
            Entity b = entity(droneB);
            check("Shift descends", b.posY < start[2] - 1.0, String.format("y %.2f, was %.2f when Shift was pressed", b.posY, start[2]));
            KeyBinding.setKeyBindState(mc.gameSettings.keyBindSneak.getKeyCode(), false);
        }));
        add(waitTicks(40));

        // ---- 11. weak signal and signal loss
        add(once(() -> teleportPilot(0.75)));
        add(waitTicks(60));
        add(once(() -> {
            check("signal quality drops with distance", FpvClient.quality > 0.3f && FpvClient.quality < 0.7f, "q=" + FpvClient.quality);
            check("still viewing at 75% range", FpvClient.isViewing(), "");
            shot("09_fpv_weak_signal");
            teleportPilot(0.93);
        }));
        add(waitTicks(50));
        add(once(() -> {
            check("signal nearly gone at 93% range", FpvClient.quality < 0.2f, "q=" + FpvClient.quality);
            shot("10_fpv_very_weak");
            teleportPilot(1.15);
        }));
        add(() -> until(() -> !FpvClient.isViewing(), 200, "signal-loss exit"));
        add(once(() -> {
            check("server ended the session when the signal was lost", !FpvClient.isViewing()
                && mc.getRenderViewEntity() == mc.player, "");
            check("shader cleared on exit", !mc.entityRenderer.isShaderActive(), "");
            check("signal loss triggers the return-home failsafe", cam(droneB) != null && cam(droneB).isReturning(), "");
            teleportPilot(0.2);
        }));
        add(waitTicks(60));

        // ---- 11b. long range: pilot ~300 blocks from drone E (Antenna Mk3 = 800 blocks)
        add(once(() -> {
            try {
                onServer(() -> {
                    EntityPlayerMP p = server().getPlayerList().getPlayers().get(0);
                    p.setPositionAndUpdate(0.5, 4, 0.5);
                    return null;
                });
            } catch (Exception ex) {
                check("go home", false, ex.toString());
            }
        }));
        add(waitTicks(60));
        add(once(() -> {
            try {
                Entity e = entity(droneE);
                double d = mc.player.getDistance(e.posX, e.posY, e.posZ);
                check("drone E is ~300 blocks from the pilot", d > 250 && d < 350, String.format("%.1f blocks", d));
                check("without a link the client has never heard of drone E's entity", mc.world.getEntityByID(droneE) == null, "");
                FpvClient.Cam c = FpvClient.get(droneE);
                check("client knows E's camera and its range (state broadcast)", c != null && Math.abs(c.range - 800f) < 0.1,
                    c == null ? "no state" : "range " + c.range);
                check("Lua getRange() on E reports the Mk3 antenna", ((Number) invoke(droneE, "getRange")[0]).doubleValue() == 800.0, "");
                FpvClient.enter(droneE);
            } catch (Exception ex) {
                check("long-range setup", false, ex.toString());
            }
        }));
        add(() -> until(FpvClient::isFlying, 400, "long-range link to come up"));
        add(waitTicks(50));
        add(once(() -> {
            try {
                Entity e = mc.world.getEntityByID(droneE);
                check("FPV is up on the far drone", FpvClient.isFlying() && FpvClient.viewingId() == droneE, "viewing=" + FpvClient.viewingId());
                check("pilot's camera is the far drone entity", mc.getRenderViewEntity() != null && mc.getRenderViewEntity().getEntityId() == droneE, "");
                check("full signal at 300 of 800 blocks", FpvClient.quality > 0.99f, "q=" + FpvClient.quality + " dist=" + FpvClient.distance);
                boolean pigSeen = false;
                if (e != null) {
                    for (Entity x : mc.world.loadedEntityList) {
                        if (x instanceof EntityPig && x.getDistance(e) < 30) pigSeen = true;
                    }
                }
                check("a pig beside the far drone is streamed to the pilot", pigSeen, "");
                int[] st = onServer(() -> ViewerManager.linkStats(mc.player.getUniqueID()));
                check("server streams chunks around the drone to the pilot", st != null && st[0] > 20, st == null ? "no session" : "watched=" + st[0]);
                check("server boosted tracking of the drone and nearby entities", st != null && st[1] >= 2, st == null ? "no session" : "boosted=" + st[1]);
                shot("11_fpv_longrange");
            } catch (Exception ex) {
                check("long-range checks", false, ex.toString());
            }
        }));
        add(once(() -> { // 700 of 800 blocks: weak signal, control lag, lost packets
            try {
                Entity e = entity(droneE);
                onServer(() -> {
                    EntityPlayerMP p = server().getPlayerList().getPlayers().get(0);
                    p.setPositionAndUpdate(e.posX - 700, 4, e.posZ);
                    return null;
                });
            } catch (Exception ex) {
                check("teleport to 700 blocks", false, ex.toString());
            }
        }));
        add(waitTicks(140));
        add(once(() -> {
            check("still linked at 700 of 800 blocks", FpvClient.isFlying() && FpvClient.viewingId() == droneE, "viewing=" + FpvClient.viewingId());
            check("signal is weak there", FpvClient.quality > 0.05f && FpvClient.quality < 0.45f, "q=" + FpvClient.quality);
            check("control lag appears with the weak signal", FpvClient.lagTicks >= 5, "lag=" + FpvClient.lagTicks + " ticks");
            check("some control packets are lost", FpvClient.packetsDropped > 0, "dropped=" + FpvClient.packetsDropped + " sent=" + FpvClient.packetsSent);
            shot("12_fpv_longrange_weak");
            FpvClient.exit(true, null);
        }));
        add(waitTicks(30));
        add(once(() -> {
            try {
                boolean released = onServer(() -> ViewerManager.linkStats(mc.player.getUniqueID()) == null);
                check("long-range link is released when FPV ends", released, "");
                boolean notTracked = onServer(() -> !server().getWorld(0).getEntityTracker().getTrackingPlayers(entity(droneE))
                    .contains(server().getPlayerList().getPlayers().get(0)));
                check("far drone is no longer tracked once the link is gone", notTracked, "");
            } catch (Exception ex) {
                check("link release", false, ex.toString());
            }
            teleportPilot(0.2);
        }));
        add(waitTicks(80));

        // ---- 11c. the failsafe must have flown drone B back to the home point it recorded at start()
        add(() -> until(() -> cam(droneB) != null && !cam(droneB).isReturning(), 1500, "return-home to finish"));
        add(waitTicks(20));
        add(once(() -> {
            try {
                CameraEnvironment env = cam(droneB);
                Entity b = entity(droneB);
                double[] h = env.home();
                double d = Math.sqrt(Math.pow(b.posX - h[0], 2) + Math.pow(b.posY - h[1], 2) + Math.pow(b.posZ - h[2], 2));
                check("drone flew back to its home point and stopped", env.hasHome() && d < 3.0, String.format("%.2f blocks from home", d));
                check("camera stays on after returning home", env.isActive(), "");
                Object[] r = invoke(droneB, "isReturning");
                check("Lua isReturning() is false once home", r != null && Boolean.FALSE.equals(r[0]), java.util.Arrays.toString(r));
            } catch (Exception ex) {
                check("return home", false, ex.toString());
            }
        }));

        // ---- 11d. waypoint autopilot on drone C
        double[] wp = new double[3];
        add(once(() -> {
            try {
                Entity c = entity(droneC);
                wp[0] = c.posX + 8; wp[1] = c.posY + 1; wp[2] = c.posZ;
                check("Lua flyTo accepts a waypoint", Boolean.TRUE.equals(invoke(droneC, "flyTo", wp[0], wp[1], wp[2])[0]), "");
            } catch (Exception ex) {
                check("flyTo", false, ex.toString());
            }
        }));
        add(() -> until(() -> cam(droneC) != null && !cam(droneC).isAutopilot(), 900, "flyTo to finish"));
        add(waitTicks(20));
        add(once(() -> {
            Entity c = entity(droneC);
            double d = Math.sqrt(Math.pow(c.posX - wp[0], 2) + Math.pow(c.posY - wp[1], 2) + Math.pow(c.posZ - wp[2], 2));
            check("drone reached the waypoint and hovers there", d < 3.0, String.format("%.2f blocks from the waypoint", d));
        }));

        // ---- 12. Esc leaves FPV; camera-off ends FPV; drone cycling
        add(once(() -> FpvClient.toggle()));
        add(() -> until(FpvClient::isFlying, 100, "FPV restart"));
        add(waitTicks(10));
        add(once(() -> mc.displayGuiScreen(new GuiIngameMenu())));
        add(waitTicks(5));
        add(once(() -> {
            check("Esc leaves FPV instead of opening the pause menu",
                !FpvClient.isViewing() && !(mc.currentScreen instanceof GuiIngameMenu), "screen=" + mc.currentScreen);
            FpvClient.toggle();
        }));
        add(() -> until(FpvClient::isFlying, 100, "FPV restart 2"));
        add(waitTicks(10));
        add(once(() -> {
            int first = FpvClient.viewingId();
            FpvClient.cycle();
            log("cycled from " + first + " to " + FpvClient.viewingId());
            check("cycling switches to the other camera drone",
                FpvClient.viewingId() >= 0 && FpvClient.viewingId() != first
                    && (FpvClient.viewingId() == droneB || FpvClient.viewingId() == droneC || FpvClient.viewingId() == droneE),
                    "now " + FpvClient.viewingId());
        }));
        add(waitTicks(30));
        add(once(() -> {
            try {
                int now = FpvClient.viewingId();
                invoke(now, "stop");
            } catch (Exception ex) {
                check("camera stop", false, ex.toString());
            }
        }));
        add(() -> until(() -> !FpvClient.isViewing(), 100, "camera-off exit"));
        add(once(() -> check("switching the camera off ends FPV", !FpvClient.isViewing()
            && mc.getRenderViewEntity() == mc.player, "")));

        // ---- 12b. goggles requirement (client and server both enforce it)
        add(once(() -> {
            Config.requireGoggles = true;
            FpvClient.toggle();
        }));
        add(waitTicks(20));
        add(once(() -> {
            check("client refuses to fly without goggles when they are required", !FpvClient.isViewing(), "");
            // ask the server directly, bypassing the client check
            com.openfpv.net.Net.CHANNEL.sendToServer(new com.openfpv.net.PacketView(droneB, true));
        }));
        add(waitTicks(20));
        add(once(() -> {
            try {
                check("server refuses a link without goggles", !onServer(() -> ViewerManager.isViewing(mc.player.getUniqueID())), "");
                onServer(() -> {
                    server().getPlayerList().getPlayers().get(0).setItemStackToSlot(
                        net.minecraft.inventory.EntityEquipmentSlot.HEAD, new ItemStack(ModItems.GOGGLES));
                    return null;
                });
            } catch (Exception ex) {
                check("goggles refusal", false, ex.toString());
            }
        }));
        add(waitTicks(10));
        add(once(() -> FpvClient.toggle()));
        add(() -> until(FpvClient::isFlying, 300, "FPV with goggles"));
        add(once(() -> {
            check("with goggles on, the pilot can fly", FpvClient.isFlying(), "");
            FpvClient.exit(true, null);
            Config.requireGoggles = false;
        }));
        add(waitTicks(20));

        // ---- 12c. in-game config screen + live reload
        add(once(() -> {
            try {
                net.minecraft.client.gui.GuiScreen gui = new com.openfpv.client.OpenFPVGuiFactory().createConfigGui(null);
                mc.displayGuiScreen(gui);
                check("config screen opens", mc.currentScreen instanceof net.minecraftforge.fml.client.config.GuiConfig, "");
            } catch (Throwable t) {
                check("config screen opens", false, t.toString());
            }
        }));
        add(waitTicks(15));
        add(once(() -> {
            shot("13_config_gui");
            mc.displayGuiScreen(null);
            double before = Config.maxTilt;
            Config.cfg.getCategory("flight").get("maxTilt").set(41.0);
            Config.read();
            check("edited config values apply live", Config.maxTilt == 41.0, "maxTilt=" + Config.maxTilt);
            Config.cfg.getCategory("flight").get("maxTilt").set(before);
            Config.read();
        }));

        // ---- 13. finish
        add(() -> {
            if (tickInStep < 10) return false;
            finish();
            return true;
        });
    }

    private static float MathHelperWrap(float d) {
        d %= 360f;
        if (d >= 180f) d -= 360f;
        if (d < -180f) d += 360f;
        return d;
    }

    private void spawnMob(World w, EntityLiving e, double x, double y, double z) {
        e.setLocationAndAngles(x, y, z, 0, 0);
        e.setNoAI(true);
        e.setEntityInvulnerable(true);
        e.enablePersistence();
        w.spawnEntity(e);
    }

    private void modes(boolean night, boolean thermal, String unused) {
        try {
            invoke(droneB, "setNightVision", night);
            invoke(droneB, "setThermal", thermal);
        } catch (Exception ex) {
            check("mode switch", false, ex.toString());
        }
    }

    /** Moves the pilot so the pilot-to-drone distance is the given fraction of the signal range. */
    private void teleportPilot(double fraction) {
        try {
            Entity b = entity(droneB);
            double d = FpvClient.range() * fraction;
            onServer(() -> {
                EntityPlayerMP p = server().getPlayerList().getPlayers().get(0);
                p.setPositionAndUpdate(b.posX - dirX * d, 4, b.posZ - dirZ * d);
                return null;
            });
        } catch (Exception ex) {
            check("teleport pilot", false, ex.toString());
        }
    }

    private void eq(Map<String, String> m, String key, String expected) {
        String got = m.get(key);
        check("Lua " + key, expected.equals(got), "expected " + expected + " got " + got);
    }

    private void finish() {
        StringBuilder sb = new StringBuilder();
        for (String r : results) sb.append(r).append('\n');
        long passed = results.stream().filter(r -> r.startsWith("PASS")).count();
        sb.append("\nTOTAL ").append(results.size()).append("  PASS ").append(passed).append("  FAIL ").append(fails).append('\n');
        try {
            Files.write(new File(OUT, "summary.txt").toPath(), sb.toString().getBytes(StandardCharsets.UTF_8));
        } catch (IOException ex) {
            OpenFPV.log.error("could not write summary", ex);
        }
        log("DONE passed=" + passed + " failed=" + fails);
        mc.shutdown();
    }
}
