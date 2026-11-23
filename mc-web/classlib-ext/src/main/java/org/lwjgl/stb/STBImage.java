package org.lwjgl.stb;

public final class STBImage {
    private static String failure;

    private STBImage() {
    }

    /** Null when the last decode succeeded, as in the native library. */
    public static String stbi_failure_reason() {
        return failure;
    }

    static void fail(String reason) {
        failure = reason;
    }
}
