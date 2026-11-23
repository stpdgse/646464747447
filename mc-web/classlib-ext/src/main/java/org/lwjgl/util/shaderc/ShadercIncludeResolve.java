package org.lwjgl.util.shaderc;

import org.lwjgl.system.NativeResource;

/** In real LWJGL this wraps a native function pointer; here the include resolver is called directly. */
public final class ShadercIncludeResolve implements NativeResource {
    final ShadercIncludeResolveI function;

    private ShadercIncludeResolve(ShadercIncludeResolveI function) {
        this.function = function;
    }

    public static ShadercIncludeResolve create(ShadercIncludeResolveI function) {
        return new ShadercIncludeResolve(function);
    }

    @Override
    public void free() {
    }
}
