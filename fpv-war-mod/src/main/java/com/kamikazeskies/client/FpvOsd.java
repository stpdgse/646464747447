package com.kamikazeskies.client;

import com.kamikazeskies.drone.DroneEntity;
import com.kamikazeskies.input.PilotSwitches;
import com.kamikazeskies.sim.FlightInput;

import net.minecraft.client.DeltaTracker;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.Font;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.multiplayer.ClientLevel;
import net.minecraft.world.level.levelgen.Heightmap;
import net.neoforged.neoforge.client.gui.GuiLayer;

/** Betaflight-style on-screen display drawn over the FPV feed. */
public final class FpvOsd implements GuiLayer {
    private static final int WHITE = 0xFFFFFFFF;
    private static final int GREEN = 0xFF55FF55;
    private static final int RED = 0xFFFF5555;
    private static final int YELLOW = 0xFFFFDD55;

    @Override
    public void render(GuiGraphicsExtractor g, DeltaTracker deltaTracker) {
        DroneEntity d = ClientPilot.drone();
        Minecraft mc = Minecraft.getInstance();
        if (d == null || mc.level == null || mc.getCameraEntity() != d) return;

        Font font = mc.font;
        int w = g.guiWidth(), h = g.guiHeight();
        FlightInput in = ClientPilot.lastInput();
        PilotSwitches sw = ClientPilot.switches();

        // --- top-left: arm state, assist mode, device ---
        boolean armed = d.isArmed();
        g.text(font, armed ? "ARMED" : "DISARMED", 10, 10, armed ? GREEN : RED, true);
        g.text(font, "MODE " + sw.assist().name(), 10, 22, WHITE, true);
        g.text(font, ClientPilot.usingGamepad() ? "PAD" : "KEY", 10, 34, WHITE, true);

        // --- bottom-center: throttle, speed, altitude ---
        double speedKmh = d.speed() * 3.6;
        ClientLevel level = mc.level;
        int ground = level.getHeight(Heightmap.Types.MOTION_BLOCKING, d.getBlockX(), d.getBlockZ());
        String line = String.format("PWR %3d%%    SPD %3.0f km/h    ALT %4.1f m", Math.round(d.motorFraction() * 100), speedKmh, d.getY() - ground);
        g.text(font, line, (w - font.width(line)) / 2, h - 28, WHITE, true);

        // --- centre: small cross ---
        g.fill(w / 2 - 6, h / 2, w / 2 - 2, h / 2 + 1, WHITE);
        g.fill(w / 2 + 2, h / 2, w / 2 + 6, h / 2 + 1, WHITE);
        g.fill(w / 2, h / 2 - 6, w / 2 + 1, h / 2 - 2, WHITE);
        g.fill(w / 2, h / 2 + 2, w / 2 + 1, h / 2 + 6, WHITE);

        // --- warnings ---
        if (sw.armRefused()) {
            String msg = "ARMING REFUSED - LOWER THROTTLE";
            g.text(font, msg, (w - font.width(msg)) / 2, h / 2 + 40, YELLOW, true);
        } else if (!armed) {
            String msg = "R: ARM    F: MODE    G: EXIT";
            g.text(font, msg, (w - font.width(msg)) / 2, h / 2 + 40, WHITE, true);
        }
    }

}
