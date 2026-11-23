package org.lwjgl.sdl;

import java.nio.IntBuffer;
import org.lwjgl.PointerBuffer;

public class SDLStdinc {
    /** Buffers returned by the shim are ordinary Java objects, so there is nothing to release. */
    public static void SDL_free(IntBuffer buffer) {
    }

    public static void SDL_free(PointerBuffer buffer) {
    }
}
