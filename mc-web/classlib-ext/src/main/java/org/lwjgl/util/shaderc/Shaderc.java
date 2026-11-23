package org.lwjgl.util.shaderc;

import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import org.lwjgl.system.Heap;
import org.lwjgl.system.Library;
import org.lwjgl.system.SharedLibrary;
import org.teavm.mcweb.host.Mcshader;

/**
 * shaderc on top of glslang (mcshader.wasm). Include directives are expanded here by calling the game's
 * include resolver, so the module itself only ever sees one flat source string.
 */
public class Shaderc {
    public static final int shaderc_compilation_status_success = 0;
    public static final int shaderc_compilation_status_compilation_error = 2;

    public static SharedLibrary getLibrary() {
        return Library.loadNative("shaderc", "shaderc_shared");
    }

    private static final class Options {
        final List<String[]> macros = new ArrayList<>();
        ShadercIncludeResolveI resolve;
        ShadercIncludeResultReleaseI release;
        long userData;
        boolean autoBind;
    }

    private static final class Result {
        int status;
        String error = "";
        ByteBuffer bytes;
    }

    private static final Map<Long, Options> OPTIONS = new HashMap<>();
    private static final Map<Long, Result> RESULTS = new HashMap<>();
    private static long nextHandle = 1;

    public static long shaderc_compiler_initialize() {
        return nextHandle++;
    }

    public static void shaderc_compiler_release(long compiler) {
    }

    public static long shaderc_compile_options_initialize() {
        long handle = nextHandle++;
        OPTIONS.put(handle, new Options());
        return handle;
    }

    public static void shaderc_compile_options_release(long options) {
        OPTIONS.remove(options);
    }

    public static void shaderc_compile_options_add_macro_definition(long options, CharSequence name,
            CharSequence value) {
        OPTIONS.get(options).macros.add(new String[] {name.toString(), value == null ? "" : value.toString()});
    }

    public static void shaderc_compile_options_set_target_env(long options, int env, int version) {
    }

    public static void shaderc_compile_options_set_auto_bind_uniforms(long options, boolean enable) {
        OPTIONS.get(options).autoBind = enable;
    }

    public static void shaderc_compile_options_set_preserve_bindings(long options, boolean enable) {
    }

    public static void shaderc_compile_options_set_generate_debug_info(long options) {
    }

    public static void shaderc_compile_options_set_optimization_level(long options, int level) {
    }

    public static void shaderc_compile_options_set_include_callbacks(long options, ShadercIncludeResolveI resolve,
            ShadercIncludeResultReleaseI release, long userData) {
        Options o = OPTIONS.get(options);
        o.resolve = resolve;
        o.release = release;
        o.userData = userData;
    }

    // compilation -------------------------------------------------------------------------------

    public static long shaderc_compile_into_spv(long compiler, ByteBuffer source, int kind, ByteBuffer inputFileName,
            ByteBuffer entryPoint, long optionsHandle) {
        Options options = OPTIONS.get(optionsHandle);
        Result result = new Result();
        long handle = nextHandle++;
        RESULTS.put(handle, result);
        String name = inputFileName == null ? "shader" : decode(inputFileName);
        try {
            StringBuilder flat = new StringBuilder();
            expand(decode(source), name, options, 0, flat, new int[] {0});
            StringBuilder preamble = new StringBuilder();
            for (String[] macro : options.macros) {
                preamble.append("#define ").append(macro[0]);
                if (!macro[1].isEmpty()) {
                    preamble.append(' ').append(macro[1]);
                }
                preamble.append('\n');
            }
            int words = Mcshader.compile(flat.toString(), kind, preamble.toString(), options.autoBind);
            if (words < 0) {
                result.status = shaderc_compilation_status_compilation_error;
                result.error = Mcshader.error();
            } else {
                byte[] spirv = Mcshader.spirv();
                long address = Heap.alloc(spirv.length);
                ByteBuffer buffer = Heap.byteBuffer(address, spirv.length);
                buffer.put(spirv);
                buffer.flip();
                result.bytes = buffer;
            }
        } catch (RuntimeException e) {
            result.status = shaderc_compilation_status_compilation_error;
            result.error = String.valueOf(e.getMessage());
        }
        return handle;
    }

