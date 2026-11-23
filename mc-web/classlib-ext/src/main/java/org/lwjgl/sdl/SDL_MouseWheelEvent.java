package org.lwjgl.sdl;

import org.lwjgl.system.Struct;

public class SDL_MouseWheelEvent extends Struct<SDL_MouseWheelEvent> {
    public SDL_MouseWheelEvent() {
        super(0);
    }

    @Override
    public int sizeof() {
        return 64;
    }
    float x;
    float y;

    public float x() {
        return x;
    }

    public float y() {
        return y;
    }
}
