package org.lwjgl.openal;

/** Device hot-plug events do not exist in the browser. */
public final class SOFTSystemEvents {
    private SOFTSystemEvents() {
    }

    public static void alcEventCallbackSOFT(SOFTSystemEventProcI callback, long userParam) {
    }

    public static boolean alcEventControlSOFT(int[] events, boolean enable) {
        return false;
    }

    public static int alcEventIsSupportedSOFT(int eventType, int deviceType) {
        return 0;
    }
}
