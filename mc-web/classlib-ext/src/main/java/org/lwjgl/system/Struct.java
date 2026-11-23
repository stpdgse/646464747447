package org.lwjgl.system;

/**
 * Base for native struct wrappers. Only the structs the game really touches are implemented, and
 * those are plain Java objects filled by the shim; this base exists to keep the hierarchy intact.
 */
public abstract class Struct<SELF extends Struct<SELF>> extends Pointer.Default {
    protected Struct(long address) {
        super(address);
    }

    public abstract int sizeof();

    public void clear() {
        MemoryUtil.memSet(address, 0, sizeof());
    }

    public void free() {
        MemoryUtil.nmemFree(address);
    }

    public boolean isNull(int offset) {
        return Heap.getLong(address + offset) == 0;
    }
}
