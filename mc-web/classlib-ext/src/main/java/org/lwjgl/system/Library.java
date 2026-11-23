package org.lwjgl.system;

import java.util.function.Consumer;

/**
 * Native library loading does not exist in the browser: the "libraries" are the Java/JavaScript
 * implementations in this package tree, so loading is always a no-op that returns a stub handle.
 */
public final class Library {
    public static final String JNI_LIBRARY_NAME = "lwjgl";

    private Library() {
    }

    public static void initialize() {
    }

    public static void loadSystem(String module, String name) {
    }

    public static void loadSystem(Consumer<String> load, Consumer<String> loadLibrary, Class<?> context,
            String module, String name) {
    }

    public static SharedLibrary loadNative(String module, String name) {
        return new WebLibrary(name);
    }

    public static SharedLibrary loadNative(Class<?> context, String module, String name) {
        return new WebLibrary(name);
    }

    public static SharedLibrary loadNative(Class<?> context, String module, String name, boolean bundledWithLWJGL) {
        return new WebLibrary(name);
    }

    public static SharedLibrary loadNative(Class<?> context, String module, Configuration<String> property,
            String... names) {
        return new WebLibrary(names.length > 0 ? names[0] : module);
    }

    public static final class WebLibrary implements SharedLibrary {
        private final String name;

        public WebLibrary(String name) {
            this.name = name;
        }

        @Override
        public String getName() {
            return name;
        }

        @Override
        public String getPath() {
            return "web:" + name;
        }

        @Override
        public long address() {
            return 1;
        }

        @Override
        public long getFunctionAddress(java.nio.ByteBuffer functionName) {
            // Entry points are Java methods; a non-zero address only tells callers the function exists.
            return functionName != null && functionName.remaining() > 1 ? 1L : 0L;
        }

        @Override
        public void free() {
        }
    }
}
