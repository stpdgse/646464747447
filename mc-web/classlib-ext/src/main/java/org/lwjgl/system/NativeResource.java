package org.lwjgl.system;

public interface NativeResource extends AutoCloseable {
    void free();

    @Override
    default void close() {
        free();
    }
}
