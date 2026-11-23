package org.lwjgl.openal;

public class ALC11 extends ALC10 {
    public static int alcGetInteger(long device, int param) {
        return ALC10.alcGetInteger(device, param);
    }
}
