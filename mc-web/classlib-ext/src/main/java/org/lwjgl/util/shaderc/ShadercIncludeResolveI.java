package org.lwjgl.util.shaderc;

public interface ShadercIncludeResolveI {
    long invoke(long userData, long requestedSource, int type, long requestingSource, long includeDepth);
}
