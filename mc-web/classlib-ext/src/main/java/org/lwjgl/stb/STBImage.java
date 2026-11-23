package org.lwjgl.stb;

import java.nio.ByteBuffer;
import java.nio.IntBuffer;
import org.lwjgl.system.Heap;
import org.teavm.mcweb.host.Mcshader;

/** stb_image, compiled to WebAssembly (the real library) inside mcshader.wasm. */
public final class STBImage {
    private static String failure;

    private STBImage() {
    }

    /** Null when the last decode succeeded, as in the native library. */
    public static String stbi_failure_reason() {
        return failure;
    }

    public static ByteBuffer stbi_load_from_memory(ByteBuffer buffer, IntBuffer x, IntBuffer y, IntBuffer channels,
            int desiredChannels) {
        int length = buffer.remaining();
        int in = Mcshader.malloc(Math.max(1, length));
        byte[] part = new byte[length];
        System.arraycopy(buffer.array(), buffer.arrayOffset() + buffer.position(), part, 0, length);
        Mcshader.write(in, part);
        int slots = Mcshader.malloc(12);
        int pixels = Mcshader.call("stbi_load_from_memory", in, length, slots, slots + 4, slots + 8, desiredChannels);
        Mcshader.free(in);
        try {
            if (pixels == 0) {
                failure = Mcshader.str(Mcshader.call("stbi_failure_reason"));
                return null;
            }
            failure = null;
            int w = Mcshader.readInt(slots);
            int h = Mcshader.readInt(slots + 4);
            int n = Mcshader.readInt(slots + 8);
            if (x != null) {
                x.put(x.position(), w);
            }
            if (y != null) {
                y.put(y.position(), h);
            }
            if (channels != null) {
                channels.put(channels.position(), n);
            }
            int outChannels = desiredChannels == 0 ? n : desiredChannels;
            int size = w * h * outChannels;
            long address = Heap.alloc(size);
            Mcshader.copyOut(pixels, address, size);
            Mcshader.call("stbi_image_free", pixels);
            return Heap.byteBuffer(address, size);
        } finally {
            Mcshader.free(slots);
        }
    }

    public static void nstbi_image_free(long address) {
        Heap.free(address);
    }
}
