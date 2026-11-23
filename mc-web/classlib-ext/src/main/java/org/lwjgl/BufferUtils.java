package org.lwjgl;

import java.nio.ByteBuffer;
import org.lwjgl.system.MemoryUtil;

public final class BufferUtils {
    private BufferUtils() {
    }

    public static ByteBuffer createByteBuffer(int capacity) {
        return MemoryUtil.memCalloc(capacity);
    }

    public static java.nio.IntBuffer createIntBuffer(int capacity) {
        return MemoryUtil.memCalloc(capacity * 4).asIntBuffer();
    }

    public static java.nio.FloatBuffer createFloatBuffer(int capacity) {
        return MemoryUtil.memCalloc(capacity * 4).asFloatBuffer();
    }
}
