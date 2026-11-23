package org.teavm.mcweb.host;

import java.nio.ByteBuffer;
import org.teavm.jso.JSBody;
import org.teavm.jso.JSObject;
import org.teavm.jso.typedarrays.ArrayBufferView;
import org.teavm.jso.typedarrays.Int8Array;
import org.teavm.jso.webgl.WebGL2RenderingContext;

/**
 * The WebGL2 context behind the OpenGL shim: handle tables (OpenGL names are ints, WebGL objects are
 * JavaScript objects), data conversion helpers and the few calls the typed JSO API lacks.
 */
public final class Gl {
    private Gl() {
    }

    public static WebGL2RenderingContext gl;

    public static void init() {
        gl = context();
        if (gl == null) {
            throw new IllegalStateException("WebGL2 is not available in this browser");
        }
        enableExtensions();
    }

    @JSBody(script = "return mcweb.gl;")
    private static native WebGL2RenderingContext context();

    @JSBody(script = "var g = mcweb.gl;"
            + "mcweb.ext = {"
            + " bv: g.getExtension('WEBGL_draw_instanced_base_vertex_base_instance'),"
            + " mdbv: g.getExtension('WEBGL_multi_draw_instanced_base_vertex_base_instance'),"
            + " md: g.getExtension('WEBGL_multi_draw'),"
            + " timer: g.getExtension('EXT_disjoint_timer_query_webgl2'),"
            + " aniso: g.getExtension('EXT_texture_filter_anisotropic'),"
            + " dbg: g.getExtension('WEBGL_debug_renderer_info'),"
            + " floatrt: g.getExtension('EXT_color_buffer_float'),"
            + " halffloatrt: g.getExtension('EXT_color_buffer_half_float'),"
            + " floatlin: g.getExtension('OES_texture_float_linear'),"
            + " clipctl: g.getExtension('EXT_clip_control'),"
            + " s3tc: g.getExtension('WEBGL_compressed_texture_s3tc')"
            + "};")
    private static native void enableExtensions();

    @JSBody(script = "return !!mcweb.ext.bv;")
    public static native boolean hasBaseVertex();

    @JSBody(script = "return !!mcweb.ext.clipctl;")
    public static native boolean hasClipControl();

    @JSBody(params = "n", script = "return mcweb.ext.dbg ? mcweb.gl.getParameter(mcweb.ext.dbg[n ? 'UNMASKED_RENDERER_WEBGL' : 'UNMASKED_VENDOR_WEBGL']) : null;")
    public static native String unmaskedInfo(int n);

    // handle tables ------------------------------------------------------------------------------

    /** Maps the integer names the game uses to WebGL objects. Name 0 is always "none". */
    public static final class Handles {
        // Slot objects instead of a JSObject[]: TeaVM's Wasm GC backend cannot store null into JSObject arrays.
        private Slot[] items = new Slot[64];
        private int[] free = new int[16];
        private int freeCount;
        private int next = 1;

        private static final class Slot {
            JSObject object;
        }

        public int add(JSObject object) {
            int id;
            if (freeCount > 0) {
                id = free[--freeCount];
            } else {
                id = next++;
                if (id >= items.length) {
                    Slot[] grown = new Slot[items.length * 2];
                    System.arraycopy(items, 0, grown, 0, items.length);
                    items = grown;
                }
            }
            Slot slot = items[id];
            if (slot == null) {
                slot = new Slot();
                items[id] = slot;
            }
            slot.object = object;
            return id;
        }

        public JSObject get(int id) {
            if (id <= 0 || id >= items.length) {
                return null;
            }
            Slot slot = items[id];
            return slot == null ? null : slot.object;
        }

        public void remove(int id) {
            if (id > 0 && id < items.length && items[id] != null && items[id].object != null) {
                items[id].object = null;
                if (freeCount == free.length) {
                    int[] grown = new int[free.length * 2];
                    System.arraycopy(free, 0, grown, 0, free.length);
                    free = grown;
                }
                free[freeCount++] = id;
            }
        }
    }

    public static final Handles BUFFERS = new Handles();
    public static final Handles TEXTURES = new Handles();
    public static final Handles FRAMEBUFFERS = new Handles();
    public static final Handles PROGRAMS = new Handles();
    public static final Handles SHADERS = new Handles();
    public static final Handles VAOS = new Handles();
    public static final Handles SAMPLERS = new Handles();
    public static final Handles QUERIES = new Handles();
    public static final Handles SYNCS = new Handles();
    public static final Handles LOCATIONS = new Handles();

    // data -------------------------------------------------------------------------------------

    /** A JavaScript copy of the remaining bytes of a heap-backed buffer. */
    public static Int8Array bytes(ByteBuffer b) {
        if (b == null) {
            return null;
        }
        if (!b.hasArray()) {
            throw new IllegalArgumentException("direct buffers without a backing array are not supported");
        }
        return copy(b.array(), b.arrayOffset() + b.position(), b.remaining());
    }

    /** A JavaScript copy of {@code length} bytes of the emulated native heap at {@code address}. */
    public static Int8Array bytes(long address, int length) {
        return copy(org.lwjgl.system.Heap.pageOf(address), org.lwjgl.system.Heap.offsetOf(address), length);
    }

