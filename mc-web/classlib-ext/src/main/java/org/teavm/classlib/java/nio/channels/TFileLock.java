package org.teavm.classlib.java.nio.channels;

import org.teavm.classlib.java.lang.TObject;

public abstract class TFileLock extends TObject implements AutoCloseable {
    public abstract boolean isValid();

    public abstract void release() throws java.io.IOException;

    @Override
    public final void close() throws java.io.IOException {
        release();
    }
}
