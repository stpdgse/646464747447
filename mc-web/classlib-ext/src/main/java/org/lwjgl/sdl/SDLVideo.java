package org.lwjgl.sdl;

import java.nio.IntBuffer;
import java.util.HashMap;
import java.util.Map;
import org.lwjgl.PointerBuffer;
import org.teavm.mcweb.host.Sdl;

/** One window (the canvas), one display (the screen), one GL context (WebGL2). */
public class SDLVideo {
    private static final long WINDOW = SDLEvents.WINDOW_HANDLE;
    private static final long CONTEXT = 1L;
    private static final int DISPLAY = 1;

    private static final Map<Integer, Integer> GL_ATTRIBUTES = new HashMap<>();
    private static long windowFlags;
    private static boolean created;
    private static SDL_DisplayMode fullscreenMode;

    private static void sizeOf(IntBuffer w, IntBuffer h, double scale) {
        if (w != null) {
            w.put(w.position(), (int) Math.round(Sdl.width() * scale));
        }
        if (h != null) {
            h.put(h.position(), (int) Math.round(Sdl.height() * scale));
        }
    }

    // window ------------------------------------------------------------------------------------

    public static long SDL_CreateWindow(CharSequence title, int w, int h, long flags) {
        Sdl.createWindow(title.toString(), w, h);
        windowFlags = flags;
        created = true;
        return WINDOW;
    }

    public static void SDL_DestroyWindow(long window) {
        created = false;
    }

    public static boolean SDL_SetWindowTitle(long window, CharSequence title) {
        Sdl.setTitle(title.toString());
        return true;
    }

    public static boolean SDL_SetWindowSize(long window, int w, int h) {
        Sdl.setSize(w, h);
        return true;
    }

    public static boolean SDL_SetWindowPosition(long window, int x, int y) {
        return true;
    }

    public static boolean SDL_GetWindowPosition(long window, IntBuffer x, IntBuffer y) {
        if (x != null) {
            x.put(x.position(), 0);
        }
        if (y != null) {
            y.put(y.position(), 0);
        }
        return true;
    }

    public static boolean SDL_GetWindowSizeInPixels(long window, IntBuffer w, IntBuffer h) {
        sizeOf(w, h, Sdl.pixelDensity());
        return true;
    }

    public static float SDL_GetWindowPixelDensity(long window) {
        return (float) Sdl.pixelDensity();
    }

    public static boolean SDL_SetWindowMinimumSize(long window, int w, int h) {
        return true;
    }

    public static boolean SDL_SetWindowMaximumSize(long window, int w, int h) {
        return true;
    }

    public static boolean SDL_SetWindowBordered(long window, boolean bordered) {
        return true;
    }

    public static boolean SDL_SetWindowIcon(long window, SDL_Surface icon) {
        return true;
    }

    public static boolean SDL_SetWindowMouseGrab(long window, boolean grabbed) {
        Sdl.setMouseGrab(grabbed);
        return true;
    }

    public static boolean SDL_RestoreWindow(long window) {
        if (Sdl.isFullscreen()) {
            Sdl.setFullscreen(false);
        }
        return true;
    }

    public static boolean SDL_SyncWindow(long window) {
        return true;
    }

    public static boolean SDL_SetWindowFullscreen(long window, boolean fullscreen) {
        Sdl.setFullscreen(fullscreen);
        return true;
    }

    public static long SDL_GetWindowFlags(long window) {
        long flags = windowFlags & ~0x1L;
        if (Sdl.isFullscreen()) {
            flags |= 0x1L;
        }
        return flags | 0x200L | 0x400L; // input + mouse focus
    }

    public static boolean SDL_SetWindowFullscreenMode(long window, SDL_DisplayMode mode) {
        fullscreenMode = mode;
        return true;
    }

    public static SDL_DisplayMode SDL_GetWindowFullscreenMode(long window) {
        return null; // borderless fullscreen: the browser decides the size
    }

    // displays ----------------------------------------------------------------------------------

    public static IntBuffer SDL_GetDisplays() {
        IntBuffer ids = IntBuffer.allocate(1);
        ids.put(0, DISPLAY);
        return ids;
    }

    public static int SDL_GetPrimaryDisplay() {
        return DISPLAY;
    }

    public static int SDL_GetDisplayForWindow(long window) {
        return DISPLAY;
    }

    public static String SDL_GetDisplayName(int display) {
        return "Browser display";
    }

    public static String SDL_GetCurrentVideoDriver() {
        return "browser";
    }

    private static SDL_DisplayMode desktopMode() {
        return new SDL_DisplayMode(Sdl.screenWidth(), Sdl.screenHeight(), 60f);
    }

    public static SDL_DisplayMode SDL_GetDesktopDisplayMode(int display) {
        return desktopMode();
    }

    public static SDL_DisplayMode SDL_GetCurrentDisplayMode(int display) {
        return desktopMode();
    }

    public static PointerBuffer SDL_GetFullscreenDisplayModes(int display) {
        PointerBuffer modes = PointerBuffer.allocateDirect(1);
        modes.put(0, SDL_DisplayMode.register(desktopMode()));
        return modes;
    }

    public static boolean SDL_GetClosestFullscreenDisplayMode(int display, int w, int h, float refreshRate,
            boolean includeHighDensity, SDL_DisplayMode closest) {
        closest.set(desktopMode());
        return true;
    }

    public static boolean SDL_GetDisplayBounds(int display, SDL_Rect rect) {
        rect.x = 0;
        rect.y = 0;
        rect.w = Sdl.screenWidth();
        rect.h = Sdl.screenHeight();
        return true;
    }

    // OpenGL ------------------------------------------------------------------------------------

    public static boolean SDL_GL_LoadLibrary(CharSequence path) {
        return true;
    }

    public static void SDL_GL_UnloadLibrary() {
    }

    public static boolean SDL_GL_SetAttribute(int attr, int value) {
        GL_ATTRIBUTES.put(attr, value);
        return true;
    }

    public static boolean SDL_GL_GetAttribute(int attr, IntBuffer value) {
        Integer v = GL_ATTRIBUTES.get(attr);
        value.put(value.position(), v == null ? 0 : v);
        return true;
    }

    public static long SDL_GL_CreateContext(long window) {
        org.teavm.mcweb.host.Gl.init();
        return CONTEXT;
    }

    public static boolean SDL_GL_DestroyContext(long context) {
        return true;
    }

    public static boolean SDL_GL_MakeCurrent(long window, long context) {
        return true;
    }

    public static long SDL_GL_GetProcAddress(CharSequence name) {
        return 1L;
    }

    public static boolean SDL_GL_SetSwapInterval(int interval) {
        return true;
    }

    /** The browser presents the canvas when control returns to its event loop, so wait for the next frame. */
    public static boolean SDL_GL_SwapWindow(long window) {
        Sdl.nextFrame();
        return true;
    }
}
