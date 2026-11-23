package org.teavm.mcweb.host;

import org.teavm.jso.JSBody;
import org.teavm.jso.typedarrays.Int8Array;

/** Things the hosting page provides through JavaScript globals. */
public final class WebHost {
    private WebHost() {
    }

    @JSBody(params = "message", script = "console.log(message);")
    public static native void consoleLog(String message);

    @JSBody(params = "message", script = "console.warn(message);")
    public static native void consoleWarn(String message);

    @JSBody(params = "message", script = "console.error(message);")
    public static native void consoleError(String message);

    @JSBody(params = "message", script = "console.warn(message + '\\n' + new Error().stack);")
    public static native void logStack(String message);

    @JSBody(script = "return globalThis.mcwebArchive || null;")
    private static native Int8Array archiveArray();

    /** The resource archive the page loaded before starting the game, or null. */
    public static byte[] archive() {
        Int8Array a = archiveArray();
        return a == null ? null : a.copyToJavaArray();
    }
}
