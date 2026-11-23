package org.lwjgl.util.shaderc;

import org.lwjgl.system.Library;
import org.lwjgl.system.SharedLibrary;

public class Shaderc {
    public static SharedLibrary getLibrary() {
        return Library.loadNative("shaderc", "shaderc_shared");
    }
}
