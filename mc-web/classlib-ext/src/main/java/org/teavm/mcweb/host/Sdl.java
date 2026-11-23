package org.teavm.mcweb.host;

import org.teavm.interop.Async;
import org.teavm.interop.AsyncCallback;
import org.teavm.jso.JSBody;
import org.teavm.jso.JSFunctor;
import org.teavm.jso.JSObject;

/**
 * Bridge to the page's {@code mcweb} JavaScript object (see tools/mcweb.js). The JavaScript side owns
 * the canvas and turns DOM events into a queue of SDL-style events that Java polls.
 */
public final class Sdl {
    private Sdl() {
    }

    @JSFunctor
    public interface FrameCallback extends JSObject {
        void run();
    }

    // window -----------------------------------------------------------------------------------

    @JSBody(params = {"title", "width", "height"}, script = "mcweb.createWindow(title, width, height);")
    public static native void createWindow(String title, int width, int height);

    @JSBody(params = "title", script = "mcweb.setTitle(title);")
    public static native void setTitle(String title);

    @JSBody(script = "return mcweb.width();")
    public static native int width();

    @JSBody(script = "return mcweb.height();")
    public static native int height();

    @JSBody(script = "return mcweb.pixelDensity();")
    public static native double pixelDensity();

    @JSBody(script = "return mcweb.screenWidth();")
    public static native int screenWidth();

    @JSBody(script = "return mcweb.screenHeight();")
    public static native int screenHeight();

    @JSBody(params = {"w", "h"}, script = "mcweb.setSize(w, h);")
    public static native void setSize(int w, int h);

    @JSBody(params = "on", script = "mcweb.setFullscreen(on);")
    public static native void setFullscreen(boolean on);

    @JSBody(script = "return mcweb.isFullscreen();")
    public static native boolean isFullscreen();

    @JSBody(params = "on", script = "mcweb.setRelativeMouse(on);")
    public static native void setRelativeMouse(boolean on);

    @JSBody(params = "on", script = "mcweb.setMouseGrab(on);")
    public static native void setMouseGrab(boolean on);

    @JSBody(params = "cursor", script = "mcweb.setCursor(cursor);")
    public static native void setCursor(String cursor);

    @JSBody(params = "on", script = "mcweb.setTextInput(on);")
    public static native void setTextInput(boolean on);

    @JSBody(params = "url", script = "window.open(url, '_blank');")
    public static native void openUrl(String url);

    @JSBody(params = "message", script = "window.alert(message);")
    public static native void alert(String message);

    // events -----------------------------------------------------------------------------------

    /** Dequeues the next event into the JavaScript-side scratch record; false when none is pending. */
    @JSBody(script = "return mcweb.poll();")
    public static native boolean poll();

    @JSBody(script = "return mcweb.evType();")
    public static native int evType();

    @JSBody(params = "k", script = "return mcweb.evInt(k);")
    public static native int evInt(int k);

    @JSBody(params = "k", script = "return mcweb.evFloat(k);")
    public static native double evFloat(int k);

    @JSBody(script = "return mcweb.evString();")
    public static native String evString();

    @JSBody(script = "mcweb.flushEvents();")
    public static native void flushEvents();

    @JSBody(params = "scancode", script = "return mcweb.keyDown(scancode) ? 1 : 0;")
    public static native int keyDown(int scancode);

    @JSBody(script = "return mcweb.mods();")
    public static native int mods();

    @JSBody(script = "return mcweb.mouseX();")
    public static native double mouseX();

    @JSBody(script = "return mcweb.mouseY();")
    public static native double mouseY();

    @JSBody(script = "return mcweb.mouseButtons();")
    public static native int mouseButtons();

    // timing and frames --------------------------------------------------------------------------

    @JSBody(script = "return performance.now() * 1000000;")
    public static native double nanos();

    /**
     * Suspends the calling fiber until the browser is ready for the next animation frame, which is
     * also when the canvas contents are presented.
     */
    @Async
    public static native void nextFrame();

    private static void nextFrame(AsyncCallback<Void> callback) {
        requestFrame(() -> callback.complete(null));
    }

    @JSBody(params = "callback", script = "mcweb.requestFrame(callback);")
    private static native void requestFrame(FrameCallback callback);

    // clipboard ---------------------------------------------------------------------------------

    @JSBody(params = "text", script = "mcweb.setClipboard(text);")
    public static native void setClipboard(String text);

    @JSBody(script = "return mcweb.getClipboard();")
    public static native String getClipboard();
}
