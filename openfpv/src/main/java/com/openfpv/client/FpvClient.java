package com.openfpv.client;

import com.openfpv.Config;
import com.openfpv.net.Net;
import com.openfpv.net.PacketAttitude;
import com.openfpv.net.PacketCameraState;
import com.openfpv.net.PacketControl;
import com.openfpv.net.PacketView;
import com.openfpv.net.PacketViewEnd;
import com.openfpv.util.CameraMath;
import com.openfpv.util.ControlLag;
import com.openfpv.util.QuadPhysics;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.OpenGlHelper;
import net.minecraft.client.settings.GameSettings;
import net.minecraft.client.settings.KeyBinding;
import net.minecraft.entity.Entity;
import net.minecraft.entity.player.EntityPlayer;
import net.minecraft.init.MobEffects;
import net.minecraft.potion.PotionEffect;
import net.minecraft.util.ResourceLocation;
import net.minecraft.util.text.TextComponentTranslation;
import net.minecraft.world.chunk.Chunk;
import net.minecraftforge.fml.relauncher.Side;
import net.minecraftforge.fml.relauncher.SideOnly;
import org.lwjgl.input.Keyboard;

/** Client-side FPV state machine: which drone is being flown, sticks, camera attitude, shaders and effects. */
@SideOnly(Side.CLIENT)
public final class FpvClient {
    private FpvClient() {}

    public static final KeyBinding KEY_TOGGLE =
        new KeyBinding("key.openfpv.toggle", Keyboard.KEY_V, "key.categories.openfpv");
    public static final KeyBinding KEY_NEXT =
        new KeyBinding("key.openfpv.next", Keyboard.KEY_B, "key.categories.openfpv");
    public static final KeyBinding KEY_YAW_LEFT =
        new KeyBinding("key.openfpv.yaw_left", Keyboard.KEY_LEFT, "key.categories.openfpv");
    public static final KeyBinding KEY_YAW_RIGHT =
        new KeyBinding("key.openfpv.yaw_right", Keyboard.KEY_RIGHT, "key.categories.openfpv");

    /** Last known state of a camera drone, as reported by the server. */
    public static final class Cam {
        public int entityId;
        public boolean active, night, thermal;
        public float fov = CameraMath.BASE_FOV, tilt, pan, energy;
        public float range = 64f;
        public int modules;
        public double x, y, z;
        long lastUpdate;
    }

    private static final Map<Integer, Cam> CAMS = new HashMap<>();
    private static final ResourceLocation SHADER_THERMAL = new ResourceLocation("shaders/post/openfpv_thermal.json");
    private static final ResourceLocation SHADER_NIGHT = new ResourceLocation("shaders/post/openfpv_night.json");
    private static final int LINK_TIMEOUT_TICKS = 200;

    private static int viewing = -1;
    /** True while the server is still streaming the drone and its surroundings to us. */
    private static boolean pending;
    private static int pendingTicks;
    private static ResourceLocation loadedShader;
    private static boolean appliedNightEffect;

    // attitude reported by the server (yaw, nose-down pitch, right roll), previous + current for interpolation
    private static float prevYaw, prevPitch, prevRoll, curYaw, curPitch, curRoll;
    private static boolean haveAtt;

    // sticks / link
    private static final ControlLag LAG = new ControlLag(20260101L);
    private static float lastPlayerYaw;
    private static boolean haveLastYaw;
    private static long tick;
    public static int lagTicks;
    public static float[] lastSticks = new float[4];
    public static int packetsSent, packetsDropped;

    // HUD telemetry
    public static double speed;       // blocks per second
    public static double distance;    // pilot <-> drone
    public static float quality = 1f; // signal quality 0..1
    public static int viewTicks;
    private static double lastX, lastY, lastZ;
    private static boolean haveLast;

    // ------------------------------------------------------------------ network callbacks

    public static void onState(PacketCameraState p) {
        Minecraft mc = Minecraft.getMinecraft();
        if (!p.active) {
            CAMS.remove(p.entityId);
            if (p.entityId == viewing) exit(true, "message.openfpv.camera_off");
            return;
        }
        Cam c = CAMS.computeIfAbsent(p.entityId, id -> new Cam());
        c.entityId = p.entityId;
        c.active = true;
        c.night = p.night;
        c.thermal = p.thermal;
        c.fov = p.fov;
        c.tilt = p.tilt;
        c.pan = p.pan;
        c.energy = p.energy;
        c.modules = p.modules;
        c.range = p.range;
        c.x = p.x;
        c.y = p.y;
        c.z = p.z;
        c.lastUpdate = mc.world != null ? mc.world.getTotalWorldTime() : 0;
    }

