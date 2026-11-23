package org.lwjgl.sdl;

import java.nio.ByteBuffer;
import org.lwjgl.system.MemoryStack;
import org.lwjgl.system.MemoryUtil;
import org.lwjgl.system.Struct;


public class SDL_MessageBoxButtonData extends Struct<SDL_MessageBoxButtonData> {
    int flags;
    int buttonID;
    String text = "";

    public SDL_MessageBoxButtonData() {
        super(0);
    }

    public static Buffer calloc(int count, MemoryStack stack) {
        return new Buffer(count);
    }

    @Override
    public int sizeof() {
        return 24;
    }

    public SDL_MessageBoxButtonData flags(int v) {
        flags = v;
        return this;
    }

    public SDL_MessageBoxButtonData buttonID(int v) {
        buttonID = v;
        return this;
    }

    public SDL_MessageBoxButtonData text(ByteBuffer v) {
        text = MemoryUtil.memUTF8(v);
        return this;
    }

    public static class Buffer extends SdlStructBuffer<SDL_MessageBoxButtonData, Buffer> {
        Buffer(int count) {
            fill(count);
        }

        @Override
        protected SDL_MessageBoxButtonData newElement() {
            return new SDL_MessageBoxButtonData();
        }

        @Override
        protected Buffer self() {
            return this;
        }
    }
}
