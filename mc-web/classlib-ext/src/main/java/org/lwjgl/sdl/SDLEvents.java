package org.lwjgl.sdl;

import org.teavm.mcweb.host.Sdl;

/** Event polling: the page queues DOM events in SDL form and this copies the next one into the struct. */
public class SDLEvents {
    public static final long WINDOW_HANDLE = 1L;
    private static int consumed;

    public static boolean SDL_PollEvent(SDL_Event event) {
        if (event == null) {
            return Sdl.poll();
        }
        if (!Sdl.poll()) {
            return false;
        }
        int type = Sdl.evType();
        event.type = type;
        if (consumed++ < 30) {
            org.teavm.mcweb.host.WebHost.consoleLog("[sdl] event 0x" + Integer.toHexString(type));
        }
        switch (type) {
            case 0x300:
            case 0x301:
                event.key.scancode = Sdl.evInt(0);
                event.key.key = Sdl.evInt(1);
                event.key.mod = (short) Sdl.evInt(2);
                event.key.repeat = Sdl.evInt(3) != 0;
                break;
            case 0x302:
                event.edit.text = Sdl.evString();
                event.edit.start = Sdl.evInt(0);
                event.edit.length = Sdl.evInt(1);
                break;
            case 0x303:
                event.text.text = Sdl.evString();
                break;
            case 0x400:
                event.motion.x = (float) Sdl.evFloat(0);
                event.motion.y = (float) Sdl.evFloat(1);
                event.motion.xrel = (float) Sdl.evFloat(2);
                event.motion.yrel = (float) Sdl.evFloat(3);
                break;
            case 0x401:
            case 0x402:
                event.button.button = (byte) Sdl.evInt(0);
                event.button.x = (float) Sdl.evFloat(0);
                event.button.y = (float) Sdl.evFloat(1);
                break;
            case 0x403:
                event.wheel.x = (float) Sdl.evFloat(0);
                event.wheel.y = (float) Sdl.evFloat(1);
                break;
            case 0x1000:
            case 0x1001:
            case 0x1002:
            case 0x1003:
                event.drop.data = Sdl.evString();
                break;
            default:
                event.window.data1 = Sdl.evInt(0);
                event.window.data2 = Sdl.evInt(1);
                break;
        }
        return true;
    }

    public static long SDL_GetWindowFromEvent(SDL_Event event) {
        return WINDOW_HANDLE;
    }

    public static void SDL_PumpEvents() {
    }

    public static void SDL_FlushEvents(int minType, int maxType) {
        Sdl.flushEvents();
    }
}
