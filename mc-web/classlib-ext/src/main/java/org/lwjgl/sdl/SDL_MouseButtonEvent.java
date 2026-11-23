package org.lwjgl.sdl;

import org.lwjgl.system.Struct;

public class SDL_MouseButtonEvent extends Struct<SDL_MouseButtonEvent> {
    public SDL_MouseButtonEvent() {
        super(0);
    }

    @Override
    public int sizeof() {
        return 64;
    }
    byte button;
    float x;
    float y;

    public byte button() {
        return button;
    }

    public float x() {
        return x;
    }

    public float y() {
        return y;
    }
}
