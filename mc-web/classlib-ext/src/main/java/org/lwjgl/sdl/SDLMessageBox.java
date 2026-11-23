package org.lwjgl.sdl;

import java.nio.IntBuffer;
import org.teavm.mcweb.host.Sdl;

public class SDLMessageBox {
    public static boolean SDL_ShowMessageBox(SDL_MessageBoxData data, IntBuffer buttonId) {
        Sdl.alert(data.title + "\n\n" + data.message);
        if (buttonId != null && data.buttons != null && data.buttons.capacity() > 0) {
            buttonId.put(buttonId.position(), data.buttons.get(0).buttonID);
        }
        return true;
    }

    public static boolean SDL_ShowSimpleMessageBox(int flags, CharSequence title, CharSequence message, long window) {
        Sdl.alert(title + "\n\n" + message);
        return true;
    }
}
