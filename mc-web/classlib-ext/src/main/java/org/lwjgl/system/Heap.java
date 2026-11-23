package org.lwjgl.system;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.IntBuffer;
import java.util.ArrayList;
import java.util.IdentityHashMap;

/**
 * Emulated native memory. WebAssembly GC objects have no addresses, so an "address" here is
 * {@code (pageId << 32) | byteOffset} into a registered {@code byte[]} page. Every buffer this
 * package hands out is backed by such a page, which makes {@code memGetInt(address)} and friends
 * work uniformly, and any foreign heap buffer can be registered on demand.
 */
public final class Heap {
    private static final ArrayList<byte[]> PAGES = new ArrayList<>();
    private static final ArrayList<Integer> FREE = new ArrayList<>();
    private static final IdentityHashMap<byte[], Integer> IDS = new IdentityHashMap<>();
    /** Addresses of primitive-view buffers (IntBuffer, FloatBuffer, ...) created by this package. */
    private static final IdentityHashMap<java.nio.Buffer, Long> VIEW_ADDRESS = new IdentityHashMap<>();

    static {
        PAGES.add(null); // id 0 is the null pointer
    }

    private Heap() {
    }

    // allocation -------------------------------------------------------------------------------

    public static long alloc(long size) {
        if (size < 0 || size > Integer.MAX_VALUE) {
            throw new OutOfMemoryError("native allocation of " + size + " bytes");
        }
        return register(new byte[(int) Math.max(size, 1)]);
    }

    /** Registers a page and returns the address of its first byte. */
    public static long register(byte[] page) {
        Integer existing = IDS.get(page);
        if (existing != null) {
            return (long) existing << 32;
        }
        int id;
        if (!FREE.isEmpty()) {
            id = FREE.remove(FREE.size() - 1);
            PAGES.set(id, page);
        } else {
            id = PAGES.size();
            PAGES.add(page);
        }
        IDS.put(page, id);
        return (long) id << 32;
    }

    public static void free(long address) {
        if (address == 0) {
            return;
        }
        int id = (int) (address >>> 32);
        byte[] page = id < PAGES.size() ? PAGES.get(id) : null;
        if (page == null) {
            return;
        }
        IDS.remove(page);
        PAGES.set(id, null);
        FREE.add(id);
    }

    public static long reallocate(long address, long newSize) {
        if (address == 0) {
            return alloc(newSize);
        }
        byte[] old = page(address);
        long fresh = alloc(newSize);
        System.arraycopy(old, offset(address), page(fresh), 0, (int) Math.min(newSize, old.length - offset(address)));
        free(address);
        return fresh;
    }

    // addressing -------------------------------------------------------------------------------

    static byte[] page(long address) {
        int id = (int) (address >>> 32);
        byte[] page = id > 0 && id < PAGES.size() ? PAGES.get(id) : null;
        if (page == null) {
            throw new IllegalStateException("invalid native address 0x" + Long.toHexString(address));
        }
        return page;
    }

    static int offset(long address) {
        return (int) address;
    }

    public static long addressOf(ByteBuffer buffer) {
        if (!buffer.hasArray()) {
            throw new UnsupportedOperationException("buffer without backing array has no address");
        }
        return register(buffer.array()) + buffer.arrayOffset() + buffer.position();
    }

    public static long addressOfView(java.nio.Buffer buffer, int elementShift) {
        Long base = VIEW_ADDRESS.get(buffer);
        if (base == null) {
            throw new UnsupportedOperationException("buffer was not created by the native heap");
        }
        return base + ((long) buffer.position() << elementShift);
    }

    // buffers ----------------------------------------------------------------------------------

    public static ByteBuffer byteBuffer(long address, int size) {
        if (address == 0) {
            throw new NullPointerException("null native address");
        }
        byte[] page = page(address);
        ByteBuffer b = ByteBuffer.wrap(page, offset(address), size).slice();
        b.order(ByteOrder.LITTLE_ENDIAN);
        return b;
    }

    public static IntBuffer intBuffer(long address, int count) {
        IntBuffer b = byteBuffer(address, count * 4).asIntBuffer();
        VIEW_ADDRESS.put(b, address);
        return b;
    }

    public static java.nio.FloatBuffer floatBuffer(long address, int count) {
        java.nio.FloatBuffer b = byteBuffer(address, count * 4).asFloatBuffer();
        VIEW_ADDRESS.put(b, address);
        return b;
    }

    public static java.nio.LongBuffer longBuffer(long address, int count) {
        java.nio.LongBuffer b = byteBuffer(address, count * 8).asLongBuffer();
        VIEW_ADDRESS.put(b, address);
        return b;
    }

    public static java.nio.ShortBuffer shortBuffer(long address, int count) {
        java.nio.ShortBuffer b = byteBuffer(address, count * 2).asShortBuffer();
        VIEW_ADDRESS.put(b, address);
        return b;
    }

    public static java.nio.DoubleBuffer doubleBuffer(long address, int count) {
        java.nio.DoubleBuffer b = byteBuffer(address, count * 8).asDoubleBuffer();
        VIEW_ADDRESS.put(b, address);
        return b;
    }

    public static void forget(java.nio.Buffer view) {
        VIEW_ADDRESS.remove(view);
    }

    // scalar access (little endian) --------------------------------------------------------------

    public static byte getByte(long a) {
        return page(a)[offset(a)];
    }

    public static void putByte(long a, byte v) {
        page(a)[offset(a)] = v;
    }

    public static short getShort(long a) {
        byte[] p = page(a);
        int o = offset(a);
        return (short) ((p[o] & 0xff) | (p[o + 1] << 8));
    }

    public static void putShort(long a, short v) {
        byte[] p = page(a);
        int o = offset(a);
        p[o] = (byte) v;
        p[o + 1] = (byte) (v >> 8);
    }

    public static int getInt(long a) {
        byte[] p = page(a);
        int o = offset(a);
        return (p[o] & 0xff) | ((p[o + 1] & 0xff) << 8) | ((p[o + 2] & 0xff) << 16) | (p[o + 3] << 24);
    }

    public static void putInt(long a, int v) {
        byte[] p = page(a);
        int o = offset(a);
        p[o] = (byte) v;
        p[o + 1] = (byte) (v >> 8);
        p[o + 2] = (byte) (v >> 16);
        p[o + 3] = (byte) (v >> 24);
    }

    public static long getLong(long a) {
        return (getInt(a) & 0xffffffffL) | ((long) getInt(a + 4) << 32);
    }

    public static void putLong(long a, long v) {
        putInt(a, (int) v);
        putInt(a + 4, (int) (v >> 32));
    }

    public static float getFloat(long a) {
        return Float.intBitsToFloat(getInt(a));
    }

    public static void putFloat(long a, float v) {
        putInt(a, Float.floatToRawIntBits(v));
    }

    public static double getDouble(long a) {
        return Double.longBitsToDouble(getLong(a));
    }

    public static void putDouble(long a, double v) {
        putLong(a, Double.doubleToRawLongBits(v));
    }

    public static void copy(long src, long dst, long bytes) {
        System.arraycopy(page(src), offset(src), page(dst), offset(dst), (int) bytes);
    }

    public static void set(long a, int value, long bytes) {
        byte[] p = page(a);
        int o = offset(a);
        java.util.Arrays.fill(p, o, o + (int) bytes, (byte) value);
    }
}
