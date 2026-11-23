package org.lwjgl.sdl;

import org.lwjgl.system.Struct;

public class SDL_WindowEvent extends Struct<SDL_WindowEvent> {
    public SDL_WindowEvent() {
        super(0);
    }

    @Override
    public int sizeof() {
        return 64;
    }
    int data1;
    int data2;

    public int data1() {
        return data1;
    }

    public int data2() {
        return data2;
    }
}
