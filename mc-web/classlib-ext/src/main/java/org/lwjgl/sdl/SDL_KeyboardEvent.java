package org.lwjgl.sdl;

import org.lwjgl.system.Struct;

public class SDL_KeyboardEvent extends Struct<SDL_KeyboardEvent> {
    public SDL_KeyboardEvent() {
        super(0);
    }

    @Override
    public int sizeof() {
        return 64;
    }
    int scancode;
    int key;
    short mod;
    boolean repeat;

    public int scancode() {
        return scancode;
    }

    public int key() {
        return key;
    }

    public short mod() {
        return mod;
    }

    public boolean repeat() {
        return repeat;
    }
}
