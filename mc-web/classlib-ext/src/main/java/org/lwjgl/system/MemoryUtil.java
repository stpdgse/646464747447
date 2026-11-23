package org.lwjgl.system;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.IntBuffer;
import java.nio.charset.StandardCharsets;
import org.lwjgl.PointerBuffer;

/** Memory access on the emulated native heap (see {@link Heap}). */
public final class MemoryUtil {
    public static final long NULL = 0L;

    private MemoryUtil() {
    }

    public interface MemoryAllocator {
        long malloc(long size);

        long calloc(long num, long size);

        long realloc(long ptr, long size);

        void free(long ptr);

        long aligned_alloc(long alignment, long size);

        void aligned_free(long ptr);
    }

    private static final MemoryAllocator ALLOCATOR = new MemoryAllocator() {
        @Override
        public long malloc(long size) {
            return Heap.alloc(size);
        }

        @Override
        public long calloc(long num, long size) {
            return Heap.alloc(num * size);
        }

        @Override
        public long realloc(long ptr, long size) {
            return Heap.reallocate(ptr, size);
        }

        @Override
        public void free(long ptr) {
            Heap.free(ptr);
        }

        @Override
        public long aligned_alloc(long alignment, long size) {
            return Heap.alloc(size);
        }

        @Override
        public void aligned_free(long ptr) {
            Heap.free(ptr);
        }
    };

    public static MemoryAllocator getAllocator(boolean aligned) {
        return ALLOCATOR;
    }

    // raw allocation ---------------------------------------------------------------------------

    public static long nmemAlloc(long size) {
        return Heap.alloc(size);
    }

    public static long nmemCalloc(long num, long size) {
        return Heap.alloc(num * size);
    }

    public static long nmemRealloc(long ptr, long size) {
        return Heap.reallocate(ptr, size);
    }

    public static void nmemFree(long ptr) {
        Heap.free(ptr);
    }

    // buffer allocation ------------------------------------------------------------------------

    public static ByteBuffer memAlloc(int size) {
        return Heap.byteBuffer(Heap.alloc(size), size);
    }

    public static ByteBuffer memCalloc(int size) {
        return memAlloc(size);
    }

    public static IntBuffer memAllocInt(int size) {
        return Heap.intBuffer(Heap.alloc((long) size * 4), size);
    }

    public static PointerBuffer memAllocPointer(int size) {
        return PointerBuffer.create(Heap.alloc((long) size * 8), size);
    }

    public static ByteBuffer memRealloc(ByteBuffer old, int size) {
        if (old == null) {
            return memAlloc(size);
        }
        long fresh = Heap.reallocate(memAddress0(old), size);
        return Heap.byteBuffer(fresh, size);
    }

    public static PointerBuffer memRealloc(PointerBuffer old, int size) {
        long fresh = old == null ? Heap.alloc((long) size * 8) : Heap.reallocate(old.address0(), (long) size * 8);
        return PointerBuffer.create(fresh, size);
    }

    public static void memFree(ByteBuffer buffer) {
        if (buffer != null) {
            Heap.free(memAddress0(buffer));
        }
    }

    public static void memFree(IntBuffer buffer) {
        if (buffer != null) {
            Heap.free(addressZero(buffer));
            Heap.forget(buffer);
        }
    }

    public static void memFree(java.nio.FloatBuffer buffer) {
        if (buffer != null) {
            Heap.free(Heap.addressOfView(buffer, 2) - ((long) buffer.position() << 2));
            Heap.forget(buffer);
        }
    }

    public static void memFree(CustomBuffer<?> buffer) {
        if (buffer != null) {
            Heap.free(buffer.address0());
        }
    }

    private static long addressZero(IntBuffer b) {
        return Heap.addressOfView(b, 2) - ((long) b.position() << 2);
    }

    // addresses --------------------------------------------------------------------------------

    /** Address of the buffer's start (ignoring position), used to free it. */
    private static long memAddress0(ByteBuffer b) {
        return Heap.addressOf(b) - b.position();
    }

    public static long memAddress(ByteBuffer buffer) {
        return Heap.addressOf(buffer);
    }

    public static long memAddress(ByteBuffer buffer, int position) {
        return Heap.addressOf(buffer) - buffer.position() + position;
    }

    public static long memAddress(IntBuffer buffer) {
        return Heap.addressOfView(buffer, 2);
    }

    public static long memAddress(java.nio.FloatBuffer buffer) {
        return Heap.addressOfView(buffer, 2);
    }

    public static long memAddress(java.nio.LongBuffer buffer) {
        return Heap.addressOfView(buffer, 3);
    }

    public static long memAddress(java.nio.ShortBuffer buffer) {
        return Heap.addressOfView(buffer, 1);
    }

    public static long memAddress(CustomBuffer<?> buffer) {
        return buffer.address();
    }

    public static long memAddressSafe(ByteBuffer buffer) {
        return buffer == null ? NULL : memAddress(buffer);
    }

    // views ------------------------------------------------------------------------------------

    public static ByteBuffer memByteBuffer(long address, int capacity) {
        return Heap.byteBuffer(address, capacity);
    }

