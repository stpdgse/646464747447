package org.lwjgl.sdl;

import org.teavm.mcweb.host.Sdl;

public class SDLMisc {
    public static boolean SDL_OpenURL(CharSequence url) {
        Sdl.openUrl(url.toString());
        return true;
    }
}
