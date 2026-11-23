package org.lwjgl.opengl;

import java.nio.ByteBuffer;
import java.util.HashMap;
import java.util.Map;
import org.lwjgl.system.Heap;
import org.teavm.jso.JSObject;
import org.teavm.jso.typedarrays.Int8Array;
import org.teavm.jso.webgl.WebGL2RenderingContext;
import org.teavm.jso.webgl.WebGLBuffer;
import org.teavm.jso.webgl.WebGLFramebuffer;
import org.teavm.jso.webgl.WebGLProgram;
import org.teavm.jso.webgl.WebGLQuery;
import org.teavm.jso.webgl.WebGLSampler;
import org.teavm.jso.webgl.WebGLShader;
import org.teavm.jso.webgl.WebGLSync;
import org.teavm.jso.webgl.WebGLTexture;
import org.teavm.jso.webgl.WebGLUniformLocation;
import org.teavm.jso.webgl.WebGLVertexArrayObject;
import org.teavm.mcweb.host.Gl;
import org.teavm.mcweb.host.GlslEs;

/**
 * OpenGL 3.3 core entry points implemented on WebGL2 (OpenGL ES 3.0). Where WebGL2 has no equivalent
 * (buffer mapping, polygon mode, logic ops, per-attachment blend state) the call is emulated or is a
 * documented no-op.
 */
public final class GL33C {
    private GL33C() {
    }

    private static WebGL2RenderingContext g() {
        return Gl.gl;
    }

    // constants the shim itself needs
    private static final int TEXTURE_2D = 0x0DE1;
    private static final int ARRAY_BUFFER = 0x8892;
    private static final int ELEMENT_ARRAY_BUFFER = 0x8893;
    private static final int MAP_WRITE_BIT = 0x2;
    private static final int MAP_FLUSH_EXPLICIT_BIT = 0x10;

    // state tracking -----------------------------------------------------------------------------

    private static final Map<Integer, Integer> BOUND_BUFFER = new HashMap<>();
    private static final Map<Integer, Integer> BOUND_TEXTURE = new HashMap<>();
    private static final Map<Integer, Integer> SHADER_TYPE = new HashMap<>();
    private static final Map<Integer, int[]> TEXTURE_INFO = new HashMap<>(); // id -> {w,h,internalFormat} of level 0
    private static int activeUnit;
    private static int nextMapId;

    private static final class Mapping {
        long address;
        int offset;
        int length;
        int access;
    }

    private static final Map<Integer, Mapping> MAPPINGS = new HashMap<>(); // buffer id -> mapping

    private static int boundBuffer(int target) {
        Integer id = BOUND_BUFFER.get(target);
        return id == null ? 0 : id;
    }

    // buffers ------------------------------------------------------------------------------------

    public static int glGenBuffers() {
        return Gl.BUFFERS.add(g().createBuffer());
    }

    public static void glDeleteBuffers(int buffer) {
        g().deleteBuffer((WebGLBuffer) Gl.BUFFERS.get(buffer));
        Gl.BUFFERS.remove(buffer);
    }

    public static void glBindBuffer(int target, int buffer) {
        BOUND_BUFFER.put(target, buffer);
        g().bindBuffer(target, (WebGLBuffer) Gl.BUFFERS.get(buffer));
    }

    public static void glBindBufferRange(int target, int index, int buffer, long offset, long size) {
        BOUND_BUFFER.put(target, buffer);
        g().bindBufferRange(target, index, (WebGLBuffer) Gl.BUFFERS.get(buffer), (int) offset, (int) size);
    }

    public static void glBufferData(int target, long size, int usage) {
        g().bufferData(target, (int) size, usage);
    }

    public static void glBufferData(int target, ByteBuffer data, int usage) {
        g().bufferData(target, Gl.bytes(data), usage);
    }

    public static void glBufferSubData(int target, long offset, ByteBuffer data) {
        g().bufferSubData(target, (int) offset, Gl.bytes(data));
    }

    public static void glCopyBufferSubData(int readTarget, int writeTarget, long readOffset, long writeOffset,
            long size) {
        g().copyBufferSubData(readTarget, writeTarget, (int) readOffset, (int) writeOffset, (int) size);
    }

