package org.teavm.mcweb.host;

import org.teavm.jso.JSBody;
import org.teavm.jso.typedarrays.Int8Array;

/**
 * Bridge to mcshader.wasm (glslang + SPIRV-Cross built for wasm32), loaded by the page as
 * {@code globalThis.mcshader}. Pointers into that module's memory travel as Java ints.
 */
public final class Mcshader {
    private Mcshader() {
    }

    @JSBody(script = "return !!globalThis.mcshader;")
    public static native boolean ready();

    @JSBody(params = {"fn", "a", "b", "c", "d", "e"}, script = "return mcshader.exports[fn](a, b, c, d, e) | 0;")
    public static native int call(String fn, int a, int b, int c, int d, int e);

    public static int call(String fn) {
        return call(fn, 0, 0, 0, 0, 0);
    }

    public static int call(String fn, int a) {
        return call(fn, a, 0, 0, 0, 0);
    }

    public static int call(String fn, int a, int b) {
        return call(fn, a, b, 0, 0, 0);
    }

    public static int call(String fn, int a, int b, int c) {
        return call(fn, a, b, c, 0, 0);
    }

    public static int call(String fn, int a, int b, int c, int d) {
        return call(fn, a, b, c, d, 0);
    }

    @JSBody(params = "n", script = "return mcshader.exports.malloc(n);")
    public static native int malloc(int n);

    @JSBody(params = "p", script = "mcshader.exports.free(p);")
    public static native void free(int p);

    @JSBody(params = {"p", "data"}, script = "mcshader.u8().set(data, p);")
    private static native void write(int p, Int8Array data);

    public static void write(int p, byte[] data) {
        write(p, Int8Array.copyFromJavaArray(data));
    }

    @JSBody(params = {"p", "n"}, script = "return new Int8Array(mcshader.memory.buffer.slice(p, p + n));")
    private static native Int8Array read(int p, int n);

    public static byte[] readBytes(int p, int n) {
        return read(p, n).copyToJavaArray();
    }

    @JSBody(params = "p", script = "return mcshader.dv().getUint32(p, true) | 0;")
    public static native int readInt(int p);

    @JSBody(params = {"p", "v"}, script = "mcshader.dv().setUint32(p, v, true);")
    public static native void writeInt(int p, int v);

    @JSBody(params = "s", script = "return mcshader.cstr(s);")
    public static native int cstr(String s);

    @JSBody(params = "p", script = "return mcshader.str(p);")
    public static native String str(int p);

    // GLSL -> SPIR-V ------------------------------------------------------------------------------

    /** Compiles; returns the SPIR-V size in words, or -1 (see {@link #error()}). */
    @JSBody(params = {"source", "stage", "preamble", "autoBind"}, script = ""
            + "var r = mcshader.compile(source, stage, preamble, autoBind);"
            + "mcshader.last = r;"
            + "return r.error ? -1 : r.words.length;")
    public static native int compile(String source, int stage, String preamble, boolean autoBind);

    @JSBody(script = "return mcshader.last.error;")
    public static native String error();

    @JSBody(script = "return new Int8Array(mcshader.last.words.buffer, mcshader.last.words.byteOffset, mcshader.last.words.byteLength);")
    private static native Int8Array spirvBytes();

    public static byte[] spirv() {
        return spirvBytes().copyToJavaArray();
    }
}
