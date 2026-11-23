package org.lwjgl.sdl;

import org.lwjgl.system.Struct;

public class SDL_TextEditingEvent extends Struct<SDL_TextEditingEvent> {
    public SDL_TextEditingEvent() {
        super(0);
    }

    @Override
    public int sizeof() {
        return 64;
    }
    String text = "";
    int start;
    int length;

    public String textString() {
        return text;
    }

    public int start() {
        return start;
    }

    public int length() {
        return length;
    }
}