    public static long nglMapBufferRange(int target, long offset, long length, int access) {
        int buffer = boundBuffer(target);
        Mapping m = new Mapping();
        m.address = Heap.alloc(Math.max(1L, length));
        m.offset = (int) offset;
        m.length = (int) length;
        m.access = access;
        MAPPINGS.put(buffer, m);
        return m.address;
    }

    public static ByteBuffer glMapBufferRange(int target, long offset, long length, int access) {
        long address = nglMapBufferRange(target, offset, length, access);
        return Heap.byteBuffer(address, (int) length);
    }

    public static void glFlushMappedBufferRange(int target, long offset, long length) {
        Mapping m = MAPPINGS.get(boundBuffer(target));
        if (m != null && length > 0) {
            g().bufferSubData(target, m.offset + (int) offset, Gl.bytes(m.address + offset, (int) length));
        }
    }

    public static boolean glUnmapBuffer(int target) {
        int buffer = boundBuffer(target);
        Mapping m = MAPPINGS.remove(buffer);
        if (m == null) {
            return false;
        }
        if ((m.access & MAP_WRITE_BIT) != 0 && (m.access & MAP_FLUSH_EXPLICIT_BIT) == 0 && m.length > 0) {
            g().bufferSubData(target, m.offset, Gl.bytes(m.address, m.length));
        }
        Heap.free(m.address);
        return true;
    }

    // vertex arrays and attributes -------------------------------------------------------------------

    public static int glGenVertexArrays() {
        return Gl.VAOS.add(g().createVertexArray());
    }

    public static void glDeleteVertexArrays(int array) {
        g().deleteVertexArray((WebGLVertexArrayObject) Gl.VAOS.get(array));
        Gl.VAOS.remove(array);
    }

    public static void glBindVertexArray(int array) {
        g().bindVertexArray((WebGLVertexArrayObject) Gl.VAOS.get(array));
    }

    public static void glEnableVertexAttribArray(int index) {
        g().enableVertexAttribArray(index);
    }

    public static void glVertexAttribPointer(int index, int size, int type, boolean normalized, int stride,
            long pointer) {
        g().vertexAttribPointer(index, size, type, normalized, stride, (int) pointer);
    }

    public static void glVertexAttribIPointer(int index, int size, int type, int stride, long pointer) {
        g().vertexAttribIPointer(index, size, type, stride, (int) pointer);
    }

    public static void glVertexAttribDivisor(int index, int divisor) {
        g().vertexAttribDivisor(index, divisor);
    }

    // drawing ------------------------------------------------------------------------------------

    public static void glDrawArrays(int mode, int first, int count) {
        g().drawArrays(mode, first, count);
    }

    public static void glDrawArraysInstanced(int mode, int first, int count, int instances) {
        g().drawArraysInstanced(mode, first, count, instances);
    }

    public static void glDrawElements(int mode, int count, int type, long indices) {
        g().drawElements(mode, count, type, (int) indices);
    }

    public static void glDrawElementsInstancedBaseVertex(int mode, int count, int type, long indices, int instances,
            int baseVertex) {
        if (baseVertex == 0) {
            g().drawElementsInstanced(mode, count, type, (int) indices, instances);
        } else if (Gl.hasBaseVertex()) {
            Gl.drawElementsInstancedBaseVertex(mode, count, type, (int) indices, instances, baseVertex, 0);
        } else {
            throw new UnsupportedOperationException("base-vertex drawing needs WEBGL_draw_instanced_base_vertex_base_instance");
        }
    }

    public static void nglMultiDrawArrays(int mode, long firsts, long counts, int drawCount) {
        for (int i = 0; i < drawCount; i++) {
            int count = Heap.getInt(counts + 4L * i);
            if (count > 0) {
                g().drawArrays(mode, Heap.getInt(firsts + 4L * i), count);
            }
        }
    }

