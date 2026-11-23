package org.lwjgl.openal;

public final class AL {
    private static final ALCapabilities CAPS = new ALCapabilities();

    private AL() {
    }

    public static ALCapabilities createCapabilities(ALCCapabilities alcCaps) {
        return CAPS;
    }

    public static ALCapabilities getCapabilities() {
        return CAPS;
    }
}
