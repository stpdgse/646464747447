package org.lwjgl.system;

import java.nio.ByteBuffer;

/** Base for buffers of pointer-sized or struct elements living in the emulated native heap. */
public abstract class CustomBuffer<SELF extends CustomBuffer<SELF>> extends Pointer.Default {
    protected ByteBuffer container;
    protected int mark;
    protected int position;
    protected int limit;
    protected int capacity;

    protected CustomBuffer(long address, ByteBuffer container, int mark, int position, int limit, int capacity) {
        super(address);
        this.container = container;
        this.mark = mark;
        this.position = position;
        this.limit = limit;
        this.capacity = capacity;
    }

    public abstract int sizeof();

    protected abstract SELF self();

    protected abstract SELF create(long address, ByteBuffer container, int mark, int position, int limit, int capacity);

    public long address0() {
        return address;
    }

    @Override
    public long address() {
        return address + (long) position * sizeof();
    }

    public long address(int offset) {
        return address + (long) offset * sizeof();
    }

    public void free() {
        MemoryUtil.memFree(this);
    }

    public int capacity() {
        return capacity;
    }

    public int position() {
        return position;
    }

    public SELF position(int newPosition) {
        if (newPosition < 0 || newPosition > limit) {
            throw new IllegalArgumentException("position " + newPosition + " out of range 0.." + limit);
        }
        position = newPosition;
        if (mark > position) {
            mark = -1;
        }
        return self();
    }

    public int limit() {
        return limit;
    }

    public SELF limit(int newLimit) {
        if (newLimit < 0 || newLimit > capacity) {
            throw new IllegalArgumentException("limit " + newLimit + " out of range 0.." + capacity);
        }
        limit = newLimit;
        if (position > limit) {
            position = limit;
        }
        if (mark > limit) {
            mark = -1;
        }
        return self();
    }

    public SELF mark() {
        mark = position;
        return self();
    }

    public SELF reset() {
        if (mark < 0) {
            throw new IllegalStateException("no mark");
        }
        position = mark;
        return self();
    }

    public SELF clear() {
        position = 0;
        limit = capacity;
        mark = -1;
        return self();
    }

    public SELF flip() {
        limit = position;
        position = 0;
        mark = -1;
        return self();
    }

    public SELF rewind() {
        position = 0;
        mark = -1;
        return self();
    }

    public int remaining() {
        return limit - position;
    }

    public boolean hasRemaining() {
        return position < limit;
    }

    public SELF slice() {
        return create(address + (long) position * sizeof(), container, -1, 0, remaining(), remaining());
    }

    public SELF slice(int offset, int length) {
        if (offset < 0 || length < 0 || offset + length > capacity) {
            throw new IllegalArgumentException("slice out of range");
        }
        return create(address + (long) offset * sizeof(), container, -1, 0, length, length);
    }

    public SELF duplicate() {
        return create(address, container, mark, position, limit, capacity);
    }

    public SELF put(SELF src) {
        if (src.remaining() > remaining()) {
            throw new java.nio.BufferOverflowException();
        }
        MemoryUtil.memCopy(src.address(), address(), (long) src.remaining() * sizeof());
        position += src.remaining();
        src.position += src.remaining();
        return self();
    }

    public SELF compact() {
        MemoryUtil.memCopy(address(), address0(), (long) remaining() * sizeof());
        position = remaining();
        limit = capacity;
        mark = -1;
        return self();
    }

    @Override
    public String toString() {
        return getClass().getSimpleName() + "[pos=" + position + " lim=" + limit + " cap=" + capacity + "]";
    }
}
