package com.kamikazeskies.client;

import java.util.List;

import com.kamikazeskies.core.KamikazeSkies;
import com.kamikazeskies.sim.SignalLevel;

import net.minecraft.client.Minecraft;
import net.minecraft.resources.Identifier;

/**
 * Analog FPV video look. The post-effect JSONs (see {@code scripts/gen_fpv_post.py}) have fixed strengths per signal
 * level, and shaders get no clock, so the noise is animated by cycling through several pre-generated phases.
 */
public final class FpvPostEffects {
    public static final int PHASES = 8;

    private static SignalLevel shown = SignalLevel.CLEAN;
    private static boolean applied;

    private FpvPostEffects() {}

    public static Identifier id(SignalLevel level, int phase) {
        return Identifier.fromNamespaceAndPath(KamikazeSkies.MODID, "fpv_" + level.ordinal() + "_" + Math.floorMod(phase, PHASES));
    }

    /** The level currently shown, with hysteresis so the picture does not flicker between levels. */
    public static SignalLevel shownLevel() { return shown; }

    /**
     * Call every client tick. The game rebuilds its list of requested post effects every frame from
     * {@link net.minecraft.client.player.LocalPlayer#getActivePostEffects()}, so the effect is set there (a per-tick
     * addition to the renderer's own list would be wiped before the frame is drawn).
     */
    public static void update() {
        Minecraft mc = Minecraft.getInstance();
        var player = mc.player;
        if (player == null) { applied = false; return; }
        var drone = ClientPilot.drone();
        if (drone == null || mc.getCameraEntity() != drone || mc.level == null) {
            shown = SignalLevel.CLEAN;
            if (applied) { player.setActivePostEffects(List.of()); applied = false; }
            return;
        }
        shown = SignalLevel.withHysteresis(shown, drone.build().videoSignal(drone.rssi()), 0.03);
        player.setActivePostEffects(List.of(id(shown, (int) (mc.level.getGameTime() + drone.tickCount))));
        applied = true;
    }
}
