package org.lwjgl.sdl;

import org.lwjgl.system.Library;
import org.lwjgl.system.SharedLibrary;

public class SDL {
    private static SharedLibrary library;

    public static SharedLibrary getLibrary() {
        if (library == null) {
            library = Library.loadNative("sdl", "SDL3");
        }
        return library;
    }
}
