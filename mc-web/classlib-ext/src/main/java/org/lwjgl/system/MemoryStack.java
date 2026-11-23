package org.lwjgl.system;

import java.nio.ByteBuffer;
import java.nio.IntBuffer;
import java.util.ArrayList;
import org.lwjgl.PointerBuffer;

/** Scoped allocation: everything allocated since {@code push()} is released by {@code close()}. */
public class MemoryStack extends Pointer.Default implements AutoCloseable {
    private static final ThreadLocal<MemoryStack> TLS = ThreadLocal.withInitial(MemoryStack::new);

    private final ArrayList<ArrayList<Object>> frames = new ArrayList<>();

    private MemoryStack() {
        super(1);
        frames.add(new ArrayList<>());
    }

    public static MemoryStack stackGet() {
        return TLS.get();
    }

    public static MemoryStack stackPush() {
        return stackGet().push();
    }

    public static MemoryStack stackPop() {
        return stackGet().pop();
    }

    public MemoryStack push() {
        frames.add(new ArrayList<>());
        return this;
    }

    public MemoryStack pop() {
        ArrayList<Object> frame = frames.remove(frames.size() - 1);
        for (Object o : frame) {
            if (o instanceof Long) {
                Heap.free((Long) o);
            } else if (o instanceof java.nio.Buffer) {
                // views are tracked by their address; forgetting them lets the page be collected
                Heap.forget((java.nio.Buffer) o);
            }
        }
        return this;
    }

    @Override
    public void close() {
        pop();
    }

    public int getPointer() {
        return frames.size();
    }

    public void setPointer(int pointer) {
        while (frames.size() > pointer) {
            pop();
        }
    }

    public int getSize() {
        return Integer.MAX_VALUE;
    }

    private long allocate(long bytes) {
        long address = Heap.alloc(bytes);
        frames.get(frames.size() - 1).add(address);
        return address;
    }

    private <B extends java.nio.Buffer> B track(B buffer) {
        frames.get(frames.size() - 1).add(buffer);
        return buffer;
    }

    // raw -------------------------------------------------------------------------------------

    public long nmalloc(int alignment, int size) {
        return allocate(size);
    }

    public long nmalloc(int size) {
        return allocate(size);
    }

    public long ncalloc(int alignment, int num, int size) {
        return allocate((long) num * size);
    }

    public ByteBuffer malloc(int size) {
        return Heap.byteBuffer(allocate(size), size);
    }

    public ByteBuffer malloc(int alignment, int size) {
        return malloc(size);
    }

    public ByteBuffer calloc(int size) {
        return malloc(size);
    }

    public ByteBuffer calloc(int alignment, int size) {
        return malloc(size);
    }

    public ByteBuffer bytes(byte... values) {
        ByteBuffer b = malloc(values.length);
        b.put(values).flip();
        return b;
    }

    public ByteBuffer bytes(byte x) {
        ByteBuffer b = malloc(1);
        b.put(0, x);
        return b;
    }

    // typed ------------------------------------------------------------------------------------

    public IntBuffer mallocInt(int size) {
        return track(Heap.intBuffer(allocate((long) size * 4), size));
    }

    public IntBuffer callocInt(int size) {
        return mallocInt(size);
    }

    public java.nio.FloatBuffer mallocFloat(int size) {
        return track(Heap.floatBuffer(allocate((long) size * 4), size));
    }

    public java.nio.FloatBuffer callocFloat(int size) {
        return mallocFloat(size);
    }

    public java.nio.LongBuffer mallocLong(int size) {
        return track(Heap.longBuffer(allocate((long) size * 8), size));
    }

    public java.nio.LongBuffer callocLong(int size) {
        return mallocLong(size);
    }

    public java.nio.ShortBuffer mallocShort(int size) {
        return track(Heap.shortBuffer(allocate((long) size * 2), size));
    }

    public java.nio.DoubleBuffer mallocDouble(int size) {
        return track(Heap.doubleBuffer(allocate((long) size * 8), size));
    }

    public PointerBuffer mallocPointer(int size) {
        return PointerBuffer.create(allocate((long) size * 8), size);
    }

    public PointerBuffer callocPointer(int size) {
        return mallocPointer(size);
    }

    public IntBuffer ints(int x) {
        IntBuffer b = mallocInt(1);
        b.put(0, x);
        return b;
    }

    public IntBuffer ints(int x, int y) {
        IntBuffer b = mallocInt(2);
        b.put(0, x).put(1, y);
        return b;
    }

    public IntBuffer ints(int... values) {
        IntBuffer b = mallocInt(values.length);
        b.put(values).flip();
        return b;
    }

    public java.nio.LongBuffer longs(long x) {
        java.nio.LongBuffer b = mallocLong(1);
        b.put(0, x);
        return b;
    }

    public java.nio.FloatBuffer floats(float x) {
        java.nio.FloatBuffer b = mallocFloat(1);
        b.put(0, x);
        return b;
    }

    public PointerBuffer pointers(long x) {
        PointerBuffer b = mallocPointer(1);
        b.put(0, x);
        return b;
    }

    // strings ----------------------------------------------------------------------------------

    public ByteBuffer ASCII(CharSequence text) {
        return ASCII(text, true);
    }

    public ByteBuffer ASCII(CharSequence text, boolean nullTerminated) {
        byte[] b = text.toString().getBytes(java.nio.charset.StandardCharsets.ISO_8859_1);
        return stringBytes(b, nullTerminated);
    }

    public ByteBuffer UTF8(CharSequence text) {
        return UTF8(text, true);
    }

    public ByteBuffer UTF8(CharSequence text, boolean nullTerminated) {
        byte[] b = text.toString().getBytes(java.nio.charset.StandardCharsets.UTF_8);
        return stringBytes(b, nullTerminated);
    }

    private ByteBuffer stringBytes(byte[] b, boolean nullTerminated) {
        ByteBuffer out = malloc(b.length + (nullTerminated ? 1 : 0));
        out.put(b);
        if (nullTerminated) {
            out.put((byte) 0);
        }
        out.flip();
        return out;
    }
}
