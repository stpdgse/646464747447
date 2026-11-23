package org.lwjgl.sdl;

import org.lwjgl.system.Struct;

public class SDL_MouseMotionEvent extends Struct<SDL_MouseMotionEvent> {
    public SDL_MouseMotionEvent() {
        super(0);
    }

    @Override
    public int sizeof() {
        return 64;
    }
    float x;
    float y;
    float xrel;
    float yrel;

    public float x() {
        return x;
    }

    public float y() {
        return y;
    }

    public float xrel() {
        return xrel;
    }

    public float yrel() {
        return yrel;
    }
}
