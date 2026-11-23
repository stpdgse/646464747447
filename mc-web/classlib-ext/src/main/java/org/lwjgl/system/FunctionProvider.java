package org.lwjgl.system;

import java.nio.ByteBuffer;

public interface FunctionProvider {
    default long getFunctionAddress(CharSequence functionName) {
        return getFunctionAddress(MemoryUtil.memASCII(functionName, true));
    }

    long getFunctionAddress(ByteBuffer functionName);
}
