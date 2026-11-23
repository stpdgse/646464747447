package org.lwjgl.util.spvc;

import java.nio.IntBuffer;
import org.lwjgl.PointerBuffer;
import org.lwjgl.system.Heap;
import org.lwjgl.system.Library;
import org.lwjgl.system.SharedLibrary;
import org.teavm.mcweb.host.Mcshader;

/**
 * SPIRV-Cross C API, forwarded to the wasm module. Handles are pointers into the module's memory
 * (they fit in an int); data crossing back to the game is copied onto the emulated native heap.
 */
public class Spvc {
    public static SharedLibrary getLibrary() {
        return Library.loadNative("spvc", "spirv-cross-c-shared");
    }

    private static long out(PointerBuffer buffer, int slot) {
        long value = Mcshader.readInt(slot) & 0xFFFFFFFFL;
        buffer.put(buffer.position(), value);
        return value;
    }

    private static int withSlot(String fn, PointerBuffer result, int... args) {
        int slot = Mcshader.malloc(8);
        int rc;
        switch (args.length) {
            case 0:
                rc = Mcshader.call(fn, slot);
                break;
            case 1:
                rc = Mcshader.call(fn, args[0], slot);
                break;
            case 2:
                rc = Mcshader.call(fn, args[0], args[1], slot);
                break;
            default:
                rc = Mcshader.call(fn, args[0], args[1], args[2], slot);
                break;
        }
        if (rc == 0) {
            out(result, slot);
        }
        Mcshader.free(slot);
        return rc;
    }

    public static int spvc_context_create(PointerBuffer context) {
        return withSlot("spvc_context_create", context);
    }

    public static void spvc_context_destroy(long context) {
        Mcshader.call("spvc_context_destroy", (int) context);
    }

    public static int spvc_context_parse_spirv(long context, IntBuffer spirv, long wordCount, PointerBuffer parsedIr) {
        int n = (int) wordCount;
        byte[] raw = new byte[n * 4];
        int base = spirv.position();
        for (int i = 0; i < n; i++) {
            int w = spirv.get(base + i);
            raw[i * 4] = (byte) w;
            raw[i * 4 + 1] = (byte) (w >> 8);
            raw[i * 4 + 2] = (byte) (w >> 16);
            raw[i * 4 + 3] = (byte) (w >> 24);
        }
        int memory = Mcshader.malloc(raw.length);
        Mcshader.write(memory, raw);
        int slot = Mcshader.malloc(8);
        int rc = Mcshader.call("spvc_context_parse_spirv", (int) context, memory, n, slot);
        if (rc == 0) {
            out(parsedIr, slot);
        }
        Mcshader.free(slot);
        // SPIRV-Cross copies the words into the parsed IR, so the staging buffer can go
        Mcshader.free(memory);
        return rc;
    }

    public static int spvc_context_create_compiler(long context, int backend, long parsedIr, int captureMode,
            PointerBuffer compiler) {
        int slot = Mcshader.malloc(8);
        int rc = Mcshader.call("spvc_context_create_compiler", (int) context, backend, (int) parsedIr, captureMode,
                slot);
        if (rc == 0) {
            out(compiler, slot);
        }
        Mcshader.free(slot);
        return rc;
    }

    public static int spvc_compiler_create_compiler_options(long compiler, PointerBuffer options) {
        return withSlot("spvc_compiler_create_compiler_options", options, (int) compiler);
    }

    public static int spvc_compiler_options_set_bool(long options, int option, boolean value) {
        return Mcshader.call("spvc_compiler_options_set_bool", (int) options, option, value ? 1 : 0);
    }

    public static int spvc_compiler_options_set_uint(long options, int option, int value) {
        return Mcshader.call("spvc_compiler_options_set_uint", (int) options, option, value);
    }

    public static int spvc_compiler_install_compiler_options(long compiler, long options) {
        return Mcshader.call("spvc_compiler_install_compiler_options", (int) compiler, (int) options);
    }

    public static int spvc_compiler_set_entry_point(long compiler, CharSequence name, int executionModel) {
        int s = Mcshader.cstr(name.toString());
        int rc = Mcshader.call("spvc_compiler_set_entry_point", (int) compiler, s, executionModel);
        Mcshader.free(s);
        return rc;
    }

    public static void spvc_compiler_set_name(long compiler, int id, CharSequence name) {
        int s = Mcshader.cstr(name.toString());
        Mcshader.call("spvc_compiler_set_name", (int) compiler, id, s);
        Mcshader.free(s);
    }

    public static String spvc_compiler_get_name(long compiler, int id) {
        return Mcshader.str(Mcshader.call("spvc_compiler_get_name", (int) compiler, id));
    }

