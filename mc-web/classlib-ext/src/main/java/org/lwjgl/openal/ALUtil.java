package org.lwjgl.openal;

import java.util.ArrayList;
import java.util.List;

public final class ALUtil {
    private ALUtil() {
    }

    public static List<String> getStringList(long device, int param) {
        List<String> names = new ArrayList<>();
        String name = ALC10.alcGetString(device, param);
        if (name != null) {
            names.add(name);
        }
        return names;
    }
}
