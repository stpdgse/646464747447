package com.openfpv.client;

import com.openfpv.Config;
import com.openfpv.OpenFPV;
import com.openfpv.util.CameraMath;
import java.lang.reflect.Method;
import java.util.HashMap;
import java.util.Map;
import net.minecraft.client.Minecraft;
import net.minecraft.client.renderer.GlStateManager;
import net.minecraft.client.renderer.entity.Render;
import net.minecraft.client.renderer.entity.RenderManager;
import net.minecraft.entity.Entity;
import net.minecraft.util.ResourceLocation;
import net.minecraftforge.fml.relauncher.Side;
import net.minecraftforge.fml.relauncher.SideOnly;

/**
 * Draws OpenComputers drones as an animated quadcopter loaded from the Blockbench model
 * assets/openfpv/models/entity/quadcopter.bbmodel: spinning props (only while powered), a lean into
 * acceleration, a camera pod that follows the gimbal, and blinking navigation lights.
 */
@SideOnly(Side.CLIENT)
public class DroneRenderer extends Render<Entity> {
    public static final ResourceLocation MODEL = new ResourceLocation(OpenFPV.MOD_ID, "models/entity/quadcopter.bbmodel");
    public static final ResourceLocation TEXTURE = new ResourceLocation(OpenFPV.MOD_ID, "textures/entity/quadcopter.png");

    private static BbModel model;
    private static boolean loadFailed;
    private static Method isRunning, lightColor;
    private final Map<Integer, float[]> lean = new HashMap<>();

    public DroneRenderer(RenderManager manager) {
        super(manager);
        shadowSize = 0.35f;
    }

    public static BbModel model() {
        if (model == null && !loadFailed) {
            try {
                model = BbModel.load(MODEL);
                OpenFPV.log.info("Loaded drone model {} ({} cubes)", MODEL, model.cubeCount());
            } catch (Exception ex) {
                loadFailed = true;
                OpenFPV.log.error("Could not load the drone model", ex);
            }
        }
        return model;
    }

    /** Whether the OC drone is powered on (synced to the client by OC). */
    public static boolean running(Entity e) {
        try {
            if (isRunning == null) isRunning = e.getClass().getMethod("isRunning");
            return Boolean.TRUE.equals(isRunning.invoke(e));
        } catch (Exception ex) {
            return false;
        }
    }

    /** The drone's own status LED colour as set from Lua with drone.setLightColor(), 0xRRGGBB. */
    static int statusColor(Entity e) {
        try {
            if (lightColor == null) lightColor = e.getClass().getMethod("lightColor");
            Object v = lightColor.invoke(e);
            return v instanceof Number ? ((Number) v).intValue() & 0xFFFFFF : 0;
        } catch (Exception ex) {
            return 0;
        }
    }

    @Override
    public void doRender(Entity entity, double x, double y, double z, float entityYaw, float partialTicks) {
        BbModel m = model();
        if (m == null) return;
        boolean running = running(entity);
        FpvClient.Cam cam = FpvClient.get(entity.getEntityId());
        float time = Minecraft.getMinecraft().world.getTotalWorldTime() + partialTicks;

        double speed = Math.sqrt(entity.motionX * entity.motionX + entity.motionY * entity.motionY + entity.motionZ * entity.motionZ);
        float yaw = entity.prevRotationYaw + CameraMath.clamp(wrap(entity.rotationYaw - entity.prevRotationYaw), -180f, 180f) * partialTicks;

        // lean into the motion: nose down when moving forward, bank towards the side we drift to
        double sin = Math.sin(Math.toRadians(yaw)), cos = Math.cos(Math.toRadians(yaw));
        double forward = -sin * entity.motionX + cos * entity.motionZ;
        double right = -cos * entity.motionX - sin * entity.motionZ;
        float[] l = lean.computeIfAbsent(entity.getEntityId(), id -> new float[2]);
        l[0] += ((float) CameraMath.clamp(forward * 55, -28, 28) - l[0]) * 0.15f;
        l[1] += ((float) CameraMath.clamp(right * 55, -28, 28) - l[1]) * 0.15f;

        float spin = running ? time * (45f + (float) CameraMath.clamp(speed * 40, 0, 1) * 55f) * 8f : 0f;
        boolean lightsOn = running && (cam == null || cam.lights);
        float podPitch = cam != null ? (float) Config.cameraUptilt + cam.tilt : (float) Config.cameraUptilt;
        float podYaw = cam != null ? -cam.pan : 0f;
        int status = statusColor(entity);

        GlStateManager.pushMatrix();
        GlStateManager.translate((float) x, (float) y, (float) z);
        GlStateManager.rotate(180f - yaw, 0f, 1f, 0f);
        GlStateManager.translate(0f, 0.02f, 0f);
        GlStateManager.rotate(l[0], 1f, 0f, 0f);
        GlStateManager.rotate(-l[1], 0f, 0f, 1f);
        GlStateManager.enableRescaleNormal();
        bindTexture(TEXTURE);
        m.render(new BbModel.Animator() {
            @Override
            public float[] rotation(String group) {
                if (group.startsWith("prop_")) {
                    boolean cw = group.equals("prop_fl") || group.equals("prop_br");
                    return new float[]{0, cw ? spin : -spin, 0};
                }
                if (group.equals("camera_pod")) return new float[]{podPitch, podYaw, 0};
                return null;
            }

            @Override
            public int light(String element) {
                if (!element.startsWith("light_")) return -1;
                if (!lightsOn && !element.equals("light_status")) return 0;
                long t = (long) time;
                switch (element) {
                    case "light_red": return (t % 40) < 30 ? 0xFF2020 : 0x501010;
                    case "light_green": return (t % 40) < 30 ? 0x20FF40 : 0x105020;
                    case "light_white": return (t % 30 == 0 || t % 30 == 3) ? 0xFFFFFF : 0;
                    case "light_status": return running ? (status != 0 ? status : 0x30FF60) : 0;
                    default: return 0;
                }
            }
        }, entity.getBrightnessForRender());
        GlStateManager.disableRescaleNormal();
        GlStateManager.popMatrix();
        super.doRender(entity, x, y, z, entityYaw, partialTicks);
    }

    private static float wrap(float d) {
        d %= 360f;
        if (d >= 180f) d -= 360f;
        if (d < -180f) d += 360f;
        return d;
    }

    @Override
    protected ResourceLocation getEntityTexture(Entity entity) {
        return TEXTURE;
    }
}