    public static void onAttitude(PacketAttitude p) {
        if (p.entityId != viewing) return;
        if (!haveAtt) {
            prevYaw = curYaw = p.yaw;
            prevPitch = curPitch = p.pitch;
            prevRoll = curRoll = p.roll;
            haveAtt = true;
        } else {
            prevYaw = curYaw;
            prevPitch = curPitch;
            prevRoll = curRoll;
            curYaw = p.yaw;
            curPitch = p.pitch;
            curRoll = p.roll;
        }
    }

    public static void onServerEnded(int reason) {
        String key;
        switch (reason) {
            case PacketViewEnd.SIGNAL_LOST: key = "message.openfpv.signal_lost"; break;
            case PacketViewEnd.DRONE_GONE: key = "message.openfpv.drone_gone"; break;
            case PacketViewEnd.CAMERA_OFF: key = "message.openfpv.camera_off"; break;
            default: key = "message.openfpv.refused"; break;
        }
        exit(false, key);
    }

    // ------------------------------------------------------------------ queries

    /** True from the moment the link is requested (even while chunks are still streaming in). */
    public static boolean isViewing() {
        return viewing >= 0;
    }

    /** True once the camera is actually showing the drone's view. */
    public static boolean isFlying() {
        return viewing >= 0 && !pending;
    }

    public static boolean isPending() {
        return pending;
    }

    public static int viewingId() {
        return viewing;
    }

    public static Cam current() {
        return viewing >= 0 ? CAMS.get(viewing) : null;
    }

    public static Cam get(int entityId) {
        return CAMS.get(entityId);
    }

    /** Signal range of the drone being flown (or the no-antenna default). */
    public static double range() {
        Cam c = current();
        return c != null ? c.range : Config.rangeForTier(0);
    }

    private static double distanceTo(Minecraft mc, Cam c) {
        Entity e = mc.world.getEntityByID(c.entityId);
        double x = e != null ? e.posX : c.x, y = e != null ? e.posY : c.y, z = e != null ? e.posZ : c.z;
        double dx = mc.player.posX - x, dy = mc.player.posY - y, dz = mc.player.posZ - z;
        return Math.sqrt(dx * dx + dy * dy + dz * dz);
    }

    /** Drones with an active camera whose signal reaches us. They need not be loaded on this client. */
    public static List<Cam> available() {
        Minecraft mc = Minecraft.getMinecraft();
        List<Cam> out = new ArrayList<>();
        if (mc.world == null || mc.player == null) return out;
        for (Cam c : CAMS.values()) {
            if (c.active && distanceTo(mc, c) <= c.range) out.add(c);
        }
        out.sort((a, b) -> Integer.compare(a.entityId, b.entityId));
        return out;
    }

    /** {yaw, nose-down pitch, right roll} of the drone as last reported by the server. */
    public static float[] attitude() {
        return new float[]{curYaw, curPitch, curRoll};
    }

    // ------------------------------------------------------------------ view control

    public static void toggle() {
        Minecraft mc = Minecraft.getMinecraft();
        if (isViewing()) {
            exit(true, null);
            return;
        }
        List<Cam> list = available();
        if (list.isEmpty()) {
            status("message.openfpv.no_camera");
            return;
        }
        Cam nearest = list.get(0);
        double best = Double.MAX_VALUE;
        for (Cam c : list) {
            double d = distanceTo(mc, c);
            if (d < best) {
                best = d;
                nearest = c;
            }
        }
        enter(nearest.entityId);
    }

    public static void cycle() {
        if (!isViewing()) {
            toggle();
            return;
        }
        List<Cam> list = available();
        if (list.size() < 2) {
            status("message.openfpv.only_camera");
            return;
        }
        int idx = 0;
        for (int i = 0; i < list.size(); i++) {
            if (list.get(i).entityId == viewing) idx = i;
        }
        int next = list.get((idx + 1) % list.size()).entityId;
        Net.CHANNEL.sendToServer(new PacketView(viewing, false));
        leaveView();
        enter(next);
    }

    /** Asks the server for the link. The camera switches over once the drone and its chunks have arrived. */
    public static void enter(int entityId) {
        viewing = entityId;
        pending = true;
        pendingTicks = 0;
        viewTicks = 0;
        haveLast = false;
        haveAtt = false;
        speed = 0;
        lagTicks = 0;
        packetsSent = packetsDropped = 0;
        LAG.clear();
        loadedShader = null;
        Net.CHANNEL.sendToServer(new PacketView(entityId, true));
        status("message.openfpv.linking");
    }

