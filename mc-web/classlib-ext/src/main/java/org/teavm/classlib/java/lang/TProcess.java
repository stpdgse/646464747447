package org.teavm.classlib.java.lang;

import java.io.InputStream;
import java.io.OutputStream;

/** Child processes do not exist in a browser; this type only lets referencing code link. */
public abstract class TProcess extends TObject {
    public abstract OutputStream getOutputStream();

    public abstract InputStream getInputStream();

    public abstract InputStream getErrorStream();

    public abstract int waitFor() throws InterruptedException;

    public boolean waitFor(long timeout, java.util.concurrent.TimeUnit unit) throws InterruptedException {
        waitFor();
        return true;
    }

    public abstract int exitValue();

    public abstract void destroy();

    public TProcess destroyForcibly() {
        destroy();
        return this;
    }
}
