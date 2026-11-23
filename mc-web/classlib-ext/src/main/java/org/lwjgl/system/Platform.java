package org.lwjgl.system;

/** The browser build reports itself as 64-bit Linux so no platform-specific code paths trigger. */
public enum Platform {
    FREEBSD("FreeBSD"), LINUX("Linux"), MACOSX("macOS"), WINDOWS("Windows");

    private final String displayName;

    public enum Architecture {
        X64, X86, ARM64, ARM32, PPC64LE, RISCV64;

        @Override
        public String toString() {
            return name();
        }
    }

    Platform(String displayName) {
        this.displayName = displayName;
    }

    public String getName() {
        return displayName;
    }

    public static int getJavaVersion() {
        return 21;
    }

    public static Platform get() {
        return LINUX;
    }

    public static Architecture getArchitecture() {
        return Architecture.X64;
    }

    public static String mapLibraryNameBundled(String name) {
        return name;
    }

    @Override
    public String toString() {
        return displayName;
    }
}
