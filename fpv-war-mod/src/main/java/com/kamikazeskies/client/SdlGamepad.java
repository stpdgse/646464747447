package com.kamikazeskies.client;

import java.nio.IntBuffer;

import org.jspecify.annotations.Nullable;
import org.lwjgl.sdl.SDLGamepad;
import org.lwjgl.sdl.SDLInit;
import org.lwjgl.sdl.SDLStdinc;

import com.kamikazeskies.core.KamikazeSkies;
import com.kamikazeskies.input.GamepadSticks;

/**
 * Reads the first connected gamepad through SDL3 (already initialised by Minecraft's window layer; we only add
 * the gamepad subsystem). Every call is defensive: a missing native or driver disables the gamepad instead of
 * crashing the client.
 */
public final class SdlGamepad {
    private static final int RESCAN_TICKS = 40;   // look for a newly plugged-in pad every 2 s

    private static boolean failed;
    private static boolean initialised;
    private static long handle;
    private static int ticksSinceScan = RESCAN_TICKS;
    private static boolean prevSouth, prevNorth, prevEast;

    private SdlGamepad() {}

    /** Edge-triggered button presses since the last poll. */
    public record Buttons(boolean arm, boolean assist, boolean exit) {
        public static final Buttons NONE = new Buttons(false, false, false);
    }

    public record Sample(GamepadSticks.Axes axes, Buttons buttons) {}

    /** @return the current pad state, or null if no gamepad is available. Call once per client tick. */
    public static @Nullable Sample poll() {
        if (failed) return null;
        try {
            if (!initialised) {
                SDLInit.SDL_InitSubSystem(SDLInit.SDL_INIT_GAMEPAD);
                initialised = true;
            }
            SDLGamepad.SDL_UpdateGamepads();
            if (handle != 0 && !SDLGamepad.SDL_GamepadConnected(handle)) {
                SDLGamepad.SDL_CloseGamepad(handle);
                handle = 0;
                KamikazeSkies.LOGGER.info("Gamepad disconnected");
            }
            if (handle == 0 && ++ticksSinceScan >= RESCAN_TICKS) {
                ticksSinceScan = 0;
                open();
            }
            if (handle == 0) return null;

            GamepadSticks.Axes axes = new GamepadSticks.Axes(
                    axis(SDLGamepad.SDL_GAMEPAD_AXIS_LEFTX), axis(SDLGamepad.SDL_GAMEPAD_AXIS_LEFTY),
                    axis(SDLGamepad.SDL_GAMEPAD_AXIS_RIGHTX), axis(SDLGamepad.SDL_GAMEPAD_AXIS_RIGHTY));
            boolean south = SDLGamepad.SDL_GetGamepadButton(handle, SDLGamepad.SDL_GAMEPAD_BUTTON_SOUTH);
            boolean north = SDLGamepad.SDL_GetGamepadButton(handle, SDLGamepad.SDL_GAMEPAD_BUTTON_NORTH);
            boolean east = SDLGamepad.SDL_GetGamepadButton(handle, SDLGamepad.SDL_GAMEPAD_BUTTON_EAST);
            Buttons b = new Buttons(south && !prevSouth, north && !prevNorth, east && !prevEast);   // A = arm, Y = mode, B = exit
            prevSouth = south; prevNorth = north; prevEast = east;
            return new Sample(axes, b);
        } catch (Throwable t) {
            failed = true;
            KamikazeSkies.LOGGER.warn("Gamepad support disabled: {}", t.toString());
            return null;
        }
    }

    private static double axis(int which) { return Math.max(-1.0, SDLGamepad.SDL_GetGamepadAxis(handle, which) / 32767.0); }

    private static void open() {
        IntBuffer ids = SDLGamepad.SDL_GetGamepads();
        if (ids == null) return;
        try {
            if (ids.remaining() > 0) {
                handle = SDLGamepad.SDL_OpenGamepad(ids.get(0));
                if (handle != 0) KamikazeSkies.LOGGER.info("Gamepad connected: {}", SDLGamepad.SDL_GetGamepadName(handle));
            }
        } finally {
            SDLStdinc.SDL_free(ids);
        }
    }
}
