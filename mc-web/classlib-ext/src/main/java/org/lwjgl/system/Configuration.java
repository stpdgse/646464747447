package org.lwjgl.system;

import java.util.HashMap;
import java.util.Map;

/** Configuration values are held in memory; nothing reads system properties in the browser. */
public class Configuration<T> {
    private static final Map<String, Object> VALUES = new HashMap<>();

    public static final Configuration<String> LIBRARY_PATH = new Configuration<>("org.lwjgl.librarypath");
    public static final Configuration<String> SHARED_LIBRARY_EXTRACT_DIRECTORY =
            new Configuration<>("org.lwjgl.system.SharedLibraryExtractDirectory");
    public static final Configuration<String> SHARED_LIBRARY_EXTRACT_PATH =
            new Configuration<>("org.lwjgl.system.SharedLibraryExtractPath");
    public static final Configuration<Boolean> SHARED_LIBRARY_EXTRACT_FORCE =
            new Configuration<>("org.lwjgl.system.SharedLibraryExtractForce");
    public static final Configuration<Boolean> EMULATE_SYSTEM_LOADLIBRARY =
            new Configuration<>("org.lwjgl.system.EmulateSystemLoadLibrary");
    public static final Configuration<String> LIBRARY_NAME = new Configuration<>("org.lwjgl.libname");
    public static final Configuration<Integer> STACK_SIZE = new Configuration<>("org.lwjgl.system.stackSize");
    public static final Configuration<Boolean> DISABLE_CHECKS = new Configuration<>("org.lwjgl.util.NoChecks");
    public static final Configuration<Boolean> DEBUG = new Configuration<>("org.lwjgl.util.Debug");
    public static final Configuration<Boolean> DEBUG_LOADER = new Configuration<>("org.lwjgl.util.DebugLoader");
    public static final Configuration<Object> DEBUG_STREAM = new Configuration<>("org.lwjgl.util.DebugStream");
    public static final Configuration<Boolean> DEBUG_MEMORY_ALLOCATOR =
            new Configuration<>("org.lwjgl.util.DebugAllocator");
    public static final Configuration<Boolean> DEBUG_STACK = new Configuration<>("org.lwjgl.util.DebugStack");
    public static final Configuration<Boolean> DEBUG_FUNCTIONS = new Configuration<>("org.lwjgl.util.DebugFunctions");
    public static final Configuration<Boolean> VULKAN_EXPLICIT_INIT =
            new Configuration<>("org.lwjgl.vulkan.explicitInit");
    public static final Configuration<String> VULKAN_LIBRARY_NAME = new Configuration<>("org.lwjgl.vulkan.libname");
    public static final Configuration<String> OPENGL_LIBRARY_NAME = new Configuration<>("org.lwjgl.opengl.libname");
    public static final Configuration<String> OPENAL_LIBRARY_NAME = new Configuration<>("org.lwjgl.openal.libname");
    public static final Configuration<String> SDL_LIBRARY_NAME = new Configuration<>("org.lwjgl.sdl.libname");
    public static final Configuration<String> STB_LIBRARY_NAME = new Configuration<>("org.lwjgl.stb.libname");
    public static final Configuration<String> FREETYPE_LIBRARY_NAME =
            new Configuration<>("org.lwjgl.freetype.libname");

    private final String property;

    private Configuration(String property) {
        this.property = property;
    }

    @SuppressWarnings("unchecked")
    public T get() {
        return (T) VALUES.get(property);
    }

    @SuppressWarnings("unchecked")
    public T get(T defaultValue) {
        Object v = VALUES.get(property);
        return v == null ? defaultValue : (T) v;
    }

    public void set(T value) {
        if (value == null) {
            VALUES.remove(property);
        } else {
            VALUES.put(property, value);
        }
    }

    public void setIfAbsent(T value) {
        VALUES.putIfAbsent(property, value);
    }
}
