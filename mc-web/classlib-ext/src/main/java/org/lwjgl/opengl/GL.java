package org.lwjgl.opengl;

import org.lwjgl.system.FunctionProvider;

public final class GL {
    private static final GLCapabilities CAPS = new GLCapabilities();
    private static final FunctionProvider PROVIDER = new org.lwjgl.system.Library.WebLibrary("webgl2");

    private GL() {
    }

    public static FunctionProvider getFunctionProvider() {
        return PROVIDER;
    }

    public static GLCapabilities createCapabilities() {
        return CAPS;
    }

    public static GLCapabilities getCapabilities() {
        return CAPS;
    }
}
