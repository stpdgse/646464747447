package org.lwjgl.util.shaderc;

import org.lwjgl.system.NativeResource;

public final class ShadercIncludeResultRelease implements ShadercIncludeResultReleaseI, NativeResource {
    final ShadercIncludeResultReleaseI function;

    private ShadercIncludeResultRelease(ShadercIncludeResultReleaseI function) {
        this.function = function;
    }

    public static ShadercIncludeResultRelease create(ShadercIncludeResultReleaseI function) {
        return new ShadercIncludeResultRelease(function);
    }

    @Override
    public void invoke(long userData, long includeResult) {
        function.invoke(userData, includeResult);
    }

    @Override
    public void free() {
    }
}
