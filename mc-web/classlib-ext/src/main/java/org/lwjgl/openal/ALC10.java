package org.lwjgl.openal;

import java.nio.IntBuffer;
import org.teavm.mcweb.host.WebAudio;

/** The ALC device/context layer: one virtual device backed by the page's AudioContext. */
public class ALC10 {
    public static final int ALC_FALSE = 0;
    public static final int ALC_TRUE = 1;
    public static final int ALC_FREQUENCY = 0x1007;
    public static final int ALC_REFRESH = 0x1008;
    public static final int ALC_SYNC = 0x1009;
    public static final int ALC_MONO_SOURCES = 0x1010;
    public static final int ALC_STEREO_SOURCES = 0x1011;
    public static final int ALC_NO_ERROR = 0;
    public static final int ALC_MAJOR_VERSION = 0x1000;
    public static final int ALC_MINOR_VERSION = 0x1001;
    public static final int ALC_ATTRIBUTES_SIZE = 0x1002;
    public static final int ALC_ALL_ATTRIBUTES = 0x1003;
    public static final int ALC_DEFAULT_DEVICE_SPECIFIER = 0x1004;
    public static final int ALC_DEVICE_SPECIFIER = 0x1005;
    public static final int ALC_EXTENSIONS = 0x1006;

    static final String DEVICE_NAME = "Web Audio";
    private static long currentContext;

    public static long alcOpenDevice(CharSequence name) {
        return WebAudio.open() ? 1L : 0L;
    }

    public static boolean alcCloseDevice(long device) {
        return true;
    }

    public static long alcCreateContext(long device, IntBuffer attributes) {
        return device == 0 ? 0L : 2L;
    }

    public static void alcDestroyContext(long context) {
    }

    public static boolean alcMakeContextCurrent(long context) {
        currentContext = context;
        return true;
    }

    public static int alcGetError(long device) {
        return ALC_NO_ERROR;
    }

    public static String alcGetString(long device, int param) {
        switch (param) {
            case ALC_DEFAULT_DEVICE_SPECIFIER:
            case ALC_DEVICE_SPECIFIER:
            case 0x1012: // ALC_DEFAULT_ALL_DEVICES_SPECIFIER
            case 0x1013: // ALC_ALL_DEVICES_SPECIFIER
                return DEVICE_NAME;
            case ALC_EXTENSIONS:
                return "";
            default:
                return null;
        }
    }

    public static int alcGetInteger(long device, int param) {
        switch (param) {
            case ALC_MAJOR_VERSION:
                return 1;
            case ALC_MINOR_VERSION:
                return 1;
            case ALC_FREQUENCY:
                return WebAudio.sampleRate();
            case ALC_MONO_SOURCES:
                return 255;
            case ALC_STEREO_SOURCES:
                return 1;
            default:
                return 0;
        }
    }

    public static void alcGetIntegerv(long device, int param, IntBuffer values) {
        if (values != null && values.remaining() > 0) {
            values.put(values.position(), alcGetInteger(device, param));
        }
    }

    public static boolean alcIsExtensionPresent(long device, CharSequence name) {
        return false;
    }
}
