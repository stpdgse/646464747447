package org.lwjgl.openal;

import org.lwjgl.system.FunctionProviderLocal;

public final class ALC {
    private static final FunctionProviderLocal PROVIDER = new FunctionProviderLocal() {
        @Override
        public long getFunctionAddress(java.nio.ByteBuffer name) {
            return 0;
        }

        @Override
        public long getFunctionAddress(long handle, java.nio.ByteBuffer name) {
            return 0;
        }
    };

    private ALC() {
    }

    private static final ALCCapabilities CAPS = new ALCCapabilities();

    public static ALCCapabilities createCapabilities(long device) {
        return CAPS;
    }

    public static FunctionProviderLocal getFunctionProvider() {
        return PROVIDER;
    }
}
