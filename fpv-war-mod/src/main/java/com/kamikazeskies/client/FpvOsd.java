package com.kamikazeskies.client;

import com.kamikazeskies.drone.DroneEntity;
import com.kamikazeskies.input.PilotSwitches;
import com.kamikazeskies.sim.FlightInput;
import com.kamikazeskies.sim.SignalLevel;

import net.minecraft.client.DeltaTracker;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.Font;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.multiplayer.ClientLevel;
import net.minecraft.world.level.levelgen.Heightmap;
import net.neoforged.neoforge.client.gui.GuiLayer;

/** Betaflight-style on-screen display drawn over the FPV feed. */
public final class FpvOsd implements GuiLayer {
    private static final int WHITE = 0xFFFFFFFF, GREEN = 0xFF55FF55, RED = 0xFFFF5555, YELLOW = 0xFFFFDD55;

    @Override
    public void render(GuiGraphicsExtractor g, DeltaTracker deltaTracker) {
        DroneEntity d = ClientPilot.drone();
        Minecraft mc = Minecraft.getInstance();
        if (d == null || mc.level == null || mc.getCameraEntity() != d) return;

        Font font = mc.font;
        int w = g.guiWidth(), h = g.guiHeight();
        PilotSwitches sw = ClientPilot.switches();
        long blink = (mc.level.getGameTime() / 6) % 2;       // ~1.7 Hz
        SignalLevel level = FpvPostEffects.shownLevel();

        // --- top-left: arm state, assist mode, device ---
        boolean armed = d.isArmed();
        g.text(font, armed ? "ARMED" : "DISARMED", 10, 10, armed ? GREEN : RED, true);
        g.text(font, "MODE " + sw.assist().name(), 10, 22, WHITE, true);
        g.text(font, ClientPilot.usingGamepad() ? "PAD" : "KEY", 10, 34, WHITE, true);

        // --- top-right: battery and signal ---
        boolean critical = d.batterySoc() < 0.05f, low = d.batteryLow();
        int batColor = critical ? (blink == 0 ? RED : 0x00000000) : low ? YELLOW : GREEN;
        String bat = String.format("%4.1fV  %3d%%", d.voltage(), Math.round(d.batterySoc() * 100));
        g.text(font, bat, w - 10 - font.width(bat), 10, batColor, true);
        int sigColor = level.ordinal() <= 1 ? GREEN : level.ordinal() <= 3 ? YELLOW : RED;
        String sig = String.format("RSSI %3d%%", Math.round(d.rssi() * 100));
        g.text(font, sig, w - 10 - font.width(sig), 22, sigColor, true);
        int bars = Math.max(0, 5 - level.ordinal());                  // 5 bars at a clean signal, 0 when lost
        for (int i = 0; i < 5; i++) {
            int bx = w - 10 - 5 * 8 + i * 8, bh = 3 + i * 2;
            g.fill(bx, 44 - bh, bx + 5, 44, i < bars ? sigColor : 0x66FFFFFF);
        }
        String timer = String.format("%02d:%02d", ClientPilot.flightSeconds() / 60, ClientPilot.flightSeconds() % 60);
        g.text(font, timer, (w - font.width(timer)) / 2, 10, WHITE, true);

        // --- bottom-centre: power, speed, altitude ---
        ClientLevel level2 = mc.level;
        int ground = level2.getHeight(Heightmap.Types.MOTION_BLOCKING, d.getBlockX(), d.getBlockZ());
        String line = String.format("PWR %3d%%    SPD %3.0f km/h    ALT %4.1f m", Math.round(d.motorFraction() * 100), d.speed() * 3.6, d.getY() - ground);
        g.text(font, line, (w - font.width(line)) / 2, h - 28, WHITE, true);

        // --- centre cross ---
        g.fill(w / 2 - 6, h / 2, w / 2 - 2, h / 2 + 1, WHITE);
        g.fill(w / 2 + 2, h / 2, w / 2 + 6, h / 2 + 1, WHITE);
        g.fill(w / 2, h / 2 - 6, w / 2 + 1, h / 2 - 2, WHITE);
        g.fill(w / 2, h / 2 + 2, w / 2 + 1, h / 2 + 6, WHITE);

        // --- warnings ---
        if (level == SignalLevel.LOST) {
            centred(g, font, "LINK LOST", w, h / 2 - 30, blink == 0 ? RED : WHITE);
        } else if (critical) {
            centred(g, font, "BATTERY CRITICAL", w, h / 2 - 30, blink == 0 ? RED : 0x00000000);
        } else if (low) {
            centred(g, font, "LOW BATTERY", w, h / 2 - 30, blink == 0 ? YELLOW : 0x00000000);
        } else if (level.ordinal() >= SignalLevel.BREAKUP.ordinal()) {
            centred(g, font, "WEAK SIGNAL", w, h / 2 - 30, YELLOW);
        }
        if (sw.armRefused()) {
            centred(g, font, "ARMING REFUSED - LOWER THROTTLE", w, h / 2 + 40, YELLOW);
        } else if (!armed) {
            centred(g, font, "R: ARM    F: MODE    G: EXIT", w, h / 2 + 40, WHITE);
        }
    }

    private static void centred(GuiGraphicsExtractor g, Font font, String s, int w, int y, int color) {
        if ((color >>> 24) == 0) return;       // blink-off phase
        g.text(font, s, (w - font.width(s)) / 2, y, color, true);
    }
}
