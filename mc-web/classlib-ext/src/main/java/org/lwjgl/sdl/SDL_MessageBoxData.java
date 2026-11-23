package org.lwjgl.sdl;

import java.nio.ByteBuffer;
import org.lwjgl.system.MemoryStack;
import org.lwjgl.system.MemoryUtil;
import org.lwjgl.system.Struct;

public class SDL_MessageBoxData extends Struct<SDL_MessageBoxData> {
    int flags;
    String title = "";
    String message = "";
    SDL_MessageBoxButtonData.Buffer buttons;

    public SDL_MessageBoxData() {
        super(0);
    }

    public static SDL_MessageBoxData calloc(MemoryStack stack) {
        return new SDL_MessageBoxData();
    }

    @Override
    public int sizeof() {
        return 64;
    }

    public SDL_MessageBoxData flags(int v) {
        flags = v;
        return this;
    }

    public SDL_MessageBoxData title(ByteBuffer v) {
        title = MemoryUtil.memUTF8(v);
        return this;
    }

    public SDL_MessageBoxData message(ByteBuffer v) {
        message = MemoryUtil.memUTF8(v);
        return this;
    }

    public SDL_MessageBoxData buttons(SDL_MessageBoxButtonData.Buffer v) {
        buttons = v;
        return this;
    }
}
