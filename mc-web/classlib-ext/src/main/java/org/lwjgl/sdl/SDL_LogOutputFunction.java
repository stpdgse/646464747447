package org.lwjgl.sdl;

/** SDL log callbacks never fire in the browser build; this only keeps the registration call valid. */
public class SDL_LogOutputFunction {
    private final SDL_LogOutputFunctionI delegate;

    private SDL_LogOutputFunction(SDL_LogOutputFunctionI delegate) {
        this.delegate = delegate;
    }

    public static SDL_LogOutputFunction create(SDL_LogOutputFunctionI delegate) {
        return new SDL_LogOutputFunction(delegate);
    }

    public long address() {
        return 1;
    }

    public void free() {
    }
}