    public static void nglMultiDrawElementsBaseVertex(int mode, long counts, int type, long indices, int drawCount,
            long baseVertices) {
        for (int i = 0; i < drawCount; i++) {
            int count = Heap.getInt(counts + 4L * i);
            if (count > 0) {
                glDrawElementsInstancedBaseVertex(mode, count, type, Heap.getLong(indices + 8L * i), 1,
                        Heap.getInt(baseVertices + 4L * i));
            }
        }
    }

    // state --------------------------------------------------------------------------------------

    private static boolean supportedCapability(int cap) {
        switch (cap) {
            case 0x0BE2: // BLEND
            case 0x0B44: // CULL_FACE
            case 0x0B71: // DEPTH_TEST
            case 0x0BD0: // DITHER
            case 0x8037: // POLYGON_OFFSET_FILL
            case 0x809E: // SAMPLE_ALPHA_TO_COVERAGE
            case 0x80A0: // SAMPLE_COVERAGE
            case 0x0C11: // SCISSOR_TEST
            case 0x0B90: // STENCIL_TEST
            case 0x8C89: // RASTERIZER_DISCARD
                return true;
            default:
                return false; // colour logic ops, sRGB framebuffer, program point size, ...: no WebGL equivalent
        }
    }

    public static void glEnable(int cap) {
        if (supportedCapability(cap)) {
            g().enable(cap);
        }
    }

    public static void glDisable(int cap) {
        if (supportedCapability(cap)) {
            g().disable(cap);
        }
    }

    public static void glEnablei(int cap, int index) {
        if (index == 0) {
            glEnable(cap);
        }
    }

    public static void glDisablei(int cap, int index) {
        if (index == 0) {
            glDisable(cap);
        }
    }

    public static void glColorMaski(int index, boolean r, boolean gr, boolean b, boolean a) {
        if (index == 0) {
            g().colorMask(r, gr, b, a);
        }
    }

    public static void glBlendEquationSeparate(int rgb, int alpha) {
        g().blendEquationSeparate(rgb, alpha);
    }

    public static void glBlendFuncSeparate(int srcRgb, int dstRgb, int srcAlpha, int dstAlpha) {
        g().blendFuncSeparate(srcRgb, dstRgb, srcAlpha, dstAlpha);
    }

    public static void glDepthFunc(int func) {
        g().depthFunc(func);
    }

    public static void glDepthMask(boolean flag) {
        g().depthMask(flag);
    }

    public static void glClearDepth(double depth) {
        g().clearDepth((float) depth);
    }

    public static void glClearColor(float r, float gr, float b, float a) {
        g().clearColor(r, gr, b, a);
    }

    public static void glClear(int mask) {
        g().clear(mask);
    }

    public static void glClearBufferfv(int buffer, int drawBuffer, float[] value) {
        g().clearBufferfv(buffer, drawBuffer, value);
    }

    public static void glPolygonOffset(float factor, float units) {
        g().polygonOffset(factor, units);
    }

    /** Wireframe and point polygon modes do not exist in WebGL. */
    public static void glPolygonMode(int face, int mode) {
    }

    /** Colour logic ops do not exist in WebGL. */
    public static void glLogicOp(int op) {
    }

    public static void glScissor(int x, int y, int w, int h) {
        g().scissor(x, y, w, h);
    }

    public static void glViewport(int x, int y, int w, int h) {
        g().viewport(x, y, w, h);
    }

    public static void glPixelStorei(int pname, int param) {
        switch (pname) {
            case 0x0CF5: // UNPACK_ALIGNMENT
            case 0x0D05: // PACK_ALIGNMENT
            case 0x0CF2: // UNPACK_ROW_LENGTH
            case 0x0CF3: // UNPACK_SKIP_ROWS
            case 0x0CF4: // UNPACK_SKIP_PIXELS
            case 0x0D02: // PACK_ROW_LENGTH
            case 0x0D03: // PACK_SKIP_ROWS
            case 0x0D04: // PACK_SKIP_PIXELS
            case 0x806E: // UNPACK_SKIP_IMAGES
            case 0x806D: // UNPACK_IMAGE_HEIGHT
                g().pixelStorei(pname, param);
                break;
            default:
                break; // swap-bytes / LSB-first / compressed block sizes are not available
        }
    }

