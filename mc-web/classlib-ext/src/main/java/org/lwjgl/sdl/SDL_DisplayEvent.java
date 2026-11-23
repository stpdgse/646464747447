package org.lwjgl.sdl;

import org.lwjgl.system.Struct;

public class SDL_DisplayEvent extends Struct<SDL_DisplayEvent> {
    public SDL_DisplayEvent() {
        super(0);
    }

    @Override
    public int sizeof() {
        return 64;
    }
    int displayID = 1;

    public int displayID() {
        return displayID;
    }
}
