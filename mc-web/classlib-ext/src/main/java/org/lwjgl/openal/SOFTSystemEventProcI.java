package org.lwjgl.openal;

public interface SOFTSystemEventProcI {
    void invoke(int eventType, int deviceType, long device, int length, long message, long userParam);
}
