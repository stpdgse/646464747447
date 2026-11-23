package org.lwjgl.util.shaderc;

import java.nio.ByteBuffer;
import org.lwjgl.system.Heap;
import org.lwjgl.system.NativeResource;
import org.lwjgl.system.Struct;

/** shaderc_include_result on the emulated native heap: name, name length, content, content length, user data. */
public class ShadercIncludeResult extends Struct<ShadercIncludeResult> implements NativeResource {
    public static final int SOURCE_NAME = 0;
    public static final int SOURCE_NAME_LENGTH = 8;
    public static final int CONTENT = 16;
    public static final int CONTENT_LENGTH = 24;
    public static final int USER_DATA = 32;
    public static final int SIZEOF = 40;

    private ShadercIncludeResult(long address) {
        super(address);
    }

    public static ShadercIncludeResult calloc() {
        long address = Heap.alloc(SIZEOF);
        Heap.set(address, 0, SIZEOF);
        return new ShadercIncludeResult(address);
    }

    public static ShadercIncludeResult create(long address) {
        return new ShadercIncludeResult(address);
    }

    @Override
    public int sizeof() {
        return SIZEOF;
    }

    public ShadercIncludeResult source_name(ByteBuffer name) {
        Heap.putLong(address + SOURCE_NAME, Heap.addressOf(name));
        Heap.putLong(address + SOURCE_NAME_LENGTH, name.remaining());
        return this;
    }

    public ShadercIncludeResult content(ByteBuffer content) {
        Heap.putLong(address + CONTENT, Heap.addressOf(content));
        Heap.putLong(address + CONTENT_LENGTH, content.remaining());
        return this;
    }

    public ShadercIncludeResult user_data(long data) {
        Heap.putLong(address + USER_DATA, data);
        return this;
    }

    @Override
    public void free() {
        Heap.free(address);
    }
}
