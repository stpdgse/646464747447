package org.lwjgl.stb;

import java.util.HashMap;
import java.util.Map;
import org.lwjgl.system.NativeResource;

/** A callback object; its "address" is an id that {@link STBImageWrite} resolves back to the object. */
public abstract class STBIWriteCallback implements STBIWriteCallbackI, NativeResource {
    private static final Map<Long, STBIWriteCallback> REGISTRY = new HashMap<>();
    private static long nextAddress = 0x5000_0000L;

    private final long address;

    protected STBIWriteCallback() {
        address = nextAddress;
        nextAddress += 0x10;
        REGISTRY.put(address, this);
    }

    public final long address() {
        return address;
    }

    public static java.nio.ByteBuffer getData(long data, int size) {
        return org.lwjgl.system.Heap.byteBuffer(data, size);
    }

    static STBIWriteCallback lookup(long address) {
        return REGISTRY.get(address);
    }

    @Override
    public void free() {
        REGISTRY.remove(address);
    }
}
