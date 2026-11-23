package org.lwjgl.system;

import java.nio.ByteBuffer;

public interface FunctionProviderLocal extends FunctionProvider {
    default long getFunctionAddress(long handle, CharSequence functionName) {
        return 0;
    }

    long getFunctionAddress(long handle, ByteBuffer functionName);
}