    public static int glGetError() {
        return g().getError();
    }

    public static void glFinish() {
        g().finish();
    }

    // framebuffers -------------------------------------------------------------------------------

    public static int glGenFramebuffers() {
        return Gl.FRAMEBUFFERS.add(g().createFramebuffer());
    }

    public static void glDeleteFramebuffers(int framebuffer) {
        g().deleteFramebuffer((WebGLFramebuffer) Gl.FRAMEBUFFERS.get(framebuffer));
        Gl.FRAMEBUFFERS.remove(framebuffer);
    }

    public static void glBindFramebuffer(int target, int framebuffer) {
        g().bindFramebuffer(target, (WebGLFramebuffer) Gl.FRAMEBUFFERS.get(framebuffer));
    }

    public static void glFramebufferTexture2D(int target, int attachment, int textureTarget, int texture, int level) {
        g().framebufferTexture2D(target, attachment, textureTarget, (WebGLTexture) Gl.TEXTURES.get(texture), level);
    }

    public static void glBlitFramebuffer(int srcX0, int srcY0, int srcX1, int srcY1, int dstX0, int dstY0, int dstX1,
            int dstY1, int mask, int filter) {
        g().blitFramebuffer(srcX0, srcY0, srcX1, srcY1, dstX0, dstY0, dstX1, dstY1, mask, filter);
    }

    public static void glDrawBuffer(int mode) {
        g().drawBuffers(new int[] {mode});
    }

    public static void glDrawBuffers(int[] buffers) {
        g().drawBuffers(buffers);
    }

    public static void glReadBuffer(int mode) {
        g().readBuffer(mode);
    }

    public static void glReadPixels(int x, int y, int w, int h, int format, int type, long offset) {
        Gl.readPixelsOffset(x, y, w, h, format, type, (int) offset);
    }

    // textures and samplers ------------------------------------------------------------------------

    public static int glGenTextures() {
        return Gl.TEXTURES.add(g().createTexture());
    }

    public static void glDeleteTextures(int texture) {
        g().deleteTexture((WebGLTexture) Gl.TEXTURES.get(texture));
        Gl.TEXTURES.remove(texture);
        TEXTURE_INFO.remove(texture);
    }

    public static void glActiveTexture(int texture) {
        activeUnit = texture - 0x84C0;
        g().activeTexture(texture);
    }

    public static void glBindTexture(int target, int texture) {
        BOUND_TEXTURE.put(activeUnit * 0x10000 + target, texture);
        g().bindTexture(target, (WebGLTexture) Gl.TEXTURES.get(texture));
    }

    public static void glTexParameteri(int target, int pname, int param) {
        g().texParameteri(target, pname, param);
    }

    private static void recordTexture(int target, int level, int w, int h, int internalFormat) {
        if (level == 0) {
            Integer id = BOUND_TEXTURE.get(activeUnit * 0x10000 + target);
            if (id != null && id != 0) {
                TEXTURE_INFO.put(id, new int[] {w, h, internalFormat});
            }
        }
    }

    public static void glTexImage2D(int target, int level, int internalFormat, int w, int h, int border, int format,
            int type, ByteBuffer pixels) {
        recordTexture(target, level, w, h, internalFormat);
        if (pixels == null) {
            Gl.texImage2DEmpty(target, level, internalFormat, w, h, border, format, type);
        } else {
            g().texImage2D(target, level, internalFormat, w, h, border, format, type,
                    Gl.typed(Gl.bytes(pixels), type));
        }
    }

    public static void glTexSubImage2D(int target, int level, int x, int y, int w, int h, int format, int type,
            ByteBuffer pixels) {
        g().texSubImage2D(target, level, x, y, w, h, format, type, Gl.typed(Gl.bytes(pixels), type));
    }

    /** Upload from the bound pixel-unpack buffer at the given byte offset. */
    public static void glTexSubImage2D(int target, int level, int x, int y, int w, int h, int format, int type,
            long offset) {
        Gl.texSubImage2DOffset(target, level, x, y, w, h, format, type, (int) offset);
    }

