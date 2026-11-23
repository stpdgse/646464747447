package org.lwjgl.stb;

import org.lwjgl.system.Heap;
import org.teavm.mcweb.host.Mcshader;

/** stb_image_write: encodes through the real library, then hands the PNG to the callback like the native call. */
public final class STBImageWrite {
    private STBImageWrite() {
    }

    public static int nstbi_write_png_to_func(long callback, long context, int w, int h, int comp, long data,
            int stride) {
        STBIWriteCallback target = STBIWriteCallback.lookup(callback);
        if (target == null) {
            throw new IllegalArgumentException("unknown write callback 0x" + Long.toHexString(callback));
        }
        int rowBytes = stride == 0 ? w * comp : stride;
        int pixels = Mcshader.copyIn(data, rowBytes * h);
        int lengthSlot = Mcshader.malloc(4);
        int png = Mcshader.call("stbi_write_png_to_mem", pixels, stride, w, h, comp, lengthSlot);
        Mcshader.free(pixels);
        int length = Mcshader.readInt(lengthSlot);
        Mcshader.free(lengthSlot);
        if (png == 0) {
            return 0;
        }
        long address = Heap.alloc(length);
        Mcshader.copyOut(png, address, length);
        Mcshader.call("stbi_image_free", png);
        try {
            target.invoke(context, address, length);
        } finally {
            Heap.free(address);
        }
        return 1;
    }
}
