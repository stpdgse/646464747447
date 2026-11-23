package org.lwjgl.sdl;

import java.util.HashMap;
import java.util.Map;
import org.lwjgl.system.MemoryStack;
import org.lwjgl.system.Struct;

public class SDL_DisplayMode extends Struct<SDL_DisplayMode> {
    /** Display modes handed out as pointers (SDL_GetFullscreenDisplayModes) are looked up by address. */
    private static final Map<Long, SDL_DisplayMode> REGISTRY = new HashMap<>();
    private static long nextAddress = 0x7000_0000L;

    int format = 0x16161804; // SDL_PIXELFORMAT_XRGB8888
    int w;
    int h;
    float pixelDensity = 1f;
    float refreshRate = 60f;

    public SDL_DisplayMode() {
        super(0);
    }

    public SDL_DisplayMode(int w, int h, float refreshRate) {
        super(0);
        this.w = w;
        this.h = h;
        this.refreshRate = refreshRate;
    }

    static long register(SDL_DisplayMode mode) {
        long address = nextAddress;
        nextAddress += 0x40;
        REGISTRY.put(address, mode);
        return address;
    }

    public static SDL_DisplayMode create(long address) {
        SDL_DisplayMode mode = REGISTRY.get(address);
        if (mode == null) {
            throw new IllegalArgumentException("unknown SDL_DisplayMode address " + address);
        }
        return mode;
    }

    public static SDL_DisplayMode malloc(MemoryStack stack) {
        return new SDL_DisplayMode();
    }

    public static SDL_DisplayMode calloc(MemoryStack stack) {
        return new SDL_DisplayMode();
    }

    void set(SDL_DisplayMode other) {
        format = other.format;
        w = other.w;
        h = other.h;
        pixelDensity = other.pixelDensity;
        refreshRate = other.refreshRate;
    }

    @Override
    public int sizeof() {
        return 32;
    }

    public int format() {
        return format;
    }

    public int w() {
        return w;
    }

    public int h() {
        return h;
    }

    public float pixel_density() {
        return pixelDensity;
    }

    public float refresh_rate() {
        return refreshRate;
    }
}
