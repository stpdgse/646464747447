package org.lwjgl.stb;

import org.teavm.mcweb.host.Mcshader;

/** stb_image_resize2 (the real library, in WebAssembly). */
public final class STBImageResize {
    private STBImageResize() {
    }

    private static int channels(int layout) {
        switch (layout) {
            case 1:
                return 1;
            case 2:
            case 9:
            case 10:
                return 2;
            case 0:
            case 3:
                return 3;
            default:
                return 4;
        }
    }

    public static long nstbir_resize_uint8_linear(long input, int inputW, int inputH, int inputStride, long output,
            int outputW, int outputH, int outputStride, int layout) {
        int c = channels(layout);
        int inStride = inputStride == 0 ? inputW * c : inputStride;
        int outStride = outputStride == 0 ? outputW * c : outputStride;
        int in = Mcshader.copyIn(input, inStride * inputH);
        int out = Mcshader.malloc(outStride * outputH);
        int result = Mcshader.call("stbir_resize_uint8_linear", in, inputW, inputH, inStride, out, outputW, outputH,
                outStride, layout);
        long ok = 0;
        if (result != 0) {
            Mcshader.copyOut(out, output, outStride * outputH);
            ok = output;
        }
        Mcshader.free(in);
        Mcshader.free(out);
        return ok;
    }
}
