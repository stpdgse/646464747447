package org.lwjgl;

public final class Version {
    public static final int VERSION_MAJOR = 3;
    public static final int VERSION_MINOR = 4;
    public static final int VERSION_REVISION = 3;

    private Version() {
    }

    public static String getVersion() {
        return "3.4.3 (browser port)";
    }
}
