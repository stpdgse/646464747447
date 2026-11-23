package org.lwjgl.sdl;

import org.teavm.mcweb.host.Sdl;

public class SDLClipboard {
    public static String SDL_GetClipboardText() {
        String text = Sdl.getClipboard();
        return text == null ? "" : text;
    }

    public static boolean SDL_SetClipboardText(CharSequence text) {
        Sdl.setClipboard(text.toString());
        return true;
    }
}