    public static int glGetTexLevelParameteri(int target, int level, int pname) {
        Integer id = BOUND_TEXTURE.get(activeUnit * 0x10000 + target);
        int[] info = id == null ? null : TEXTURE_INFO.get(id);
        if (info == null || level != 0) {
            return 0;
        }
        switch (pname) {
            case 0x1000: // TEXTURE_WIDTH
                return info[0];
            case 0x1001: // TEXTURE_HEIGHT
                return info[1];
            case 0x1003: // TEXTURE_INTERNAL_FORMAT
                return info[2];
            default:
                return 0;
        }
    }

    /** Texture buffer objects do not exist in WebGL2. */
    public static void glTexBuffer(int target, int internalFormat, int buffer) {
        Gl.warn("glTexBuffer is not supported by WebGL2");
    }

    public static int glGenSamplers() {
        return Gl.SAMPLERS.add(g().createSampler());
    }

    public static void glDeleteSamplers(int sampler) {
        g().deleteSampler((WebGLSampler) Gl.SAMPLERS.get(sampler));
        Gl.SAMPLERS.remove(sampler);
    }

    public static void glBindSampler(int unit, int sampler) {
        g().bindSampler(unit, (WebGLSampler) Gl.SAMPLERS.get(sampler));
    }

    public static void glSamplerParameteri(int sampler, int pname, int param) {
        g().samplerParameteri((WebGLSampler) Gl.SAMPLERS.get(sampler), pname, param);
    }

    public static void glSamplerParameterf(int sampler, int pname, float param) {
        g().samplerParameterf((WebGLSampler) Gl.SAMPLERS.get(sampler), pname, param);
    }

    // shaders and programs -------------------------------------------------------------------------

    public static int glCreateShader(int type) {
        int id = Gl.SHADERS.add(g().createShader(type));
        SHADER_TYPE.put(id, type);
        return id;
    }

    public static void glDeleteShader(int shader) {
        g().deleteShader((WebGLShader) Gl.SHADERS.get(shader));
        Gl.SHADERS.remove(shader);
        SHADER_TYPE.remove(shader);
    }

    public static void nglShaderSource(int shader, int count, long strings, long lengths) {
        StringBuilder sb = new StringBuilder();
        for (int i = 0; i < count; i++) {
            long ptr = Heap.getLong(strings + 8L * i);
            int len = lengths == 0 ? -1 : Heap.getInt(lengths + 4L * i);
            if (len < 0) {
                len = 0;
                while (Heap.getByte(ptr + len) != 0) {
                    len++;
                }
            }
            byte[] raw = new byte[len];
            for (int j = 0; j < len; j++) {
                raw[j] = Heap.getByte(ptr + j);
            }
            sb.append(new String(raw, java.nio.charset.StandardCharsets.UTF_8));
        }
        g().shaderSource((WebGLShader) Gl.SHADERS.get(shader), GlslEs.convert(sb.toString()));
    }

    public static void glCompileShader(int shader) {
        g().compileShader((WebGLShader) Gl.SHADERS.get(shader));
    }

    public static int glGetShaderi(int shader, int pname) {
        return Gl.shaderParameter(Gl.SHADERS.get(shader), pname);
    }

    public static String glGetShaderInfoLog(int shader, int maxLength) {
        String log = g().getShaderInfoLog((WebGLShader) Gl.SHADERS.get(shader));
        return log == null ? "" : log;
    }

    public static int glCreateProgram() {
        return Gl.PROGRAMS.add(g().createProgram());
    }

    public static void glDeleteProgram(int program) {
        g().deleteProgram((WebGLProgram) Gl.PROGRAMS.get(program));
        Gl.PROGRAMS.remove(program);
    }

    public static void glAttachShader(int program, int shader) {
        g().attachShader((WebGLProgram) Gl.PROGRAMS.get(program), (WebGLShader) Gl.SHADERS.get(shader));
    }

    public static void glBindAttribLocation(int program, int index, CharSequence name) {
        g().bindAttribLocation((WebGLProgram) Gl.PROGRAMS.get(program), index, name.toString());
    }

