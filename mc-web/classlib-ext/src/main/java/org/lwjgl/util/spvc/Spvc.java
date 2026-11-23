package org.lwjgl.util.spvc;

import org.lwjgl.system.Library;
import org.lwjgl.system.SharedLibrary;

public class Spvc {
    public static SharedLibrary getLibrary() {
        return Library.loadNative("spvc", "spirv-cross-c-shared");
    }
}
