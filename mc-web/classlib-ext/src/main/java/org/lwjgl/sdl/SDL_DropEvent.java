package org.lwjgl.sdl;

import org.lwjgl.system.Struct;

public class SDL_DropEvent extends Struct<SDL_DropEvent> {
    public SDL_DropEvent() {
        super(0);
    }

    @Override
    public int sizeof() {
        return 64;
    }
    String data = "";

    public String dataString() {
        return data;
    }
}