    public static void glLinkProgram(int program) {
        g().linkProgram((WebGLProgram) Gl.PROGRAMS.get(program));
    }

    public static void glUseProgram(int program) {
        g().useProgram((WebGLProgram) Gl.PROGRAMS.get(program));
    }

    public static int glGetProgrami(int program, int pname) {
        return Gl.programParameter(Gl.PROGRAMS.get(program), pname);
    }

    public static String glGetProgramInfoLog(int program, int maxLength) {
        String log = g().getProgramInfoLog((WebGLProgram) Gl.PROGRAMS.get(program));
        return log == null ? "" : log;
    }

    public static int glGetUniformLocation(int program, CharSequence name) {
        JSObject location = Gl.uniformLocation(Gl.PROGRAMS.get(program), name.toString());
        return location == null ? -1 : Gl.LOCATIONS.add(location);
    }

    public static void glUniform1i(int location, int value) {
        if (location >= 0) {
            g().uniform1i((WebGLUniformLocation) Gl.LOCATIONS.get(location), value);
        }
    }

    public static int glGetUniformBlockIndex(int program, CharSequence name) {
        return g().getUniformBlockIndex((WebGLProgram) Gl.PROGRAMS.get(program), name.toString());
    }

    public static void glUniformBlockBinding(int program, int blockIndex, int binding) {
        g().uniformBlockBinding((WebGLProgram) Gl.PROGRAMS.get(program), blockIndex, binding);
    }

    // queries and sync ------------------------------------------------------------------------------

    public static void glGenQueries(int[] ids) {
        for (int i = 0; i < ids.length; i++) {
            ids[i] = Gl.QUERIES.add(g().createQuery());
        }
    }

    public static void glDeleteQueries(int[] ids) {
        for (int id : ids) {
            g().deleteQuery((WebGLQuery) Gl.QUERIES.get(id));
            Gl.QUERIES.remove(id);
        }
    }

    /** GPU timestamp queries are not part of WebGL2 core; results always read as zero. */
    public static void glQueryCounter(int id, int target) {
    }

    public static int glGetQueryObjecti(int id, int pname) {
        return pname == 0x8867 ? 1 : 0; // QUERY_RESULT_AVAILABLE
    }

    public static long glGetQueryObjectui64(int id, int pname) {
        return 0L;
    }

    public static long glFenceSync(int condition, int flags) {
        return Gl.SYNCS.add(g().fenceSync(condition, flags));
    }

    public static int glClientWaitSync(long sync, int flags, long timeout) {
        return g().clientWaitSync((WebGLSync) Gl.SYNCS.get((int) sync), flags, 0);
    }

    public static void glDeleteSync(long sync) {
        g().deleteSync((WebGLSync) Gl.SYNCS.get((int) sync));
        Gl.SYNCS.remove((int) sync);
    }

    // queries of state ------------------------------------------------------------------------------

    public static String glGetString(int name) {
        switch (name) {
            case 0x1F00: // VENDOR
                return orDefault(Gl.unmaskedInfo(0), "WebGL");
            case 0x1F01: // RENDERER
                return orDefault(Gl.unmaskedInfo(1), "WebGL2") + " (WebGL2)";
            case 0x1F02: // VERSION
                return "3.3.0 WebGL 2.0 (browser port)";
            case 0x8B8C: // SHADING_LANGUAGE_VERSION
                return "3.30";
            default:
                return "";
        }
    }

    private static String orDefault(String v, String d) {
        return v == null || v.isEmpty() ? d : v;
    }

    public static int glGetInteger(int pname) {
        switch (pname) {
            case 0x821B: // MAJOR_VERSION
                return 3;
            case 0x821C: // MINOR_VERSION
                return 3;
            case 0x9122: // MAX_VERTEX_OUTPUT_COMPONENTS
                return 64;
            default:
                return (int) Gl.parameter(pname);
        }
    }

    public static long glGetInteger64(int pname) {
        return (long) Gl.parameter(pname);
    }

    public static float glGetFloat(int pname) {
        return (float) Gl.parameter(pname);
    }
}
