package org.lwjgl.sdl;

import org.lwjgl.system.Struct;

public class SDL_TextInputEvent extends Struct<SDL_TextInputEvent> {
    public SDL_TextInputEvent() {
        super(0);
    }

    @Override
    public int sizeof() {
        return 64;
    }
    String text = "";

    public String textString() {
        return text;
    }
}