    private static Int8Array copy(byte[] source, int offset, int length) {
        // plain System.arraycopy: Arrays.copyOfRange would be compiled as a coroutine here
        byte[] part = new byte[length];
        System.arraycopy(source, offset, part, 0, length);
        return Int8Array.copyFromJavaArray(part);
    }

    /** Reinterprets bytes as the element type a texture upload with this GL type needs. */
    @JSBody(params = {"a", "type"}, script = ""
            + "switch (type) {"
            + " case 0x1401: return new Uint8Array(a.buffer, a.byteOffset, a.byteLength);"          // UNSIGNED_BYTE
            + " case 0x1400: return new Int8Array(a.buffer, a.byteOffset, a.byteLength);"
            + " case 0x1403: case 0x8033: case 0x8034: case 0x8363: case 0x140B:"                    // 16-bit unsigned
            + "   return new Uint16Array(a.buffer, a.byteOffset, a.byteLength >> 1);"
            + " case 0x1402: return new Int16Array(a.buffer, a.byteOffset, a.byteLength >> 1);"
            + " case 0x1405: case 0x8368: case 0x8C3E: case 0x84FA: case 0x8DAD:"                     // 32-bit unsigned
            + "   return new Uint32Array(a.buffer, a.byteOffset, a.byteLength >> 2);"
            + " case 0x1404: return new Int32Array(a.buffer, a.byteOffset, a.byteLength >> 2);"
            + " case 0x1406: return new Float32Array(a.buffer, a.byteOffset, a.byteLength >> 2);"
            + " default: return new Uint8Array(a.buffer, a.byteOffset, a.byteLength);"
            + "}")
    public static native ArrayBufferView typed(Int8Array bytes, int type);

    // calls the typed API lacks ------------------------------------------------------------------

    @JSBody(params = {"target", "level", "x", "y", "w", "h", "format", "type", "offset"},
            script = "mcweb.gl.texSubImage2D(target, level, x, y, w, h, format, type, offset);")
    public static native void texSubImage2DOffset(int target, int level, int x, int y, int w, int h, int format,
            int type, int offset);

    @JSBody(params = {"x", "y", "w", "h", "format", "type", "offset"},
            script = "mcweb.gl.readPixels(x, y, w, h, format, type, offset);")
    public static native void readPixelsOffset(int x, int y, int w, int h, int format, int type, int offset);

    @JSBody(params = {"target", "level", "internalFormat", "w", "h", "border", "format", "type"},
            script = "mcweb.gl.texImage2D(target, level, internalFormat, w, h, border, format, type, null);")
    public static native void texImage2DEmpty(int target, int level, int internalFormat, int w, int h, int border,
            int format, int type);

    @JSBody(params = {"mode", "count", "type", "offset", "instances", "baseVertex", "baseInstance"},
            script = "mcweb.ext.bv.drawElementsInstancedBaseVertexBaseInstanceWEBGL(mode, count, type, offset, instances, baseVertex, baseInstance);")
    public static native void drawElementsInstancedBaseVertex(int mode, int count, int type, int offset,
            int instances, int baseVertex, int baseInstance);

    @JSBody(params = {"mode", "first", "count", "instances", "baseInstance"},
            script = "mcweb.ext.bv.drawArraysInstancedBaseInstanceWEBGL(mode, first, count, instances, baseInstance);")
    public static native void drawArraysInstancedBaseInstance(int mode, int first, int count, int instances,
            int baseInstance);

    @JSBody(params = {"p"}, script = "var v = mcweb.gl.getParameter(p); return typeof v === 'number' ? v : (typeof v === 'boolean' ? (v ? 1 : 0) : 0);")
    public static native double parameter(int p);

    @JSBody(params = {"query", "pname"}, script = "var v = mcweb.gl.getQueryParameter(query, pname); return typeof v === 'boolean' ? (v ? 1 : 0) : (v | 0);")
    public static native int queryParameter(JSObject query, int pname);

    @JSBody(params = {"program", "pname"}, script = "var v = mcweb.gl.getProgramParameter(program, pname); return typeof v === 'boolean' ? (v ? 1 : 0) : (v | 0);")
    public static native int programParameter(JSObject program, int pname);

    @JSBody(params = {"shader", "pname"}, script = "var v = mcweb.gl.getShaderParameter(shader, pname); return typeof v === 'boolean' ? (v ? 1 : 0) : (v | 0);")
    public static native int shaderParameter(JSObject shader, int pname);

    @JSBody(params = {"program", "name"}, script = "var l = mcweb.gl.getUniformLocation(program, name); return l;")
    public static native JSObject uniformLocation(JSObject program, String name);

    @JSBody(params = {"target", "pname"}, script = "var v = mcweb.gl.getBufferParameter(target, pname); return v | 0;")
    public static native int bufferParameter(int target, int pname);

    /** Reads {@code size} bytes of a buffer back into a zero-padded array of {@code total} bytes. */
    @JSBody(params = {"buffer", "size", "total"}, script = ""
            + "var g = mcweb.gl;"
            + "var d = new Int8Array(total);"
            + "g.bindBuffer(g.COPY_READ_BUFFER, buffer);"
            + "g.getBufferSubData(g.COPY_READ_BUFFER, 0, d, 0, size);"
            + "return d;")
    public static native Int8Array readBuffer(JSObject buffer, int size, int total);

    @JSBody(params = {"msg"}, script = "console.warn('[gl] ' + msg);")
    public static native void warn(String msg);
}