    public static ByteBuffer memByteBuffer(IntBuffer buffer) {
        return Heap.byteBuffer(memAddress(buffer), buffer.remaining() * 4);
    }

    public static IntBuffer memIntBuffer(long address, int capacity) {
        return Heap.intBuffer(address, capacity);
    }

    public static java.nio.FloatBuffer memFloatBuffer(long address, int capacity) {
        return Heap.floatBuffer(address, capacity);
    }

    public static java.nio.LongBuffer memLongBuffer(long address, int capacity) {
        return Heap.longBuffer(address, capacity);
    }

    public static PointerBuffer memPointerBuffer(long address, int capacity) {
        return PointerBuffer.create(address, capacity);
    }

    public static ByteBuffer memSlice(ByteBuffer buffer, int offset, int capacity) {
        ByteBuffer d = buffer.duplicate();
        d.position(buffer.position() + offset);
        d.limit(buffer.position() + offset + capacity);
        ByteBuffer s = d.slice();
        s.order(ByteOrder.LITTLE_ENDIAN);
        return s;
    }

    // scalar access ----------------------------------------------------------------------------

    public static byte memGetByte(long a) {
        return Heap.getByte(a);
    }

    public static short memGetShort(long a) {
        return Heap.getShort(a);
    }

    public static int memGetInt(long a) {
        return Heap.getInt(a);
    }

    public static long memGetLong(long a) {
        return Heap.getLong(a);
    }

    public static float memGetFloat(long a) {
        return Heap.getFloat(a);
    }

    public static double memGetDouble(long a) {
        return Heap.getDouble(a);
    }

    public static long memGetAddress(long a) {
        return Heap.getLong(a);
    }

    public static void memPutByte(long a, byte v) {
        Heap.putByte(a, v);
    }

    public static void memPutShort(long a, short v) {
        Heap.putShort(a, v);
    }

    public static void memPutInt(long a, int v) {
        Heap.putInt(a, v);
    }

    public static void memPutLong(long a, long v) {
        Heap.putLong(a, v);
    }

    public static void memPutFloat(long a, float v) {
        Heap.putFloat(a, v);
    }

    public static void memPutDouble(long a, double v) {
        Heap.putDouble(a, v);
    }

    public static void memPutAddress(long a, long v) {
        Heap.putLong(a, v);
    }

    // bulk -------------------------------------------------------------------------------------

    public static void memCopy(long src, long dst, long bytes) {
        Heap.copy(src, dst, bytes);
    }

    public static void memCopy(ByteBuffer src, ByteBuffer dst) {
        dst.put(src);
    }

    public static void memSet(long a, int value, long bytes) {
        Heap.set(a, value, bytes);
    }

    public static void memSet(CustomBuffer<?> buffer, int value) {
        Heap.set(buffer.address(), value, (long) buffer.remaining() * buffer.sizeof());
    }

    // strings ----------------------------------------------------------------------------------

    public static ByteBuffer memASCII(CharSequence text) {
        return memASCII(text, true);
    }

    public static ByteBuffer memASCII(CharSequence text, boolean nullTerminated) {
        return encode(text, nullTerminated, StandardCharsets.ISO_8859_1);
    }

    public static ByteBuffer memUTF8(CharSequence text) {
        return memUTF8(text, true);
    }

    public static ByteBuffer memUTF8(CharSequence text, boolean nullTerminated) {
        return encode(text, nullTerminated, StandardCharsets.UTF_8);
    }

    private static ByteBuffer encode(CharSequence text, boolean nullTerminated, java.nio.charset.Charset cs) {
        byte[] bytes = text.toString().getBytes(cs);
        ByteBuffer b = memAlloc(bytes.length + (nullTerminated ? 1 : 0));
        b.put(bytes);
        if (nullTerminated) {
            b.put((byte) 0);
        }
        b.flip();
        return b;
    }

    public static String memASCII(long address) {
        return decode(address, -1, StandardCharsets.ISO_8859_1);
    }

    public static String memASCII(long address, int length) {
        return decode(address, length, StandardCharsets.ISO_8859_1);
    }

    public static String memUTF8(long address) {
        return decode(address, -1, StandardCharsets.UTF_8);
    }

    public static String memUTF8(long address, int length) {
        return decode(address, length, StandardCharsets.UTF_8);
    }

    public static String memUTF8(ByteBuffer buffer) {
        byte[] b = new byte[buffer.remaining()];
        buffer.duplicate().get(b);
        int n = 0;
        while (n < b.length && b[n] != 0) {
            n++;
        }
        return new String(b, 0, n, StandardCharsets.UTF_8);
    }

    private static String decode(long address, int length, java.nio.charset.Charset cs) {
        if (address == NULL) {
            return null;
        }
        int n = length;
        if (n < 0) {
            n = 0;
            while (Heap.getByte(address + n) != 0) {
                n++;
            }
        }
        byte[] b = new byte[n];
        for (int i = 0; i < n; i++) {
            b[i] = Heap.getByte(address + i);
        }
        return new String(b, cs);
    }
}
