package org.lwjgl.sdl;

import java.nio.ByteBuffer;

public class SDLSurface {
    public static SDL_Surface SDL_CreateSurfaceFrom(int width, int height, int format, ByteBuffer pixels, int pitch) {
        return new SDL_Surface(width, height, format, pixels, pitch);
    }

    public static void SDL_DestroySurface(SDL_Surface surface) {
    }

    /** Window icons have no meaning in a browser tab. */
    public static boolean SDL_AddSurfaceAlternateImage(SDL_Surface surface, SDL_Surface image) {
        return true;
    }
}
