package com.kamikazeskies.client;

import java.util.Set;

import com.kamikazeskies.drone.DroneEntity;
import com.kamikazeskies.sim.CameraMath;
import com.kamikazeskies.sim.Quat;

import net.minecraft.client.Minecraft;
import net.minecraft.resources.Identifier;
import net.neoforged.neoforge.client.event.RenderGuiLayerEvent;
import net.neoforged.neoforge.client.event.RenderHandEvent;
import net.neoforged.neoforge.client.event.ViewportEvent;

/** Camera orientation, field of view and HUD hiding while the goggles are on. */
public final class FpvView {
    /** Camera tilt up from the drone's nose, as on a real freestyle quad. */
    public static final double CAMERA_TILT_UP_DEG = 25.0;
    /** Wide FPV lens. */
    public static final float FOV_DEGREES = 105f;
    /** Flip if the horizon tilts the wrong way on screen. */
    private static final double ROLL_SIGN = 1.0;

    private static final Set<String> HIDDEN_VANILLA_LAYERS = Set.of(
            "crosshair", "hotbar", "player_health", "armor_level", "food_level", "vehicle_health", "air_level",
            "contextual_info_bar_background", "contextual_info_bar", "experience_level", "selected_item_name", "effects");

    private FpvView() {}

    private static DroneEntity activeDrone() {
        DroneEntity d = ClientPilot.drone();
        return d != null && Minecraft.getInstance().getCameraEntity() == d ? d : null;
    }

    public static void onCameraAngles(ViewportEvent.ComputeCameraAngles e) {
        DroneEntity d = activeDrone();
        if (d == null) return;
        Quat q = d.renderOrientation((float) e.getPartialTick());
        CameraMath.View v = CameraMath.view(q, CAMERA_TILT_UP_DEG);
        e.setYaw((float) v.yawDeg());
        e.setPitch((float) v.pitchDeg());
        e.setRoll((float) (ROLL_SIGN * v.rollDeg()));
    }

    public static void onFov(ViewportEvent.ComputeFov e) {
        if (activeDrone() != null) e.setFOV(FOV_DEGREES);
    }

    public static void onRenderHand(RenderHandEvent e) {
        if (activeDrone() != null) e.setCanceled(true);   // the operator's arm is not part of the FPV feed
    }

    public static void onGuiLayer(RenderGuiLayerEvent.Pre e) {
        if (activeDrone() == null) return;
        Identifier name = e.getName();
        if ("minecraft".equals(name.getNamespace()) && HIDDEN_VANILLA_LAYERS.contains(name.getPath())) e.setCanceled(true);
    }
}
