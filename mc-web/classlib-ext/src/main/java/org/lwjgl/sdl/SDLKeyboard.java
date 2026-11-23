package org.lwjgl.sdl;

import java.nio.ByteBuffer;
import org.teavm.mcweb.host.Sdl;

public class SDLKeyboard {
    private static final ByteBuffer STATE = ByteBuffer.allocate(512);

    public static boolean SDL_StartTextInput(long window) {
        Sdl.setTextInput(true);
        return true;
    }

    public static boolean SDL_StopTextInput(long window) {
        Sdl.setTextInput(false);
        return true;
    }

    public static boolean SDL_ClearComposition(long window) {
        return true;
    }

    public static boolean SDL_SetTextInputArea(long window, SDL_Rect.Buffer rect, int cursor) {
        return true;
    }

    public static short SDL_GetModState() {
        return (short) Sdl.mods();
    }

    /** Snapshot of the held keys, indexed by SDL scancode. */
    public static ByteBuffer SDL_GetKeyboardState() {
        for (int i = 0; i < 512; i++) {
            STATE.put(i, (byte) Sdl.keyDown(i));
        }
        return STATE;
    }

    public static int SDL_GetKeyFromScancode(int scancode, short modState, boolean keyEvent) {
        return SdlKeys.keycode(scancode, (modState & 0x3) != 0);
    }

    public static String SDL_GetKeyName(int keycode) {
        return SdlKeys.name(keycode);
    }
}