    private static void leaveView() {
        Minecraft mc = Minecraft.getMinecraft();
        viewing = -1;
        pending = false;
        if (mc.player != null) mc.setRenderViewEntity(mc.player);
        stopShader();
        removeNightEffect();
        quality = 1f;
        lagTicks = 0;
        LAG.clear();
    }

    /** Leave the FPV view. {@code notifyServer} is false when the server is the one that ended it. */
    public static void exit(boolean notifyServer, String messageKey) {
        if (viewing < 0) return;
        int id = viewing;
        leaveView();
        if (notifyServer) Net.CHANNEL.sendToServer(new PacketView(id, false));
        if (messageKey != null) status(messageKey);
    }

    public static void reset() {
        CAMS.clear();
        if (viewing >= 0) leaveView();
    }

    private static void status(String key) {
        Minecraft mc = Minecraft.getMinecraft();
        if (mc.player != null) mc.player.sendStatusMessage(new TextComponentTranslation(key), true);
    }

    // ------------------------------------------------------------------ per-tick work

    public static void clientTick() {
        Minecraft mc = Minecraft.getMinecraft();
        if (mc.world == null || mc.player == null) {
            if (viewing >= 0 || !CAMS.isEmpty()) reset();
            return;
        }
        tick++;
        long now = mc.world.getTotalWorldTime();
        CAMS.values().removeIf(c -> now - c.lastUpdate > 100);

        while (KEY_TOGGLE.isPressed()) toggle();
        while (KEY_NEXT.isPressed()) cycle();

        if (viewing < 0) return;
        Cam cam = CAMS.get(viewing);
        if (cam == null || !cam.active) {
            exit(true, "message.openfpv.camera_off");
            return;
        }
        Entity e = mc.world.getEntityByID(viewing);

        if (pending) {
            if (e != null && !e.isDead && haveAtt && chunkLoaded(mc, e)) {
                pending = false;
                mc.setRenderViewEntity(e);
                mc.gameSettings.thirdPersonView = 0;
                haveLastYaw = false;
                status("message.openfpv.entered");
            } else if (++pendingTicks > LINK_TIMEOUT_TICKS) {
                exit(true, "message.openfpv.link_failed");
            }
            return;
        }
        if (e == null || e.isDead) {
            exit(true, "message.openfpv.drone_gone");
            return;
        }
        viewTicks++;
        if (mc.getRenderViewEntity() != e) mc.setRenderViewEntity(e);

        // telemetry: OC's drone resets prevPos itself, so track the position between our own ticks
        if (haveLast) {
            double dx = e.posX - lastX, dy = e.posY - lastY, dz = e.posZ - lastZ;
            double instant = Math.sqrt(dx * dx + dy * dy + dz * dz) * 20.0;
            speed = speed * 0.6 + instant * 0.4;
        } else {
            speed = 0;
        }
        lastX = e.posX;
        lastY = e.posY;
        lastZ = e.posZ;
        haveLast = true;
        distance = mc.player.getDistance(e);
        quality = CameraMath.signalQuality(distance, cam.range);

        updateShader(cam);
        updateNightEffect(mc.player, cam);
        sendControl(mc, e);
    }

    private static boolean chunkLoaded(Minecraft mc, Entity e) {
        Chunk c = mc.world.getChunkProvider().getLoadedChunk(((int) Math.floor(e.posX)) >> 4, ((int) Math.floor(e.posZ)) >> 4);
        return c != null && !c.isEmpty();
    }

    /** Reads the sticks, pushes them through the (possibly laggy, lossy) link and sends what arrives. */
    private static void sendControl(Minecraft mc, Entity drone) {
        GameSettings gs = mc.gameSettings;
        EntityPlayer p = mc.player;
        float pitch = (gs.keyBindForward.isKeyDown() ? 1 : 0) - (gs.keyBindBack.isKeyDown() ? 1 : 0);
        float roll = (gs.keyBindRight.isKeyDown() ? 1 : 0) - (gs.keyBindLeft.isKeyDown() ? 1 : 0);
        float throttle = (gs.keyBindJump.isKeyDown() ? 1 : 0) - (gs.keyBindSneak.isKeyDown() ? 1 : 0);
        float yawKeys = (KEY_YAW_RIGHT.isKeyDown() ? 1 : 0) - (KEY_YAW_LEFT.isKeyDown() ? 1 : 0);

        // The mouse turns the drone: whatever the vanilla camera code turned the player by this tick is a yaw command.
        float mouse = 0f;
        if (haveLastYaw) mouse = CameraMath.clamp(QuadPhysics.wrapDegrees(p.rotationYaw - lastPlayerYaw) / (float) Config.yawRate, -1f, 1f);
        lastPlayerYaw = p.rotationYaw;
        haveLastYaw = true;
        p.rotationPitch = p.prevRotationPitch = 0f;   // the drone has no free-look; keep the pilot's own pitch quiet

        float yaw = CameraMath.clamp(yawKeys + mouse, -1f, 1f);
        ControlLag.Sticks sticks = new ControlLag.Sticks(pitch, roll, yaw, throttle);
        lastSticks = new float[]{pitch, roll, yaw, throttle};

        ControlLag.Sticks out;
        if (Config.controlLag) {
            lagTicks = ControlLag.lagTicks(quality, Config.maxLagTicks);
            if (!LAG.push(tick, sticks, quality, Config.maxLagTicks, Config.maxPacketLoss)) packetsDropped++;
            out = LAG.poll(tick);
        } else {
            lagTicks = 0;
            out = sticks;
        }
        if (out != null) {
            Net.CHANNEL.sendToServer(new PacketControl(drone.getEntityId(), out));
            packetsSent++;
        }
    }

