package org.lwjgl.sdl;

import org.lwjgl.system.MemoryStack;
import org.lwjgl.system.Struct;


public class SDL_Rect extends Struct<SDL_Rect> {
    int x;
    int y;
    int w;
    int h;

    public SDL_Rect() {
        super(0);
    }

    public static SDL_Rect malloc(MemoryStack stack) {
        return new SDL_Rect();
    }

    public static SDL_Rect calloc(MemoryStack stack) {
        return new SDL_Rect();
    }

    public static Buffer malloc(int count, MemoryStack stack) {
        return new Buffer(count);
    }

    public static Buffer calloc(int count, MemoryStack stack) {
        return new Buffer(count);
    }

    @Override
    public int sizeof() {
        return 16;
    }

    public int x() {
        return x;
    }

    public int y() {
        return y;
    }

    public int w() {
        return w;
    }

    public int h() {
        return h;
    }

    public SDL_Rect x(int v) {
        x = v;
        return this;
    }

    public SDL_Rect y(int v) {
        y = v;
        return this;
    }

    public SDL_Rect w(int v) {
        w = v;
        return this;
    }

    public SDL_Rect h(int v) {
        h = v;
        return this;
    }

    public static class Buffer extends SdlStructBuffer<SDL_Rect, Buffer> {
        Buffer(int count) {
            fill(count);
        }

        @Override
        protected SDL_Rect newElement() {
            return new SDL_Rect();
        }

        @Override
        protected Buffer self() {
            return this;
        }

        public Buffer x(int v) {
            current().x = v;
            return this;
        }

        public Buffer y(int v) {
            current().y = v;
            return this;
        }

        public Buffer w(int v) {
            current().w = v;
            return this;
        }

        public Buffer h(int v) {
            current().h = v;
            return this;
        }
    }
}