    public static int shaderc_result_get_compilation_status(long result) {
        return RESULTS.get(result).status;
    }

    public static String shaderc_result_get_error_message(long result) {
        return RESULTS.get(result).error;
    }

    public static ByteBuffer shaderc_result_get_bytes(long result) {
        return RESULTS.get(result).bytes;
    }

    public static void shaderc_result_release(long result) {
        Result r = RESULTS.remove(result);
        if (r != null && r.bytes != null) {
            Heap.free(Heap.addressOf(r.bytes));
        }
    }

    // includes ----------------------------------------------------------------------------------

    private static String decode(ByteBuffer b) {
        byte[] raw = new byte[b.remaining()];
        b.duplicate().get(raw);
        return new String(raw, StandardCharsets.UTF_8);
    }

    private static String readString(long address, long length) {
        byte[] raw = new byte[(int) length];
        for (int i = 0; i < raw.length; i++) {
            raw[i] = Heap.getByte(address + i);
        }
        return new String(raw, StandardCharsets.UTF_8);
    }

    private static long cString(String s) {
        byte[] raw = s.getBytes(StandardCharsets.UTF_8);
        long address = Heap.alloc(raw.length + 1);
        for (int i = 0; i < raw.length; i++) {
            Heap.putByte(address + i, raw[i]);
        }
        Heap.putByte(address + raw.length, (byte) 0);
        return address;
    }

    /** Appends {@code source} to {@code out}, replacing {@code #include} lines with the resolved text. */
    private static void expand(String source, String file, Options options, int depth, StringBuilder out,
            int[] fileCounter) {
        if (depth > 32) {
            throw new IllegalStateException("#include nested too deeply in " + file);
        }
        int fileIndex = fileCounter[0]++;
        String[] lines = source.split("\n", -1);
        for (int i = 0; i < lines.length; i++) {
            String line = lines[i];
            String trimmed = line.trim();
            if (trimmed.startsWith("#include") && options != null && options.resolve != null) {
                int open = trimmed.indexOf('"');
                int type = 0; // shaderc_include_type_relative
                int close;
                if (open >= 0) {
                    close = trimmed.indexOf('"', open + 1);
                } else {
                    open = trimmed.indexOf('<');
                    close = trimmed.indexOf('>', open + 1);
                    type = 1; // shaderc_include_type_standard
                }
                if (open < 0 || close < 0) {
                    throw new IllegalStateException(file + ":" + (i + 1) + ": malformed #include");
                }
                String requested = trimmed.substring(open + 1, close);
                long requestedPtr = cString(requested);
                long requestingPtr = cString(file);
                long resultPtr = options.resolve.invoke(options.userData, requestedPtr, type, requestingPtr, depth + 1);
                Heap.free(requestedPtr);
                Heap.free(requestingPtr);
                long namePtr = Heap.getLong(resultPtr + ShadercIncludeResult.SOURCE_NAME);
                long nameLen = Heap.getLong(resultPtr + ShadercIncludeResult.SOURCE_NAME_LENGTH);
                long contentPtr = Heap.getLong(resultPtr + ShadercIncludeResult.CONTENT);
                long contentLen = Heap.getLong(resultPtr + ShadercIncludeResult.CONTENT_LENGTH);
                String includeName = nameLen > 0 ? readString(namePtr, nameLen) : "";
                String content = readString(contentPtr, contentLen);
                if (includeName.isEmpty()) {
                    throw new IllegalStateException(file + ":" + (i + 1) + ": " + content);
                }
                out.append("#line 1 ").append(fileCounter[0]).append('\n');
                expand(content, includeName, options, depth + 1, out, fileCounter);
                out.append("\n#line ").append(i + 2).append(' ').append(fileIndex).append('\n');
                if (options.release != null) {
                    options.release.invoke(options.userData, resultPtr);
                }
            } else {
                out.append(line);
                if (i < lines.length - 1) {
                    out.append('\n');
                }
            }
        }
    }
}
