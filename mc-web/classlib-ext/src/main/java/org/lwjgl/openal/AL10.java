package org.lwjgl.openal;

import java.nio.ByteBuffer;
import org.teavm.mcweb.host.WebAudio;

/** OpenAL 1.0 core on Web Audio. */
public class AL10 {
    public static final int AL_NONE = 0;
    public static final int AL_FALSE = 0;
    public static final int AL_TRUE = 1;
    public static final int AL_SOURCE_RELATIVE = 0x202;
    public static final int AL_CONE_INNER_ANGLE = 0x1001;
    public static final int AL_CONE_OUTER_ANGLE = 0x1002;
    public static final int AL_PITCH = 0x1003;
    public static final int AL_POSITION = 0x1004;
    public static final int AL_DIRECTION = 0x1005;
    public static final int AL_VELOCITY = 0x1006;
    public static final int AL_LOOPING = 0x1007;
    public static final int AL_BUFFER = 0x1009;
    public static final int AL_GAIN = 0x100A;
    public static final int AL_ORIENTATION = 0x100F;
    public static final int AL_SOURCE_STATE = 0x1010;
    public static final int AL_INITIAL = 0x1011;
    public static final int AL_PLAYING = 0x1012;
    public static final int AL_PAUSED = 0x1013;
    public static final int AL_STOPPED = 0x1014;
    public static final int AL_BUFFERS_QUEUED = 0x1015;
    public static final int AL_BUFFERS_PROCESSED = 0x1016;
    public static final int AL_FORMAT_MONO8 = 0x1100;
    public static final int AL_FORMAT_MONO16 = 0x1101;
    public static final int AL_FORMAT_STEREO8 = 0x1102;
    public static final int AL_FORMAT_STEREO16 = 0x1103;
    public static final int AL_NO_ERROR = 0;
    public static final int AL_INVALID_NAME = 0xA001;
    public static final int AL_INVALID_ENUM = 0xA002;
    public static final int AL_INVALID_VALUE = 0xA003;
    public static final int AL_INVALID_OPERATION = 0xA004;
    public static final int AL_OUT_OF_MEMORY = 0xA005;

    public static void alEnable(int capability) {
    }

    public static void alDisable(int capability) {
    }

    public static int alGetError() {
        return AL_NO_ERROR;
    }

    public static void alGenBuffers(int[] buffers) {
        for (int i = 0; i < buffers.length; i++) {
            buffers[i] = WebAudio.genBuffer();
        }
    }

    public static void alDeleteBuffers(int[] buffers) {
        for (int b : buffers) {
            WebAudio.deleteBuffer(b);
        }
    }

    public static void alBufferData(int buffer, int format, ByteBuffer data, int frequency) {
        WebAudio.bufferData(buffer, format, data.array(), data.arrayOffset() + data.position(), data.remaining(),
                frequency);
    }

    public static void alGenSources(int[] sources) {
        for (int i = 0; i < sources.length; i++) {
            sources[i] = WebAudio.genSource();
        }
    }

    public static void alDeleteSources(int[] sources) {
        for (int s : sources) {
            WebAudio.deleteSource(s);
        }
    }

    public static void alSourcei(int source, int param, int value) {
        WebAudio.sourcei(source, param, value);
    }

    public static void alSourcef(int source, int param, float value) {
        WebAudio.sourcef(source, param, value);
    }

    public static void alSourcefv(int source, int param, float[] values) {
        WebAudio.sourcefv(source, param, values.length > 0 ? values[0] : 0, values.length > 1 ? values[1] : 0,
                values.length > 2 ? values[2] : 0);
    }

    public static void alSource3f(int source, int param, float x, float y, float z) {
        WebAudio.sourcefv(source, param, x, y, z);
    }

    public static int alGetSourcei(int source, int param) {
        return WebAudio.getSourcei(source, param);
    }

    public static void alSourcePlay(int source) {
        WebAudio.play(source);
    }

    public static void alSourcePause(int source) {
        WebAudio.pause(source);
    }

    public static void alSourceStop(int source) {
        WebAudio.stop(source);
    }

    public static void alSourceQueueBuffers(int source, int[] buffers) {
        for (int b : buffers) {
            WebAudio.queue(source, b);
        }
    }

    public static void alSourceUnqueueBuffers(int source, int[] buffers) {
        int n = WebAudio.unqueue(source, buffers.length);
        for (int i = 0; i < n; i++) {
            buffers[i] = WebAudio.unqueuedId(i);
        }
    }

    public static void alListener3f(int param, float x, float y, float z) {
        if (param == AL_POSITION) {
            WebAudio.listenerPosition(x, y, z);
        }
    }

    public static void alListenerfv(int param, float[] values) {
        if (param == AL_ORIENTATION && values.length >= 6) {
            WebAudio.listenerOrientation(values[0], values[1], values[2], values[3], values[4], values[5]);
        } else if (param == AL_POSITION && values.length >= 3) {
            WebAudio.listenerPosition(values[0], values[1], values[2]);
        }
    }

    public static void alListenerf(int param, float value) {
    }
}
