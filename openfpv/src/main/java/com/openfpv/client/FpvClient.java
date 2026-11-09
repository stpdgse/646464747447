package com.openfpv.client;

import com.openfpv.Config;
import com.openfpv.net.Net;
import com.openfpv.net.PacketCameraState;
import com.openfpv.net.PacketControl;
import com.openfpv.net.PacketView;
import com.openfpv.net.PacketViewEnd;
import com.openfpv.util.CameraMath;
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
import net.minecraftforge.fml.relauncher.Side;
import net.minecraftforge.fml.relauncher.SideOnly;
import org.lwjgl.input.Keyboard;

/** Client-side FPV state machine: which drone is being flown, input, shaders and effects. */
@SideOnly(Side.CLIENT)
public final class FpvClient {
    private FpvClient() {}

    public static final KeyBinding KEY_TOGGLE =
        new KeyBinding("key.openfpv.toggle", Keyboard.KEY_V, "key.categories.openfpv");
    public static final KeyBinding KEY_NEXT =
        new KeyBinding("key.openfpv.next", Keyboard.KEY_B, "key.categories.openfpv");

    /** Last known state of a camera drone, as reported by the server. */
    public static final class Cam {
        public int entityId;
        public boolean active, night, thermal;
        public float fov = CameraMath.BASE_FOV, tilt, pan, energy;
        public int modules;
        long lastUpdate;
    }

    private static final Map<Integer, Cam> CAMS = new HashMap<>();
    private static final ResourceLocation SHADER_THERMAL = new ResourceLocation("shaders/post/openfpv_thermal.json");
    private static final ResourceLocation SHADER_NIGHT = new ResourceLocation("shaders/post/openfpv_night.json");

    private static int viewing = -1;
    private static ResourceLocation loadedShader;
    private static boolean appliedNightEffect;

    // Flight input bookkeeping
    private static double lastDx, lastDy, lastDz;
    private static int sinceControl;

    // HUD telemetry (client side, measured from the drone entity)
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
        c.lastUpdate = mc.world != null ? mc.world.getTotalWorldTime() : 0;
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

    public static boolean isViewing() {
        return viewing >= 0;
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

    /** Effective signal range: config range, but never beyond what the client has chunks for. */
    public static double range() {
        Minecraft mc = Minecraft.getMinecraft();
        double chunkLimit = Math.max(1, mc.gameSettings.renderDistanceChunks - 1) * 16.0;
        return Math.min(Config.maxRange, chunkLimit);
    }

    /** Drones that have an active camera, are loaded on this client and are within signal range. */
    public static List<Cam> available() {
        Minecraft mc = Minecraft.getMinecraft();
        List<Cam> out = new ArrayList<>();
        if (mc.world == null || mc.player == null) return out;
        double range = range();
        for (Cam c : CAMS.values()) {
            Entity e = mc.world.getEntityByID(c.entityId);
            if (c.active && e != null && !e.isDead && mc.player.getDistance(e) <= range) out.add(c);
        }
        out.sort((a, b) -> Integer.compare(a.entityId, b.entityId));
        return out;
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
            double d = mc.player.getDistanceSq(mc.world.getEntityByID(c.entityId));
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
        enter(next);
    }

    public static void enter(int entityId) {
        Minecraft mc = Minecraft.getMinecraft();
        Entity e = mc.world == null ? null : mc.world.getEntityByID(entityId);
        if (e == null) return;
        viewing = entityId;
        viewTicks = 0;
        haveLast = false;
        speed = 0;
        lastDx = lastDy = lastDz = 0;
        sinceControl = 0;
        loadedShader = null;
        Net.CHANNEL.sendToServer(new PacketView(entityId, true));
        mc.setRenderViewEntity(e);
        mc.gameSettings.thirdPersonView = 0;
        status("message.openfpv.entered");
    }

    /** Leave the FPV view. {@code notifyServer} is false when the server is the one that ended it. */
    public static void exit(boolean notifyServer, String messageKey) {
        Minecraft mc = Minecraft.getMinecraft();
        if (viewing < 0) return;
        int id = viewing;
        viewing = -1;
        if (notifyServer) Net.CHANNEL.sendToServer(new PacketView(id, false));
        if (mc.player != null) mc.setRenderViewEntity(mc.player);
        stopShader();
        removeNightEffect();
        quality = 1f;
        if (messageKey != null) status(messageKey);
    }

    public static void reset() {
        CAMS.clear();
        if (viewing >= 0) {
            viewing = -1;
            Minecraft mc = Minecraft.getMinecraft();
            if (mc.player != null) mc.setRenderViewEntity(mc.player);
            stopShader();
            removeNightEffect();
        }
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
        long now = mc.world.getTotalWorldTime();
        CAMS.values().removeIf(c -> now - c.lastUpdate > 100);

        while (KEY_TOGGLE.isPressed()) toggle();
        while (KEY_NEXT.isPressed()) cycle();

        if (viewing < 0) return;
        Entity e = mc.world.getEntityByID(viewing);
        Cam cam = CAMS.get(viewing);
        if (e == null || e.isDead) {
            exit(true, "message.openfpv.drone_gone");
            return;
        }
        if (cam == null || !cam.active) {
            exit(true, "message.openfpv.camera_off");
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
        quality = CameraMath.signalQuality(distance, range());

        updateShader(cam);
        updateNightEffect(mc.player, cam);
        sendControl(mc, e);
    }

    private static void sendControl(Minecraft mc, Entity drone) {
        GameSettings gs = mc.gameSettings;
        double forward = (gs.keyBindForward.isKeyDown() ? 1 : 0) - (gs.keyBindBack.isKeyDown() ? 1 : 0);
        double strafe = (gs.keyBindRight.isKeyDown() ? 1 : 0) - (gs.keyBindLeft.isKeyDown() ? 1 : 0);
        double up = (gs.keyBindJump.isKeyDown() ? 1 : 0) - (gs.keyBindSneak.isKeyDown() ? 1 : 0);
        double[] v = CameraMath.flightVector(mc.player.rotationYaw, forward, strafe, up);
        boolean changed = Math.abs(v[0] - lastDx) > 0.01 || Math.abs(v[1] - lastDy) > 0.01 || Math.abs(v[2] - lastDz) > 0.01;
        boolean moving = Math.abs(v[0]) + Math.abs(v[1]) + Math.abs(v[2]) > 0.01;
        if (changed || (moving && ++sinceControl >= 4)) {
            Net.CHANNEL.sendToServer(new PacketControl(drone.getEntityId(), v[0], v[1], v[2]));
            lastDx = v[0];
            lastDy = v[1];
            lastDz = v[2];
            sinceControl = 0;
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

    /** Aim the drone entity's camera before the world is drawn: mouse look plus gimbal offsets. */
    public static void aimCamera(float partialTicks) {
        Minecraft mc = Minecraft.getMinecraft();
        if (viewing < 0 || mc.player == null) return;
        Entity e = mc.getRenderViewEntity();
        Cam cam = current();
        if (e == null || e == mc.player || cam == null) return;
        EntityPlayer p = mc.player;
        float yaw = p.prevRotationYaw + (p.rotationYaw - p.prevRotationYaw) * partialTicks + cam.pan;
        float pitch = p.prevRotationPitch + (p.rotationPitch - p.prevRotationPitch) * partialTicks - cam.tilt;
        pitch = CameraMath.clamp(pitch, -90f, 90f);
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
