package org.lwjgl.system;

public interface SharedLibrary extends FunctionProvider, NativeResource, Pointer {
    String getName();

    String getPath();
}
