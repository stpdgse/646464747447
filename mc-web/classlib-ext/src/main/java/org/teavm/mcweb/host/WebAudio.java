package org.teavm.mcweb.host;

import org.teavm.jso.JSBody;
import org.teavm.jso.typedarrays.Int8Array;

/** Bridge to tools/mcaudio.js, the Web Audio implementation behind the OpenAL shim. */
public final class WebAudio {
    private WebAudio() {
    }

    @JSBody(script = "return !!globalThis.mcaudio && mcaudio.open();")
    public static native boolean open();

    @JSBody(script = "return mcaudio.sampleRate();")
    public static native int sampleRate();

    @JSBody(script = "return mcaudio.genBuffer();")
    public static native int genBuffer();

    @JSBody(params = "id", script = "mcaudio.deleteBuffer(id);")
    public static native void deleteBuffer(int id);

    @JSBody(params = {"id", "format", "data", "freq"}, script = "mcaudio.bufferData(id, format, data, freq);")
    private static native void bufferData(int id, int format, Int8Array data, int freq);

    public static void bufferData(int id, int format, byte[] data, int offset, int length, int freq) {
        // plain System.arraycopy: Arrays.copyOfRange would be compiled as a coroutine here
        byte[] part = new byte[length];
        System.arraycopy(data, offset, part, 0, length);
        bufferData(id, format, Int8Array.copyFromJavaArray(part), freq);
    }

    @JSBody(script = "return mcaudio.genSource();")
    public static native int genSource();

    @JSBody(params = "id", script = "mcaudio.deleteSource(id);")
    public static native void deleteSource(int id);

    @JSBody(params = {"id", "param", "v"}, script = "mcaudio.sourcei(id, param, v);")
    public static native void sourcei(int id, int param, int v);

    @JSBody(params = {"id", "param", "v"}, script = "mcaudio.sourcef(id, param, v);")
    public static native void sourcef(int id, int param, float v);

    @JSBody(params = {"id", "param", "x", "y", "z"}, script = "mcaudio.sourcefv(id, param, x, y, z);")
    public static native void sourcefv(int id, int param, float x, float y, float z);

    @JSBody(params = {"id", "param"}, script = "return mcaudio.getSourcei(id, param);")
    public static native int getSourcei(int id, int param);

    @JSBody(params = "id", script = "mcaudio.play(id);")
    public static native void play(int id);

    @JSBody(params = "id", script = "mcaudio.pause(id);")
    public static native void pause(int id);

    @JSBody(params = "id", script = "mcaudio.stop(id);")
    public static native void stop(int id);

    @JSBody(params = {"id", "buffer"}, script = "mcaudio.queue(id, buffer);")
    public static native void queue(int id, int buffer);

    @JSBody(params = {"id", "count"}, script = "return mcaudio.unqueue(id, count);")
    public static native int unqueue(int id, int count);

    @JSBody(params = "i", script = "return mcaudio.unqueuedId(i);")
    public static native int unqueuedId(int i);

    @JSBody(params = {"x", "y", "z"}, script = "mcaudio.listenerPos(x, y, z);")
    public static native void listenerPosition(float x, float y, float z);

    @JSBody(params = {"fx", "fy", "fz", "ux", "uy", "uz"}, script = "mcaudio.listenerOrientation(fx, fy, fz, ux, uy, uz);")
    public static native void listenerOrientation(float fx, float fy, float fz, float ux, float uy, float uz);
}
