package org.lwjgl;

import java.nio.ByteBuffer;
import org.lwjgl.system.CustomBuffer;
import org.lwjgl.system.Heap;
import org.lwjgl.system.MemoryUtil;
import org.lwjgl.system.Pointer;

/** A buffer of 64-bit pointers in the emulated native heap. */
public class PointerBuffer extends CustomBuffer<PointerBuffer> {
    public PointerBuffer(ByteBuffer source) {
        this(Heap.addressOf(source), source, -1, 0, source.remaining() / Pointer.POINTER_SIZE,
                source.remaining() / Pointer.POINTER_SIZE);
    }

    PointerBuffer(long address, ByteBuffer container, int mark, int position, int limit, int capacity) {
        super(address, container, mark, position, limit, capacity);
    }

    public static PointerBuffer create(long address, int capacity) {
        return new PointerBuffer(address, null, -1, 0, capacity, capacity);
    }

    public static PointerBuffer allocateDirect(int capacity) {
        return create(Heap.alloc((long) capacity * Pointer.POINTER_SIZE), capacity);
    }

    @Override
    public int sizeof() {
        return Pointer.POINTER_SIZE;
    }

    @Override
    protected PointerBuffer self() {
        return this;
    }

    @Override
    protected PointerBuffer create(long address, ByteBuffer container, int mark, int position, int limit,
            int capacity) {
        return new PointerBuffer(address, container, mark, position, limit, capacity);
    }

    public long get() {
        return Heap.getLong(address());
    }

    public long get(int index) {
        return Heap.getLong(address(index));
    }

    public PointerBuffer put(long value) {
        Heap.putLong(address(), value);
        position++;
        return this;
    }

    public PointerBuffer put(int index, long value) {
        Heap.putLong(address(index), value);
        return this;
    }

    public PointerBuffer put(ByteBuffer value) {
        return put(MemoryUtil.memAddress(value));
    }

    public PointerBuffer put(int index, ByteBuffer value) {
        return put(index, MemoryUtil.memAddress(value));
    }

    public PointerBuffer put(Pointer value) {
        return put(value.address());
    }

    public ByteBuffer getByteBuffer(int length) {
        return MemoryUtil.memByteBuffer(get(), length);
    }

    public String getStringUTF8() {
        return MemoryUtil.memUTF8(get());
    }
}
