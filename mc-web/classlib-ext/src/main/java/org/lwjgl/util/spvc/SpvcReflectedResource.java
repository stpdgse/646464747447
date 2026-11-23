package org.lwjgl.util.spvc;

import org.lwjgl.sdl.SdlStructBuffer;
import org.lwjgl.system.Heap;
import org.lwjgl.system.NativeResource;
import org.lwjgl.system.Struct;

/**
 * spvc_reflected_resource: {id, base_type_id, type_id, name}. {@link Spvc} copies the list out of the
 * SPIRV-Cross module into the emulated native heap in this 24-byte layout.
 */
public class SpvcReflectedResource extends Struct<SpvcReflectedResource> implements NativeResource {
    public static final int SIZEOF = 24;

    private SpvcReflectedResource(long address) {
        super(address);
    }

    public static Buffer create(long address, int count) {
        return new Buffer(address, count);
    }

    @Override
    public int sizeof() {
        return SIZEOF;
    }

    public int id() {
        return Heap.getInt(address);
    }

    public int base_type_id() {
        return Heap.getInt(address + 4);
    }

    public int type_id() {
        return Heap.getInt(address + 8);
    }

    public String nameString() {
        long name = Heap.getLong(address + 16);
        if (name == 0) {
            return null;
        }
        int len = 0;
        while (Heap.getByte(name + len) != 0) {
            len++;
        }
        byte[] raw = new byte[len];
        for (int i = 0; i < len; i++) {
            raw[i] = Heap.getByte(name + i);
        }
        return new String(raw, java.nio.charset.StandardCharsets.UTF_8);
    }

    @Override
    public void free() {
    }

    public static class Buffer extends SdlStructBuffer<SpvcReflectedResource, Buffer> {
        private final long address;
        private final int count;

        Buffer(long address, int count) {
            this.address = address;
            this.count = count;
            fill(count);
        }

        private int index;

        @Override
        protected SpvcReflectedResource newElement() {
            return new SpvcReflectedResource(address + (long) SIZEOF * index++);
        }

        @Override
        protected Buffer self() {
            return this;
        }
    }
}
