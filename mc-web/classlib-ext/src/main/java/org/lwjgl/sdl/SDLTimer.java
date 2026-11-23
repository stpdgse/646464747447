package org.lwjgl.sdl;

import org.teavm.mcweb.host.Sdl;

public class SDLTimer {
    public static long SDL_GetTicksNS() {
        return (long) Sdl.nanos();
    }
}
