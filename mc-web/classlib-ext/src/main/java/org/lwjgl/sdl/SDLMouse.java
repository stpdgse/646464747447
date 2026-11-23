package org.lwjgl.sdl;

import java.nio.FloatBuffer;
import org.teavm.mcweb.host.Sdl;

public class SDLMouse {
    private static final String[] CURSORS = {
        "default", "text", "wait", "crosshair", "progress", "nwse-resize", "nesw-resize", "ew-resize",
        "ns-resize", "move", "not-allowed", "pointer", "nwse-resize", "ns-resize", "nesw-resize",
        "nwse-resize", "ew-resize", "nesw-resize", "ns-resize"
    };

    public static long SDL_CreateSystemCursor(int id) {
        return 1 + id;
    }

    public static long SDL_GetDefaultCursor() {
        return 1;
    }

    public static boolean SDL_SetCursor(long cursor) {
        int id = (int) (cursor - 1);
        Sdl.setCursor(id >= 0 && id < CURSORS.length ? CURSORS[id] : "default");
        return true;
    }

    public static boolean SDL_SetWindowRelativeMouseMode(long window, boolean enabled) {
        Sdl.setRelativeMouse(enabled);
        return true;
    }

    public static void SDL_WarpMouseInWindow(long window, float x, float y) {
    }

    public static int SDL_GetMouseState(FloatBuffer x, FloatBuffer y) {
        if (x != null) {
            x.put(x.position(), (float) Sdl.mouseX());
        }
        if (y != null) {
            y.put(y.position(), (float) Sdl.mouseY());
        }
        return Sdl.mouseButtons();
    }

    public static int SDL_GetGlobalMouseState(FloatBuffer x, FloatBuffer y) {
        return SDL_GetMouseState(x, y);
    }
}
