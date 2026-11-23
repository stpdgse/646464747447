package org.lwjgl.sdl;

import org.lwjgl.system.NativeResource;
import org.lwjgl.system.Struct;

/** An SDL event filled in from the JavaScript event queue by {@link SDLEvents#SDL_PollEvent}. */
public class SDL_Event extends Struct<SDL_Event> implements NativeResource {
    int type;
    final SDL_KeyboardEvent key = new SDL_KeyboardEvent();
    final SDL_MouseMotionEvent motion = new SDL_MouseMotionEvent();
    final SDL_MouseButtonEvent button = new SDL_MouseButtonEvent();
    final SDL_MouseWheelEvent wheel = new SDL_MouseWheelEvent();
    final SDL_TextInputEvent text = new SDL_TextInputEvent();
    final SDL_TextEditingEvent edit = new SDL_TextEditingEvent();
    final SDL_WindowEvent window = new SDL_WindowEvent();
    final SDL_DisplayEvent display = new SDL_DisplayEvent();
    final SDL_DropEvent drop = new SDL_DropEvent();

    public SDL_Event() {
        super(0);
    }

    public static SDL_Event malloc() {
        return new SDL_Event();
    }

    public static SDL_Event calloc() {
        return new SDL_Event();
    }

    @Override
    public int sizeof() {
        return 128;
    }

    @Override
    public void free() {
    }

    @Override
    public void close() {
    }

    public int type() {
        return type;
    }

    public SDL_KeyboardEvent key() {
        return key;
    }

    public SDL_MouseMotionEvent motion() {
        return motion;
    }

    public SDL_MouseButtonEvent button() {
        return button;
    }

    public SDL_MouseWheelEvent wheel() {
        return wheel;
    }

    public SDL_TextInputEvent text() {
        return text;
    }

    public SDL_TextEditingEvent edit() {
        return edit;
    }

    public SDL_WindowEvent window() {
        return window;
    }

    public SDL_DisplayEvent display() {
        return display;
    }

    public SDL_DropEvent drop() {
        return drop;
    }
}
