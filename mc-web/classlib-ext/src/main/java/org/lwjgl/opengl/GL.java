package org.lwjgl.opengl;

import org.lwjgl.system.FunctionProvider;

public final class GL {
    private static final GLCapabilities CAPS = new GLCapabilities();
    private static final FunctionProvider PROVIDER = name -> 0L;

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