    // ------------------------------------------------------------------ vision modes

    private static void updateShader(Cam cam) {
        ResourceLocation wanted = cam.thermal ? SHADER_THERMAL : (cam.night ? SHADER_NIGHT : null);
        Minecraft mc = Minecraft.getMinecraft();
        if (!OpenGlHelper.shadersSupported) return;
        if (wanted == null) {
            if (loadedShader != null) stopShader();
            return;
        }
        if (!wanted.equals(loadedShader) || !mc.entityRenderer.isShaderActive()) {
            mc.entityRenderer.loadShader(wanted);
            loadedShader = wanted;
        }
    }

    private static void stopShader() {
        Minecraft mc = Minecraft.getMinecraft();
        if (loadedShader != null || mc.entityRenderer.isShaderActive()) {
            mc.entityRenderer.stopUseShader();
        }
        loadedShader = null;
    }

    /** Vanilla only brightens the world for the real player's night-vision effect, so borrow it. */
    private static void updateNightEffect(EntityPlayer player, Cam cam) {
        boolean want = cam.night || cam.thermal;
        if (want) {
            PotionEffect cur = player.getActivePotionEffect(MobEffects.NIGHT_VISION);
            if (cur == null || cur.getDuration() < 300) {
                player.addPotionEffect(new PotionEffect(MobEffects.NIGHT_VISION, 600, 0, false, false));
            }
            appliedNightEffect = true;
        } else {
            removeNightEffect();
        }
    }

    private static void removeNightEffect() {
        Minecraft mc = Minecraft.getMinecraft();
        if (appliedNightEffect && mc.player != null) {
            mc.player.removeActivePotionEffect(MobEffects.NIGHT_VISION);
        }
        appliedNightEffect = false;
    }

    // ------------------------------------------------------------------ render-frame work

    /** Pitch (nose-down, camera mounting and gimbal included) of the view, in Minecraft degrees. */
    public static float viewPitch(float partialTicks) {
        Cam cam = current();
        float p = prevPitch + (curPitch - prevPitch) * partialTicks;
        float tilt = cam != null ? cam.tilt : 0f;
        return CameraMath.clamp(p - (float) Config.cameraUptilt - tilt, -90f, 90f);
    }

    public static float viewYawAngle(float partialTicks) {
        Cam cam = current();
        return QuadPhysics.lerpAngle(prevYaw, curYaw, partialTicks) + (cam != null ? cam.pan : 0f);
    }

    /** Camera roll in degrees for the render hook; zero unless flying. */
    public static float viewRoll(float partialTicks) {
        if (!isFlying()) return 0f;
        return prevRoll + (curRoll - prevRoll) * partialTicks;
    }

    /** Aims the drone entity's camera before the world is drawn: the drone's own attitude plus gimbal offsets. */
    public static void aimCamera(float partialTicks) {
        Minecraft mc = Minecraft.getMinecraft();
        if (!isFlying() || mc.player == null) return;
        Entity e = mc.getRenderViewEntity();
        if (e == null || e == mc.player || current() == null) return;
        float yaw = viewYawAngle(partialTicks);
        float pitch = viewPitch(partialTicks);
        e.rotationYaw = e.prevRotationYaw = yaw;
        e.rotationPitch = e.prevRotationPitch = pitch;
    }

    /** Heading of the view in Minecraft yaw degrees (for the compass). */
    public static float viewYaw() {
        Minecraft mc = Minecraft.getMinecraft();
        Entity e = mc.getRenderViewEntity();
        return e == null ? 0f : e.rotationYaw;
    }
}
