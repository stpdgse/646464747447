package org.lwjgl.sdl;

public class SDLInit {
    public static boolean SDL_Init(int flags) {
        return true;
    }

    public static void SDL_Quit() {
    }

    public static boolean SDL_SetAppMetadataProperty(CharSequence name, CharSequence value) {
        return true;
    }
}
