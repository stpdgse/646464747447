package org.lwjgl.sdl;

import org.lwjgl.system.Struct;

public class SDL_PixelFormatDetails extends Struct<SDL_PixelFormatDetails> {
    public SDL_PixelFormatDetails() {
        super(0);
    }

    @Override
    public int sizeof() {
        return 32;
    }

    public byte Rbits() {
        return 8;
    }

    public byte Gbits() {
        return 8;
    }

    public byte Bbits() {
        return 8;
    }

    public byte Abits() {
        return 0;
    }

    public byte bits_per_pixel() {
        return 32;
    }
}
