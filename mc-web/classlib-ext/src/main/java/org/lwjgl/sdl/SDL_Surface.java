package org.lwjgl.sdl;

import java.nio.ByteBuffer;
import org.lwjgl.system.Struct;

public class SDL_Surface extends Struct<SDL_Surface> {
    final int w;
    final int h;
    final int format;
    final ByteBuffer pixels;
    final int pitch;

    SDL_Surface(int w, int h, int format, ByteBuffer pixels, int pitch) {
        super(0);
        this.w = w;
        this.h = h;
        this.format = format;
        this.pixels = pixels;
        this.pitch = pitch;
    }

    @Override
    public int sizeof() {
        return 64;
    }

    public int w() {
        return w;
    }

    public int h() {
        return h;
    }

    public int format() {
        return format;
    }

    public ByteBuffer pixels() {
        return pixels;
    }

    public int pitch() {
        return pitch;
    }
}