    /** Result strings live as long as the context in the module; this copy lives on the game's heap. */
    public static int spvc_compiler_compile(long compiler, PointerBuffer source) {
        int slot = Mcshader.malloc(8);
        int rc = Mcshader.call("spvc_compiler_compile", (int) compiler, slot);
        if (rc == 0) {
            String text = Mcshader.str(Mcshader.readInt(slot));
            byte[] raw = text.getBytes(java.nio.charset.StandardCharsets.UTF_8);
            long address = Heap.alloc(raw.length + 1);
            for (int i = 0; i < raw.length; i++) {
                Heap.putByte(address + i, raw[i]);
            }
            Heap.putByte(address + raw.length, (byte) 0);
            source.put(source.position(), address);
        }
        Mcshader.free(slot);
        return rc;
    }

    public static int spvc_compiler_create_shader_resources(long compiler, PointerBuffer resources) {
        return withSlot("spvc_compiler_create_shader_resources", resources, (int) compiler);
    }

    public static int spvc_resources_get_resource_list_for_type(long resources, int type, PointerBuffer list,
            PointerBuffer count) {
        int listSlot = Mcshader.malloc(8);
        int countSlot = listSlot + 4;
        int rc = Mcshader.call("spvc_resources_get_resource_list_for_type", (int) resources, type, listSlot, countSlot);
        if (rc == 0) {
            int src = Mcshader.readInt(listSlot);
            int n = Mcshader.readInt(countSlot);
            long table = Heap.alloc(Math.max(1, n) * SpvcReflectedResource.SIZEOF);
            for (int i = 0; i < n; i++) {
                int entry = src + i * 16; // wasm32 layout: id, base_type_id, type_id, name*
                long dst = table + (long) i * SpvcReflectedResource.SIZEOF;
                Heap.putInt(dst, Mcshader.readInt(entry));
                Heap.putInt(dst + 4, Mcshader.readInt(entry + 4));
                Heap.putInt(dst + 8, Mcshader.readInt(entry + 8));
                String name = Mcshader.str(Mcshader.readInt(entry + 12));
                long namePtr = 0;
                if (name != null) {
                    byte[] raw = name.getBytes(java.nio.charset.StandardCharsets.UTF_8);
                    namePtr = Heap.alloc(raw.length + 1);
                    for (int j = 0; j < raw.length; j++) {
                        Heap.putByte(namePtr + j, raw[j]);
                    }
                    Heap.putByte(namePtr + raw.length, (byte) 0);
                }
                Heap.putLong(dst + 16, namePtr);
            }
            list.put(list.position(), table);
            count.put(count.position(), n);
        }
        Mcshader.free(listSlot);
        return rc;
    }

    public static int spvc_compiler_get_decoration(long compiler, int id, int decoration) {
        return Mcshader.call("spvc_compiler_get_decoration", (int) compiler, id, decoration);
    }

    public static boolean spvc_compiler_get_binary_offset_for_decoration(long compiler, int id, int decoration,
            IntBuffer offset) {
        int slot = Mcshader.malloc(8);
        int rc = Mcshader.call("spvc_compiler_get_binary_offset_for_decoration", (int) compiler, id, decoration,
                slot);
        boolean ok = (rc & 0xFF) != 0;
        if (ok) {
            offset.put(offset.position(), Mcshader.readInt(slot));
        }
        Mcshader.free(slot);
        return ok;
    }

    public static int spvc_compiler_get_declared_struct_size(long compiler, long structType, PointerBuffer size) {
        return withSlot("spvc_compiler_get_declared_struct_size", size, (int) compiler, (int) structType);
    }

    public static long spvc_compiler_get_type_handle(long compiler, int id) {
        return Mcshader.call("spvc_compiler_get_type_handle", (int) compiler, id) & 0xFFFFFFFFL;
    }

    public static int spvc_type_get_array_dimension(long type, int dimension) {
        return Mcshader.call("spvc_type_get_array_dimension", (int) type, dimension);
    }

    public static int spvc_type_get_basetype(long type) {
        return Mcshader.call("spvc_type_get_basetype", (int) type);
    }

    public static int spvc_type_get_image_dimension(long type) {
        return Mcshader.call("spvc_type_get_image_dimension", (int) type);
    }

    public static int spvc_type_get_num_array_dimensions(long type) {
        return Mcshader.call("spvc_type_get_num_array_dimensions", (int) type);
    }

    public static int spvc_type_get_vector_size(long type) {
        return Mcshader.call("spvc_type_get_vector_size", (int) type);
    }
}
